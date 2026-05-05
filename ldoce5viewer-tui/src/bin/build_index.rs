//! Build Tantivy + incremental + filemap indices from an LDOCE5 `ldoce5.data` folder.
//!
//! Usage:
//!   cargo run --bin build_index -- /path/to/ldoce5.data [out_data_dir]

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use md5::compute as md5_compute;
use roxmltree::Document;

use ldoce5viewer_tui::config;
use ldoce5viewer_tui::data::{self, ArchiveReader, CDBMaker, ARCHIVE_DIRS};
use ldoce5viewer_tui::search::{FulltextMaker, IncrementalMaker};

#[derive(Debug, Clone)]
struct Item {
    itemtype: String,
    label: String,
    path: String,
    content: String,
    sortkey: String,
    asfilter: String,
    prio: u8,
}

fn shorten_id(id: &str) -> String {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}", parts[2], parts[3])
    } else {
        id.to_string()
    }
}

fn collapse_space(s: &str) -> String {
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn collect_text(node: roxmltree::Node, exclude: &HashSet<&str>) -> String {
    let mut out = String::new();

    fn rec(n: roxmltree::Node, out: &mut String, exclude: &HashSet<&str>) {
        match n.node_type() {
            roxmltree::NodeType::Text => {
                if let Some(t) = n.text() {
                    if !t.trim().is_empty() {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(t.trim());
                    }
                }
            }
            roxmltree::NodeType::Element => {
                let tag = n.tag_name().name().to_lowercase();
                if exclude.contains(tag.as_str()) {
                    return;
                }
                // initial text (text before children)
                if let Some(t) = n.text() {
                    if !t.trim().is_empty() {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(t.trim());
                    }
                }
                for c in n.children() {
                    rec(c, out, exclude);
                }
            }
            _ => {}
        }
    }

    let exclude = exclude;
    rec(node, &mut out, exclude);
    collapse_space(&out)
}

fn extract_items_from_entry(
    entry_data: &[u8],
) -> Result<(Vec<Item>, HashMap<String, Vec<String>>)> {
    let xml = std::str::from_utf8(entry_data).context("entry not utf8")?;
    let doc = Document::parse(xml).context("parse xml")?;
    let root = doc.root_element();
    let root_id = root.attribute("id").unwrap_or("");
    let root_id_short = shorten_id(root_id);

    // Find Head
    let head = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Head");

    let mut items: Vec<Item> = Vec::new();
    let mut variations: HashMap<String, Vec<String>> = HashMap::new();

    let exclude: HashSet<&str> = ["span", "object", "gloss"].iter().copied().collect();

    // HWD base text
    let hwd_base = head
        .and_then(|h| {
            h.descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "HWD")
        })
        .and_then(|hwd| {
            hwd.descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "BASE")
        })
        .map(|n| collect_text(n, &exclude))
        .unwrap_or_default();

    // is_freq
    let is_freq = head
        .and_then(|h| {
            h.descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "FREQ")
        })
        .is_some();

    // build hwdlabel similar to Python's make_hwd_label
    let mut baselabel = hwd_base.clone();
    // HOMNUM
    if let Some(h) = head {
        if let Some(homnum) = h
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "HOMNUM")
        {
            let hom = collect_text(homnum, &exclude);
            if !hom.is_empty() {
                baselabel.push_str(&format!("<s>{}</s>", hom));
            }
        }
    }
    let mut hwdlabel = if is_freq {
        format!("<f>{}</f>", baselabel)
    } else {
        format!("<n>{}</n>", baselabel)
    };
    // POS
    if let Some(h) = head {
        let poslist: Vec<String> = h
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "POS")
            .filter_map(|n| n.text().map(|s| s.to_string()))
            .collect();
        if !poslist.is_empty() {
            hwdlabel.push_str(&format!(" <p>{}</p>", poslist.join(", ")));
        }
    }

    let hwd_label = format!("<h>{}</h>", hwdlabel.clone());
    let path_root = format!("/fs/{}", root_id_short);

    /// Return the path to `elem` by looking at its own `id` first, then
    /// walking up to the nearest ancestor that has one.
    fn elem_path(elem: roxmltree::Node, root_id_short: &str, path_root: &str) -> String {
        let pid = {
            let mut n = elem;
            loop {
                if let Some(id) = n.attribute("id") {
                    break id.to_owned();
                }
                match n.parent() {
                    Some(p) => n = p,
                    None => break String::new(),
                }
            }
        };
        if pid.is_empty() {
            path_root.to_owned()
        } else {
            format!("/fs/{}#{}", root_id_short, shorten_id(&pid))
        }
    }

    // headword item (hm)
    let item = Item {
        itemtype: "hm".to_string(),
        label: hwd_label.clone(),
        path: path_root.clone(),
        content: collect_text(root, &exclude),
        sortkey: hwd_base.clone(),
        asfilter: String::new(),
        prio: 1,
    };
    items.push(item);

    // ── Headword inflection variants (hv) ──────────────────────────────────
    // HWD/INFLX — different inflected forms of the headword
    if let Some(h) = head {
        if let Some(hwd_elem) = h
            .children()
            .find(|n| n.is_element() && n.tag_name().name() == "HWD")
        {
            for inflx in hwd_elem
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "INFLX")
            {
                let plain = collect_text(inflx, &exclude);
                if plain.is_empty() || plain == hwd_base {
                    continue;
                }
                let inflx_label = format!(
                    "<h><v>{}</v> &rarr; {}</h>",
                    plain,
                    hwdlabel.clone()
                );
                items.push(Item {
                    itemtype: "hv".to_string(),
                    label: inflx_label,
                    path: path_root.clone(),
                    content: plain.clone(),
                    sortkey: plain.clone(),
                    asfilter: String::new(),
                    prio: 2,
                });
            }
        }
    }

    // ── Phrasal verbs (hp) ─────────────────────────────────────────────────
    for phrvb in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "PhrVbEntry")
    {
        let phrvbhwd = phrvb
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "PHRVBHWD");
        if let Some(phrvbhwd) = phrvbhwd {
            let plain = collect_text(phrvbhwd, &exclude);
            if plain.is_empty() {
                continue;
            }
            let path = elem_path(phrvb, &root_id_short, &path_root);
            let label =
                format!("<h><pv>{}</pv> <p>phrasal verb</p></h>", plain);
            items.push(Item {
                itemtype: "hp".to_string(),
                label,
                path,
                content: plain.clone(),
                sortkey: plain.clone(),
                asfilter: String::new(),
                prio: 1,
            });
        }
    }

    // ── Run-on derivatives (hm) ────────────────────────────────────────────
    for runon in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "RunOn")
    {
        let deriv = runon
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "DERIV");
        if let Some(deriv) = deriv {
            let base_elem = deriv
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "BASE");
            if let Some(base_elem) = base_elem {
                let mut plain = collect_text(base_elem, &exclude);
                // strip stress markers
                plain = plain.replace('\u{02c8}', "").replace('\u{02cc}', "");
                if plain.is_empty() {
                    continue;
                }
                let poslist: Vec<String> = runon
                    .descendants()
                    .filter(|n| n.is_element() && n.tag_name().name() == "POS")
                    .filter_map(|n| n.text().map(|s| s.to_string()))
                    .collect();
                let pos_str = poslist.join(", ");
                let label = if pos_str.is_empty() {
                    format!("<h><n>{}</n></h>", plain)
                } else {
                    format!("<h><n>{}</n> <p>{}</p></h>", plain, pos_str)
                };
                let path = elem_path(deriv, &root_id_short, &path_root);
                items.push(Item {
                    itemtype: "hm".to_string(),
                    label,
                    path,
                    content: plain.clone(),
                    sortkey: plain.clone(),
                    asfilter: String::new(),
                    prio: 1,
                });
            }
        }
    }

    // ── Lexical units / phrases (p, pl) ────────────────────────────────────
    for lexunit in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "LEXUNIT")
    {
        if lexunit.attribute("id").is_none() {
            continue;
        }
        let plain = collect_text(lexunit, &exclude);
        if plain.is_empty() {
            continue;
        }
        let path = elem_path(lexunit, &root_id_short, &path_root);
        let label = format!("<l><o>{}</o> ({})</l>", plain, hwdlabel.clone());
        items.push(Item {
            itemtype: "pl".to_string(),
            label,
            path,
            content: plain.clone(),
            sortkey: plain.clone(),
            asfilter: String::new(),
            prio: 9,
        });
    }

    // ── PROPFORM / PROPFORMPREP (p) ────────────────────────────────────────
    for tag_name in &["PROPFORM", "PROPFORMPREP"] {
        for elem in root
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == *tag_name)
        {
            if elem.attribute("id").is_none() {
                continue;
            }
            let plain = collect_text(elem, &exclude);
            if plain.is_empty() {
                continue;
            }
            let path = elem_path(elem, &root_id_short, &path_root);
            let label =
                format!("<c><o>{}</o> ({})</c>", plain, hwdlabel.clone());
            items.push(Item {
                itemtype: "p".to_string(),
                label,
                path,
                content: plain.clone(),
                sortkey: plain.clone(),
                asfilter: String::new(),
                prio: 10,
            });
        }
    }

    // ── Collocations: COLLO / COLLOC (p) ──────────────────────────────────
    for tag_name in &["COLLO", "COLLOC"] {
        for elem in root
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == *tag_name)
        {
            if elem.attribute("id").is_none() {
                continue;
            }
            let mut plain = collect_text(elem, &exclude);
            // remove leading article
            if plain.starts_with("a ") {
                plain = plain[2..].to_owned();
            } else if plain.starts_with("an ") {
                plain = plain[3..].to_owned();
            }
            if plain.is_empty() {
                continue;
            }
            let path = elem_path(elem, &root_id_short, &path_root);
            let label =
                format!("<c><o>{}</o> ({})</c>", plain, hwdlabel.clone());
            items.push(Item {
                itemtype: "p".to_string(),
                label,
                path,
                content: plain.clone(),
                sortkey: plain.clone(),
                asfilter: String::new(),
                prio: 10,
            });
        }
    }

    // ── Collocate structured entries (p + e) ───────────────────────────────
    // Each Collocate gives a collocation heading and its ColloExa examples.
    for collocate in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Collocate")
    {
        if collocate.attribute("id").is_none() {
            continue;
        }
        // Build collocation title from COLLOC + LEXVAR + ORTHVAR children
        let colloc_texts: Vec<String> = collocate
            .children()
            .filter(|n| {
                n.is_element()
                    && matches!(
                        n.tag_name().name(),
                        "COLLOC" | "LEXVAR" | "ORTHVAR"
                    )
            })
            .map(|n| collect_text(n, &exclude))
            .filter(|s| !s.is_empty())
            .collect();
        if colloc_texts.is_empty() {
            continue;
        }
        let colloc_title = colloc_texts.join(", ");

        // Index per-ColloExa example sentences
        for collexa in collocate
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "ColloExa")
        {
            let base = collexa
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "BASE");
            if let Some(base) = base {
                // Include COLLOINEXA text inside the example
                let mut ex_parts: Vec<String> = Vec::new();
                if let Some(t) = base.text() {
                    ex_parts.push(t.to_owned());
                }
                for child in base.children() {
                    let cn = child.tag_name().name();
                    if cn == "COLLOINEXA" {
                        ex_parts.push(collect_text(child, &exclude));
                    }
                    if let Some(t) = child.tail() {
                        ex_parts.push(t.to_owned());
                    }
                }
                let ex_text = collapse_space(&ex_parts.join(" "));
                if ex_text.is_empty() {
                    continue;
                }
                let path = elem_path(collocate, &root_id_short, &path_root);
                let ex_label = format!(
                    "{} &mdash; <b>{}</b>",
                    hwd_label,
                    colloc_title
                );
                items.push(Item {
                    itemtype: "e".to_string(),
                    label: ex_label,
                    path,
                    content: ex_text,
                    sortkey: hwd_base.clone(),
                    asfilter: String::new(),
                    prio: 20,
                });
            }
        }
    }

    // ── Thesaurus exponents (e, d) ─────────────────────────────────────────
    for exponent in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Exponent")
    {
        if exponent.attribute("id").is_none() {
            continue;
        }
        let path = elem_path(exponent, &root_id_short, &path_root);

        // ThesExa examples
        for thesexa in exponent
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "THESEXA")
        {
            if let Some(base) = thesexa
                .descendants()
                .find(|n| n.is_element() && n.tag_name().name() == "BASE")
            {
                let text = collect_text(base, &exclude);
                if !text.is_empty() {
                    items.push(Item {
                        itemtype: "e".to_string(),
                        label: hwd_label.clone(),
                        path: path.clone(),
                        content: text,
                        sortkey: hwd_base.clone(),
                        asfilter: String::new(),
                        prio: 20,
                    });
                }
            }
        }

        // DEF within exponent
        for def in exponent
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "DEF")
        {
            let text = collect_text(def, &exclude);
            if !text.is_empty() {
                items.push(Item {
                    itemtype: "d".to_string(),
                    label: hwd_label.clone(),
                    path: path.clone(),
                    content: text,
                    sortkey: hwd_base.clone(),
                    asfilter: String::new(),
                    prio: 30,
                });
            }
        }
    }

    // ── Definitions (DEF) -> d ─────────────────────────────────────────────
    for def in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "DEF")
    {
        let text = collect_text(def, &exclude);
        if text.is_empty() {
            continue;
        }
        let fullpath = elem_path(def, &root_id_short, &path_root);
        items.push(Item {
            itemtype: "d".to_string(),
            label: hwd_label.clone(),
            path: fullpath,
            content: text,
            sortkey: hwd_base.clone(),
            asfilter: String::new(),
            prio: 30,
        });
    }

    // ── Examples (EXAMPLE/BASE) -> e ──────────────────────────────────────
    for ex in root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "EXAMPLE")
    {
        if let Some(base) = ex
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "BASE")
        {
            // Collect text including COLLOINEXA spans
            let mut parts: Vec<String> = Vec::new();
            if let Some(t) = base.text() {
                parts.push(t.to_owned());
            }
            for child in base.children() {
                parts.push(collect_text(child, &exclude));
                if let Some(t) = child.tail() {
                    parts.push(t.to_owned());
                }
            }
            let text = collapse_space(&parts.join(" "));
            if text.is_empty() {
                continue;
            }
            // Use the EXAMPLE element's own id if present; otherwise walk up.
            let fullpath = elem_path(ex, &root_id_short, &path_root);
            items.push(Item {
                itemtype: "e".to_string(),
                label: hwd_label.clone(),
                path: fullpath.clone(),
                content: text,
                sortkey: hwd_base.clone(),
                asfilter: String::new(),
                prio: 20,
            });

            // Per-collocation sub-entries: COLLOINEXA within the example
            let colloinexa_texts: Vec<String> = base
                .descendants()
                .filter(|n| n.is_element() && n.tag_name().name() == "COLLOINEXA")
                .map(|n| collect_text(n, &exclude))
                .filter(|s| !s.is_empty())
                .collect();
            if !colloinexa_texts.is_empty() {
                let coplain = colloinexa_texts.join(" ");
                let colabel = format!(
                    "<c><o>{}</o> ({})</c>",
                    colloinexa_texts.join(" &hellip; "),
                    hwdlabel.clone()
                );
                items.push(Item {
                    itemtype: "p".to_string(),
                    label: colabel,
                    path: fullpath,
                    content: coplain.clone(),
                    sortkey: coplain,
                    asfilter: String::new(),
                    prio: 15,
                });
            }
        }
    }

    Ok((items, variations))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Source directory may be provided as the first argument. If it isn't, try
    // to auto-discover a likely `ldoce5.data` location.
    let src_dir: PathBuf = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else if let Some(pb) = data::discover_ldoce5_dir() {
        println!(
            "Auto-discovered LDOCE5 data directory: \"{}\"",
            pb.display()
        );
        pb
    } else {
        eprintln!("Usage: build_index /path/to/ldoce5.data [out_data_dir]");
        std::process::exit(2);
    };

    let out_dir = if args.len() > 2 {
        PathBuf::from(&args[2])
    } else {
        config::data_dir()
    };

    println!("Source: \"{}\"", src_dir.display());
    println!("Output: \"{}\"", out_dir.display());

    if !data::is_ldoce5_dir(&src_dir) {
        return Err(anyhow!(
            "{} does not look like a valid LDOCE5 data directory",
            src_dir.display()
        ));
    }

    fs::create_dir_all(&out_dir)
        .with_context(|| format!("create out dir {}", out_dir.display()))?;

    // paths
    let filemap_path = out_dir.join("filemap.cdb");
    let incremental_path = out_dir.join("incremental.db");
    let incremental_tmp = out_dir.join("incremental.tmp");
    let fulltext_hp_dir = out_dir.join("fulltext_hp");
    let fulltext_de_dir = out_dir.join("fulltext_de");

    let _ = fs::remove_file(&filemap_path);
    let _ = fs::remove_file(&incremental_path);
    let _ = fs::remove_file(&incremental_tmp);
    let _ = fs::remove_dir_all(&fulltext_hp_dir);
    let _ = fs::remove_dir_all(&fulltext_de_dir);

    // create makers
    println!("Creating filemap.cdb...");
    let f = File::create(&filemap_path).context("create filemap.cdb")?;
    let mut filemap_maker = CDBMaker::new(f).context("cdb maker")?;

    println!("Creating incremental maker...");
    let mut incr_maker =
        IncrementalMaker::new(&incremental_path, &incremental_tmp).context("incremental maker")?;

    println!("Creating fulltext makers...");
    let mut fulltext_hp =
        FulltextMaker::new(&fulltext_hp_dir).map_err(|e| anyhow!("fulltext hp maker: {:?}", e))?;
    let mut fulltext_de =
        FulltextMaker::new(&fulltext_de_dir).map_err(|e| anyhow!("fulltext de maker: {:?}", e))?;

    // iterate archives
    for &(name, _rel) in ARCHIVE_DIRS.iter() {
        println!("Scanning archive {}...", name);
        let files = match data::list_files(&src_dir, name) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("list_files error for {}: {:?}", name, e);
                continue;
            }
        };

        // lazily open ArchiveReader when needed
        let mut arch_reader_opt: Option<ArchiveReader> = None;

        for entry in files {
            // mapped name
            let mut mapped_name = entry.name.clone();
            if name == "picture" {
                if let Some(first) = entry.dir_path.get(0) {
                    mapped_name = format!("{}/{}", first, entry.name);
                }
            } else if name == "fs" || name == "pronpractice" {
                let arch_reader = arch_reader_opt.get_or_insert_with(|| {
                    ArchiveReader::new(&src_dir, name).expect("open arch reader")
                });
                match arch_reader.read(entry.location) {
                    Ok(d) => {
                        if let Ok(s) = std::str::from_utf8(&d) {
                            if let Ok(doc) = Document::parse(s) {
                                let root = doc.root_element();
                                if let Some(idv) = root.attribute("id") {
                                    mapped_name = shorten_id(idv);
                                } else if let Some(idm) = root.attribute("idm_id") {
                                    mapped_name = idm.to_string();
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            } else if mapped_name.ends_with(".xml") {
                let arch_reader = arch_reader_opt.get_or_insert_with(|| {
                    ArchiveReader::new(&src_dir, name).expect("open arch reader")
                });
                match arch_reader.read(entry.location) {
                    Ok(d) => {
                        if let Ok(s) = std::str::from_utf8(&d) {
                            if let Ok(doc) = Document::parse(s) {
                                let root = doc.root_element();
                                if let Some(idv) = root.attribute("id") {
                                    mapped_name = idv.to_string();
                                } else if let Some(idm) = root.attribute("idm_id") {
                                    mapped_name = idm.to_string();
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            // compute md5 key
            let md = md5_compute(format!("{}:{}", name, mapped_name));
            let key = &md[0..10];

            // pack location bytes
            let (cmpo, cmps, orgo, orgs) = entry.location;
            let mut val: Vec<u8> = Vec::new();
            if cmps < 65536 && orgo < 65536 && orgs < 65536 {
                val.extend_from_slice(&(cmpo as u32).to_le_bytes());
                val.extend_from_slice(&(cmps as u16).to_le_bytes());
                val.extend_from_slice(&(orgo as u16).to_le_bytes());
                val.extend_from_slice(&(orgs as u16).to_le_bytes());
            } else {
                val.extend_from_slice(&(cmpo as u32).to_le_bytes());
                val.extend_from_slice(&(cmps as u32).to_le_bytes());
                val.extend_from_slice(&(orgo as u32).to_le_bytes());
                val.extend_from_slice(&(orgs as u32).to_le_bytes());
            }

            filemap_maker
                .add(key, &val)
                .with_context(|| format!("cdb add {}:{}", name, mapped_name))?;

            // For fs archive, extract items and index
            if name == "fs" {
                let arch_reader = arch_reader_opt.get_or_insert_with(|| {
                    ArchiveReader::new(&src_dir, name).expect("open arch reader")
                });
                match arch_reader.read(entry.location) {
                    Ok(d) => {
                        match extract_items_from_entry(&d) {
                            Ok((items, _vars)) => {
                                for it in items {
                                    let firstc = it.itemtype.chars().next().unwrap_or('?');
                                    if firstc == 'p'
                                        || firstc == 'h'
                                        || firstc == 'a'
                                        || it.itemtype == "hm"
                                    {
                                        // add to incremental and fulltext_hp
                                        incr_maker
                                            .add_item(
                                                &it.content,
                                                &it.itemtype,
                                                &it.label,
                                                &it.path,
                                                it.prio,
                                            )
                                            .context("incr add")?;
                                        fulltext_hp
                                            .add_item(
                                                &it.itemtype,
                                                &it.content,
                                                &it.asfilter,
                                                &it.label,
                                                &it.path,
                                                it.prio as u64,
                                                &it.sortkey,
                                            )
                                            .map_err(|e| anyhow!("fulltext hp add: {:?}", e))?;
                                    }
                                    if it.itemtype == "d" || it.itemtype == "e" {
                                        fulltext_de
                                            .add_item(
                                                &it.itemtype,
                                                &it.content,
                                                &it.asfilter,
                                                &it.label,
                                                &it.path,
                                                it.prio as u64,
                                                &it.sortkey,
                                            )
                                            .map_err(|e| anyhow!("fulltext de add: {:?}", e))?;
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "warning: extract item failed for {}: {:?}",
                                    mapped_name, e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("warning: read fs entry {} failed: {:?}", mapped_name, e);
                    }
                }
            }
        }
    }

    println!("Finalizing incremental...");
    incr_maker.finalize().context("incr finalize")?;

    println!("Committing fulltext hp...");
    fulltext_hp
        .commit()
        .map_err(|e| anyhow!("commit hp: {:?}", e))?;
    println!("Committing fulltext de...");
    fulltext_de
        .commit()
        .map_err(|e| anyhow!("commit de: {:?}", e))?;

    println!("Finalizing filemap.cdb...");
    filemap_maker.finalize().context("finalize filemap")?;

    println!("Done.");
    Ok(())
}
