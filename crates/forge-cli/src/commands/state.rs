pub fn run(json: bool) -> anyhow::Result<()> {
    let storage = super::open_storage()?;
    let total = storage.count_resources().map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if json {
        println!("{}", serde_json::json!({ "resources": total }));
    } else {
        println!("FORGE STATE");
        println!();
        println!("Resources       {total}");
    }
    Ok(())
}
