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
use ldoce5viewer_tui::data::{self, ArchiveReader, ARCHIVE_DIRS, CDBMaker};
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

fn extract_items_from_entry(entry_data: &[u8]) -> Result<(Vec<Item>, HashMap<String, Vec<String>>)> {
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
        .and_then(|h| h.descendants().find(|n| n.is_element() && n.tag_name().name() == "HWD") )
        .and_then(|hwd| hwd.descendants().find(|n| n.is_element() && n.tag_name().name() == "BASE"))
        .map(|n| collect_text(n, &exclude))
        .unwrap_or_default();

    // is_freq
    let is_freq = head.and_then(|h| h.descendants().find(|n| n.is_element() && n.tag_name().name() == "FREQ")).is_some();

    // build hwdlabel similar to Python's make_hwd_label
    let mut baselabel = hwd_base.clone();
    // HOMNUM
    if let Some(h) = head {
        if let Some(homnum) = h.descendants().find(|n| n.is_element() && n.tag_name().name() == "HOMNUM") {
            let hom = collect_text(homnum, &exclude);
            if !hom.is_empty() {
                baselabel.push_str(&format!("<s>{}</s>", hom));
            }
        }
    }
    let mut hwdlabel = if is_freq { format!("<f>{}</f>", baselabel) } else { format!("<n>{}</n>", baselabel) };
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

    // headword item
    let path_root = format!("/fs/{}", root_id_short);
    let item = Item {
        itemtype: "hm".to_string(),
        label: format!("<h>{}</h>", hwdlabel.clone()),
        path: path_root.clone(),
        content: collect_text(root, &exclude),
        sortkey: hwd_base.clone(),
        asfilter: String::new(),
        prio: 1,
    };
    items.push(item);

    // Definitions (DEF) -> d
    for def in root.descendants().filter(|n| n.is_element() && n.tag_name().name() == "DEF") {
        let text = collect_text(def, &exclude);
        if text.is_empty() { continue; }
        // find nearest ancestor with id
        let mut anc = def;
        while anc.attribute("id").is_none() {
            if let Some(p) = anc.parent() { anc = p; } else { break; }
        }
        let pid = anc.attribute("id").unwrap_or("");
        let fullpath = if pid.is_empty() { path_root.clone() } else { format!("/fs/{}#{}", root_id_short, shorten_id(pid)) };
        items.push(Item { itemtype: "d".to_string(), label: format!("<h>{}</h>", hwdlabel.clone()), path: fullpath, content: text, sortkey: hwd_base.clone(), asfilter: String::new(), prio: 30 });
    }

    // Examples
    for ex in root.descendants().filter(|n| n.is_element() && n.tag_name().name() == "EXAMPLE") {
        if let Some(base) = ex.descendants().find(|n| n.is_element() && n.tag_name().name() == "BASE") {
            let text = collect_text(base, &exclude);
            if text.is_empty() { continue; }
            let mut anc = ex;
            while anc.attribute("id").is_none() {
                if let Some(p) = anc.parent() { anc = p; } else { break; }
            }
            let pid = anc.attribute("id").unwrap_or("");
            let fullpath = if pid.is_empty() { path_root.clone() } else { format!("/fs/{}#{}", root_id_short, shorten_id(pid)) };
            items.push(Item { itemtype: "e".to_string(), label: format!("<h>{}</h>", hwdlabel.clone()), path: fullpath, content: text, sortkey: hwd_base.clone(), asfilter: String::new(), prio: 20 });
        }
    }

    Ok((items, variations))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: build_index /path/to/ldoce5.data [out_data_dir]");
        std::process::exit(2);
    }
    let src_dir = PathBuf::from(&args[1]);
    let out_dir = if args.len() > 2 { PathBuf::from(&args[2]) } else { config::data_dir() };

    println!("Source: {}", src_dir.display());
    println!("Output: {}", out_dir.display());

    if !data::is_ldoce5_dir(&src_dir) {
        return Err(anyhow!("{} does not look like a valid LDOCE5 data directory", src_dir.display()));
    }

    fs::create_dir_all(&out_dir).with_context(|| format!("create out dir {}", out_dir.display()))?;

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
    let mut incr_maker = IncrementalMaker::new(&incremental_path, &incremental_tmp).context("incremental maker")?;

    println!("Creating fulltext makers...");
    let mut fulltext_hp = FulltextMaker::new(&fulltext_hp_dir).map_err(|e| anyhow!("fulltext hp maker: {:?}", e))?;
    let mut fulltext_de = FulltextMaker::new(&fulltext_de_dir).map_err(|e| anyhow!("fulltext de maker: {:?}", e))?;

    // iterate archives
    for &(name, _rel) in ARCHIVE_DIRS.iter() {
        println!("Scanning archive {}...", name);
        let files = match data::list_files(&src_dir, name) {
            Ok(v) => v,
            Err(e) => { eprintln!("list_files error for {}: {:?}", name, e); continue; }
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
                let arch_reader = arch_reader_opt.get_or_insert_with(|| ArchiveReader::new(&src_dir, name).expect("open arch reader"));
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
                let arch_reader = arch_reader_opt.get_or_insert_with(|| ArchiveReader::new(&src_dir, name).expect("open arch reader"));
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

            filemap_maker.add(key, &val).with_context(|| format!("cdb add {}:{}", name, mapped_name))?;

            // For fs archive, extract items and index
            if name == "fs" {
                let arch_reader = arch_reader_opt.get_or_insert_with(|| ArchiveReader::new(&src_dir, name).expect("open arch reader"));
                match arch_reader.read(entry.location) {
                    Ok(d) => {
                        match extract_items_from_entry(&d) {
                            Ok((items, _vars)) => {
                                for it in items {
                                    let firstc = it.itemtype.chars().next().unwrap_or('?');
                                    if firstc == 'p' || firstc == 'h' || firstc == 'a' || it.itemtype == "hm" {
                                        // add to incremental and fulltext_hp
                                        incr_maker.add_item(&it.content, &it.itemtype, &it.label, &it.path, it.prio).context("incr add")?;
                                        fulltext_hp.add_item(&it.itemtype, &it.content, &it.asfilter, &it.label, &it.path, it.prio as u64, &it.sortkey).map_err(|e| anyhow!("fulltext hp add: {:?}", e))?;
                                    }
                                    if it.itemtype == "d" || it.itemtype == "e" {
                                        fulltext_de.add_item(&it.itemtype, &it.content, &it.asfilter, &it.label, &it.path, it.prio as u64, &it.sortkey).map_err(|e| anyhow!("fulltext de add: {:?}", e))?;
                                    }
                                }
                            }
                            Err(e) => { eprintln!("warning: extract item failed for {}: {:?}", mapped_name, e); }
                        }
                    }
                    Err(e) => { eprintln!("warning: read fs entry {} failed: {:?}", mapped_name, e); }
                }
            }
        }
    }

    println!("Finalizing incremental...");
    incr_maker.finalize().context("incr finalize")?;

    println!("Committing fulltext hp...");
    fulltext_hp.commit().map_err(|e| anyhow!("commit hp: {:?}", e))?;
    println!("Committing fulltext de...");
    fulltext_de.commit().map_err(|e| anyhow!("commit de: {:?}", e))?;

    println!("Finalizing filemap.cdb...");
    filemap_maker.finalize().context("finalize filemap")?;

    println!("Done.");
    Ok(())
}
