use forge_plugin::Plugin;
use forge_plugin_filesystem::FilesystemPlugin;

pub fn run() -> anyhow::Result<()> {
    let storage = super::open_storage()?;

    // v0.1: filesystem plugin only, rooted at the current directory,
    // skipping .forge/ itself. Git/Docker plugins are stubs (Phases 13-14).
    let plugin = FilesystemPlugin::new(".");
    let resources = plugin
        .scan()
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    for resource in &resources {
        if resource.id.as_str().starts_with("fs://file/.forge/") {
            continue;
        }
        let attrs = serde_json::to_string(&resource.attributes)?;
        storage
            .upsert_resource(
                resource.id.as_str(),
                resource.provider(),
                resource.kind(),
                &attrs,
                &resource.observed_at.to_rfc3339(),
                &resource.source,
            )
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    }

    let total = storage.count_resources().map_err(|e| anyhow::anyhow!(e.to_string()))?;
    println!("FORGE STATE");
    println!();
    println!("Resources       {total}");
    println!("(git/docker plugins not yet implemented — Phases 13-14)");
    Ok(())
}
