//! Filesystem plugin (Phase 12). Scans a root directory and represents
//! each file as a `fs://file/<relative path>` Resource. This is the
//! simplest possible plugin, kept in-process for v0.1 (see forge-plugin's
//! module docs re: the staged WASM migration in Phase 15) so the
//! Change/Plan/Execute/Verify pipeline has something real to run against
//! before Git/Docker are wired up.

use forge_core::{CompensationSupport, ForgeError, ResourceId, Timestamp};
use forge_plugin::Plugin;
use forge_resource::{Observation, Relation, Resource};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

pub struct FilesystemPlugin {
    root: PathBuf,
}

impl FilesystemPlugin {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl Plugin for FilesystemPlugin {
    fn name(&self) -> &str {
        "filesystem"
    }

    fn scan(&self) -> Result<Vec<Resource>, ForgeError> {
        let mut resources = Vec::new();
        for entry in walkdir::WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let rel = entry
                .path()
                .strip_prefix(&self.root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            let id = ResourceId::parse(format!("fs://file/{rel}"))
                .map_err(|e| ForgeError::Storage(e.to_string()))?;

            let mut resource = Resource::new(id, "filesystem");
            if let Ok(meta) = entry.metadata() {
                resource
                    .attributes
                    .insert("size_bytes".to_string(), json!(meta.len()));
            }
            resources.push(resource);
        }
        Ok(resources)
    }

    fn discover_relations(&self, resources: &[Resource]) -> Result<Vec<Relation>, ForgeError> {
        // v0.1: no cross-file relations discovered yet (e.g. import graphs
        // are a future plugin capability, not filesystem's job).
        let _ = resources;
        Ok(Vec::new())
    }

    fn observe(&self, resource_id: &str) -> Result<Vec<Observation>, ForgeError> {
        let id = ResourceId::parse(resource_id).map_err(|e| ForgeError::Storage(e.to_string()))?;
        let rel_path = resource_id
            .strip_prefix("fs://file/")
            .ok_or_else(|| ForgeError::Storage("not a filesystem resource".into()))?;
        let full_path = self.root.join(rel_path);

        let mut observations = Vec::new();
        match fs::metadata(&full_path) {
            Ok(meta) => {
                observations.push(Observation {
                    resource: id.clone(),
                    attribute: "exists".to_string(),
                    value: json!(true),
                    observed_at: Timestamp::now(),
                    source: "filesystem".to_string(),
                });
                observations.push(Observation {
                    resource: id,
                    attribute: "size_bytes".to_string(),
                    value: json!(meta.len()),
                    observed_at: Timestamp::now(),
                    source: "filesystem".to_string(),
                });
            }
            Err(_) => {
                observations.push(Observation {
                    resource: id,
                    attribute: "exists".to_string(),
                    value: json!(false),
                    observed_at: Timestamp::now(),
                    source: "filesystem".to_string(),
                });
            }
        }
        Ok(observations)
    }

    fn execute(&self, kind: &str, params: &Value) -> Result<Value, ForgeError> {
        match kind {
            "filesystem.write" => {
                let path = params
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ForgeError::Storage("missing 'path' param".into()))?;
                let content = params
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let full_path = self.root.join(path);
                fs::write(&full_path, content)
                    .map_err(|e| ForgeError::Storage(e.to_string()))?;
                Ok(json!({ "written": path }))
            }
            other => Err(ForgeError::Storage(format!("unsupported action kind: {other}"))),
        }
    }

    fn compensation_support(&self, kind: &str) -> CompensationSupport {
        match kind {
            // Overwriting a file is compensable only if the previous
            // content was captured in the Change's `before` snapshot —
            // that capture happens in forge-change, not here, so this
            // plugin honestly reports `Partial` rather than assuming it.
            "filesystem.write" => CompensationSupport::Partial,
            _ => CompensationSupport::Unsupported,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn scan_finds_files_under_root() {
        let dir = tempfile::tempdir().unwrap();
        let mut f = fs::File::create(dir.path().join("hello.txt")).unwrap();
        writeln!(f, "hi").unwrap();

        let plugin = FilesystemPlugin::new(dir.path());
        let resources = plugin.scan().unwrap();
        assert_eq!(resources.len(), 1);
        assert!(resources[0].id.as_str().ends_with("hello.txt"));
    }

    #[test]
    fn observe_reports_exists_false_for_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = FilesystemPlugin::new(dir.path());
        let obs = plugin.observe("fs://file/does-not-exist.txt").unwrap();
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].value, json!(false));
    }
}
