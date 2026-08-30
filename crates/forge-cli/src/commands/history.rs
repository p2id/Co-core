pub fn run(json: bool) -> anyhow::Result<()> {
    let storage = super::open_storage()?;
    let evidence = storage
        .list_evidence(50)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    if json {
        let items: Vec<serde_json::Value> = evidence
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "type": e.evidence_type,
                    "timestamp": e.timestamp,
                    "change": e.change_id,
                    "plan": e.plan_id,
                    "execution": e.execution_id,
                    "result": e.result,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else if evidence.is_empty() {
        println!("No evidence recorded yet. Run `forge plan` then `forge apply` to produce some.");
    } else {
        println!("HISTORY");
        println!();
        for e in evidence {
            let result = e.result.as_deref().unwrap_or("-");
            println!("  {}  {}  {}  result={}", e.timestamp, e.id, e.evidence_type, result);
        }
    }
    Ok(())
}
