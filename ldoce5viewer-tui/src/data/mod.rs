//! Data layer: CDB constant database and IDM archive reader.

pub mod cdb;
pub mod idm_reader;

pub use cdb::{CDBMaker, CDBReader, CDBError};
pub use idm_reader::{ArchiveReader, ArchiveError, FileEntry, ARCHIVE_DIRS, is_ldoce5_dir, list_files};
