use super::{db_path, forge_dir};
use forge_storage::Storage;
use std::fs;

pub fn run() -> anyhow::Result<()> {
    fs::create_dir_all(forge_dir().join("plugins"))?;

    let config_path = forge_dir().join("config.toml");
    if !config_path.exists() {
        fs::write(
            &config_path,
            "# Forge configuration (Section 25)\n\
             environment = \"development\"\n\n\
             [policy]\n\
             require_verification = true\n\
             require_compensation = true\n\
             require_approval = true\n\n\
             [plugins]\n\
             enabled = [\"filesystem\"]\n",
        )?;
    }

    // Creating the Storage runs migrations (ADR 0002), so `forge.db`
    // exists with schema in place after this call.
    Storage::open(db_path().to_str().expect("valid utf8 path"))
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    println!("Initialized Forge in {}/", super::FORGE_DIR);
    println!("  {}/config.toml", super::FORGE_DIR);
    println!("  {}/{}", super::FORGE_DIR, super::DB_FILE);
    Ok(())
}
