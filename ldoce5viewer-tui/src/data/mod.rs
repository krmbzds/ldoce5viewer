//! Data layer: CDB constant database and IDM archive reader.

pub mod cdb;
pub mod idm_reader;

use std::path::PathBuf;

pub use cdb::{CDBError, CDBMaker, CDBReader};
pub use idm_reader::{
    is_ldoce5_dir, list_files, ArchiveError, ArchiveReader, FileEntry, ARCHIVE_DIRS,
};

/// Try to discover `ldoce5.data` in a set of common locations for the current
/// platform. Returns `Some(PathBuf)` when a valid LDOCE5 data directory is
/// found (validated with `is_ldoce5_dir`), otherwise `None`.
pub fn discover_ldoce5_dir() -> Option<PathBuf> {
    use std::env;

    let mut candidates: Vec<PathBuf> = Vec::new();

    // Current working directory
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("ldoce5.data"));
    }

    // Home directory
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join("ldoce5.data"));
        candidates.push(home.join("Downloads").join("ldoce5.data"));
    }

    // Platform-specific common locations
    #[cfg(target_os = "windows")]
    {
        if let Ok(pf) = env::var("ProgramFiles") {
            candidates.push(
                PathBuf::from(pf)
                    .join("Longman")
                    .join("LDOCE5")
                    .join("ldoce5.data"),
            );
        }
        if let Ok(pfx) = env::var("ProgramFiles(x86)") {
            candidates.push(
                PathBuf::from(pfx)
                    .join("Longman")
                    .join("LDOCE5")
                    .join("ldoce5.data"),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/Volumes/Longman_Dictionary/ldoce5.data"));
        candidates.push(PathBuf::from("/Volumes/Longman_Dictiona/ldoce5.data"));
        candidates.push(PathBuf::from("/Applications/ldoce5.data"));

        // Scan /Volumes for volumes that contain "longman" in the name and
        // probe for `ldoce5.data` there.
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for e in entries.flatten() {
                if let Some(name) = e.file_name().to_str() {
                    let lname = name.to_lowercase();
                    if lname.contains("longman") {
                        candidates.push(PathBuf::from("/Volumes").join(name).join("ldoce5.data"));
                    }
                }
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        candidates.push(PathBuf::from("/usr/share/ldoce5/ldoce5.data"));
        candidates.push(PathBuf::from("/usr/local/share/ldoce5/ldoce5.data"));
        candidates.push(PathBuf::from("/opt/Longman/ldoce5/ldoce5.data"));
    }

    // Try discovered candidates and return the first one that validates.
    for p in candidates.into_iter() {
        if p.exists() && idm_reader::is_ldoce5_dir(&p) {
            return Some(p);
        }
    }

    None
}
