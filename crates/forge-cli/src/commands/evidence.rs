pub fn run(change: &str) -> anyhow::Result<()> {
    let storage = super::open_storage()?;
    let ids = storage
        .evidence_for_change(change)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if ids.is_empty() {
        println!("No evidence found for change {change}");
    } else {
        println!("Evidence for {change}:");
        for id in ids {
            println!("  {id}");
        }
    }
    Ok(())
}
