pub mod init;
pub mod scan;
pub mod state;
pub mod changes;
pub mod inspect;
pub mod history;
pub mod evidence;
pub mod plan;
pub mod apply;
pub mod stub;

use forge_storage::Storage;
use std::path::PathBuf;

pub const FORGE_DIR: &str = ".forge";
pub const DB_FILE: &str = "forge.db";

pub fn forge_dir() -> PathBuf {
    PathBuf::from(FORGE_DIR)
}

pub fn db_path() -> PathBuf {
    forge_dir().join(DB_FILE)
}

pub fn open_storage() -> anyhow::Result<Storage> {
    if !db_path().exists() {
        anyhow::bail!("no .forge/forge.db found — run `forge init` first");
    }
    Storage::open(db_path().to_str().expect("valid utf8 path"))
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}
