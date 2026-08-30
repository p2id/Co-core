pub fn run(resource: &str) -> anyhow::Result<()> {
    let storage = super::open_storage()?;
    match storage.get_resource(resource).map_err(|e| anyhow::anyhow!(e.to_string()))? {
        Some(r) => {
            println!("RESOURCE {}", r.id);
            println!("  provider:    {}", r.provider);
            println!("  kind:        {}", r.kind);
            println!("  observed_at: {}", r.observed_at);
            println!("  source:      {}", r.source);
            let pretty: serde_json::Value = serde_json::from_str(&r.attributes_json)
                .unwrap_or(serde_json::Value::Null);
            println!("  attributes:  {}", serde_json::to_string_pretty(&pretty)?);
        }
        None => {
            println!("No resource found with id {resource}");
            println!("(run `forge scan` first, or check the id with `forge state --json`)");
        }
    }
    Ok(())
}
