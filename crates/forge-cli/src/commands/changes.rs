pub fn run(json: bool) -> anyhow::Result<()> {
    let storage = super::open_storage()?;
    let changes = storage
        .list_changes(50)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if json {
        let items: Vec<serde_json::Value> = changes
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "target": c.target,
                    "operation": c.operation,
                    "source": c.source,
                    "actor": c.actor,
                    "timestamp": c.timestamp,
                    "confidence": c.confidence,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else if changes.is_empty() {
        println!("No changes recorded yet. Run `forge plan` to detect changes against DESIRED state.");
    } else {
        println!("CHANGES");
        println!();
        for c in changes {
            println!("  {}  {}  {} -> {}", c.id, c.operation, c.target, c.timestamp);
        }
    }
    Ok(())
}
