//! Git plugin (Phase 13).
//!
//! Shells out to the system `git` binary via `std::process::Command`
//! rather than embedding a Git library. This keeps the plugin's own
//! dependency footprint minimal and its behavior identical to whatever
//! `git` the operator already has installed and configured (credentials,
//! hooks, `.gitconfig`, etc.) — the same tradeoff a human operator running
//! `git` commands directly would make.
//!
//! KNOWN GAP (ADR 0007): calls to `Command::new("git")` here are not yet
//! routed through `forge_plugin::HostCapabilityGate`. That wiring lands
//! with the Phase 15 WASM boundary (ADR 0005); until then, this plugin's
//! access to the repository is unrestricted by Forge's own policy layer.
//!
//! Scope for v0.1:
//! - The repository itself is one `Resource` (`git://repository/<name>`)
//!   carrying `branch`, `head_sha`, `dirty`, and `remote_url` attributes.
//! - Each tracked file is a `Resource` (`git://file/<relative path>`)
//!   carrying its current `blob_sha`.
//! - A `Contains` relation links the repository to each tracked file
//!   (Section 11), giving `forge-graph` something real to traverse once
//!   `forge plan` is extended beyond the filesystem plugin.
//! - `execute` supports `git.commit` (stage + commit) and `git.checkout`
//!   (checkout a ref). Anything else is rejected, not guessed at.

use forge_core::{CompensationSupport, ForgeError, ResourceId, Timestamp};
use forge_plugin::Plugin;
use forge_resource::{Observation, Relation, RelationKind, Resource};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct GitPlugin {
    root: PathBuf,
}

impl GitPlugin {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn git(&self, args: &[&str]) -> Result<String, ForgeError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .output()
            .map_err(|e| ForgeError::Storage(format!("failed to invoke git: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(ForgeError::Storage(format!(
                "git {} failed: {stderr}",
                args.join(" ")
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn is_git_repo(&self) -> bool {
        self.git(&["rev-parse", "--is-inside-work-tree"])
            .map(|s| s == "true")
            .unwrap_or(false)
    }

    fn repository_name(&self) -> Result<String, ForgeError> {
        let top = self.git(&["rev-parse", "--show-toplevel"])?;
        Ok(Path::new(&top)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "repository".to_string()))
    }

    fn repository_resource(&self) -> Result<Resource, ForgeError> {
        let name = self.repository_name()?;
        let id = ResourceId::parse(format!("git://repository/{name}"))
            .map_err(|e| ForgeError::Storage(e.to_string()))?;
        let mut resource = Resource::new(id, "git");

        let branch = self
            .git(&["rev-parse", "--abbrev-ref", "HEAD"])
            .unwrap_or_else(|_| "HEAD".to_string());
        let head_sha = self.git(&["rev-parse", "HEAD"]).unwrap_or_default();
        let dirty = !self
            .git(&["status", "--porcelain"])
            .unwrap_or_default()
            .is_empty();
        let remote_url = self.git(&["remote", "get-url", "origin"]).ok();

        resource.attributes.insert("branch".to_string(), json!(branch));
        resource.attributes.insert("head_sha".to_string(), json!(head_sha));
        resource.attributes.insert("dirty".to_string(), json!(dirty));
        if let Some(url) = remote_url {
            resource.attributes.insert("remote_url".to_string(), json!(url));
        }
        Ok(resource)
    }

    fn tracked_files(&self) -> Result<Vec<(String, String)>, ForgeError> {
        // `git ls-files -s` prints "<mode> <blob_sha> <stage> <path>".
        let raw = self.git(&["ls-files", "-s"])?;
        let mut files = Vec::new();
        for line in raw.lines() {
            let mut parts = line.split_whitespace();
            let _mode = parts.next();
            let blob_sha = parts.next().unwrap_or_default().to_string();
            let _stage = parts.next();
            let path = line
                .splitn(4, char::is_whitespace)
                .nth(3)
                .unwrap_or_default()
                .to_string();
            if !path.is_empty() {
                files.push((path, blob_sha));
            }
        }
        Ok(files)
    }
}

impl Plugin for GitPlugin {
    fn name(&self) -> &str {
        "git"
    }

    fn scan(&self) -> Result<Vec<Resource>, ForgeError> {
        if !self.is_git_repo() {
            return Err(ForgeError::Storage(format!(
                "{} is not inside a git working tree",
                self.root.display()
            )));
        }

        let mut resources = vec![self.repository_resource()?];

        for (path, blob_sha) in self.tracked_files()? {
            let id = ResourceId::parse(format!("git://file/{path}"))
                .map_err(|e| ForgeError::Storage(e.to_string()))?;
            let mut resource = Resource::new(id, "git");
            resource
                .attributes
                .insert("blob_sha".to_string(), json!(blob_sha));
            resources.push(resource);
        }

        Ok(resources)
    }

    fn discover_relations(&self, resources: &[Resource]) -> Result<Vec<Relation>, ForgeError> {
        let repo = resources
            .iter()
            .find(|r| r.kind() == "repository" && r.provider() == "git");
        let repo = match repo {
            Some(r) => r,
            None => return Ok(Vec::new()),
        };

        let mut relations = Vec::new();
        for resource in resources {
            if resource.id == repo.id {
                continue;
            }
            if resource.provider() == "git" && resource.kind() == "file" {
                relations.push(Relation {
                    from: repo.id.clone(),
                    to: resource.id.clone(),
                    kind: RelationKind::Contains,
                    discovered_by: "git".to_string(),
                    discovered_at: Timestamp::now(),
                });
            }
        }
        Ok(relations)
    }

    fn observe(&self, resource_id: &str) -> Result<Vec<Observation>, ForgeError> {
        let id = ResourceId::parse(resource_id).map_err(|e| ForgeError::Storage(e.to_string()))?;

        if resource_id.starts_with("git://repository/") {
            let repo = self.repository_resource()?;
            return Ok(repo
                .attributes
                .into_iter()
                .map(|(attribute, value)| Observation {
                    resource: id.clone(),
                    attribute,
                    value,
                    observed_at: Timestamp::now(),
                    source: "git".to_string(),
                })
                .collect());
        }

        if let Some(path) = resource_id.strip_prefix("git://file/") {
            let files = self.tracked_files()?;
            let found = files.iter().find(|(p, _)| p == path);
            return Ok(match found {
                Some((_, sha)) => vec![
                    Observation {
                        resource: id.clone(),
                        attribute: "exists".to_string(),
                        value: json!(true),
                        observed_at: Timestamp::now(),
                        source: "git".to_string(),
                    },
                    Observation {
                        resource: id,
                        attribute: "blob_sha".to_string(),
                        value: json!(sha),
                        observed_at: Timestamp::now(),
                        source: "git".to_string(),
                    },
                ],
                None => vec![Observation {
                    resource: id,
                    attribute: "exists".to_string(),
                    value: json!(false),
                    observed_at: Timestamp::now(),
                    source: "git".to_string(),
                }],
            });
        }

        Err(ForgeError::Storage(format!(
            "unrecognized git resource id: {resource_id}"
        )))
    }

    fn execute(&self, kind: &str, params: &Value) -> Result<Value, ForgeError> {
        match kind {
            "git.commit" => {
                let message = params
                    .get("message")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ForgeError::Storage("missing 'message' param".into()))?;
                self.git(&["add", "-A"])?;
                let sha_before = self.git(&["rev-parse", "HEAD"]).unwrap_or_default();
                self.git(&["commit", "-m", message])?;
                let sha_after = self.git(&["rev-parse", "HEAD"])?;
                Ok(json!({ "before_sha": sha_before, "after_sha": sha_after }))
            }
            "git.checkout" => {
                let reference = params
                    .get("ref")
                    .and_then(Value::as_str)
                    .ok_or_else(|| ForgeError::Storage("missing 'ref' param".into()))?;
                let sha_before = self.git(&["rev-parse", "HEAD"]).unwrap_or_default();
                self.git(&["checkout", reference])?;
                Ok(json!({ "before_sha": sha_before, "checked_out": reference }))
            }
            other => Err(ForgeError::Storage(format!("unsupported action kind: {other}"))),
        }
    }

    fn compensation_support(&self, kind: &str) -> CompensationSupport {
        match kind {
            // `git revert`/`git reset --hard <before_sha>` is a real,
            // reliable inverse as long as the commit hasn't been pushed
            // and rewritten elsewhere — Supported for the local-only case
            // this plugin operates in.
            "git.commit" => CompensationSupport::Supported,
            // Checking back out the previous ref is possible, but any
            // uncommitted working-tree changes made between the two
            // checkouts are not recoverable by this plugin — Partial,
            // not Supported (ADR 0004: never overclaim compensation).
            "git.checkout" => CompensationSupport::Partial,
            _ => CompensationSupport::Unsupported,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn init_repo(dir: &Path) {
        Command::new("git").arg("-C").arg(dir).args(["init", "-q"]).status().ok();
        Command::new("git").arg("-C").arg(dir).args(["config", "user.email", "test@example.com"]).status().ok();
        Command::new("git").arg("-C").arg(dir).args(["config", "user.name", "Test"]).status().ok();
    }

    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    #[test]
    fn scan_fails_gracefully_outside_a_repo() {
        if !git_available() {
            return; // environment has no git binary — nothing to assert
        }
        let dir = tempfile::tempdir().unwrap();
        let plugin = GitPlugin::new(dir.path());
        assert!(plugin.scan().is_err());
    }

    #[test]
    fn scan_finds_repository_and_tracked_file() {
        if !git_available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        init_repo(dir.path());
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        Command::new("git").arg("-C").arg(dir.path()).args(["add", "a.txt"]).status().ok();
        Command::new("git").arg("-C").arg(dir.path()).args(["commit", "-q", "-m", "init"]).status().ok();

        let plugin = GitPlugin::new(dir.path());
        let resources = plugin.scan().unwrap();
        assert!(resources.iter().any(|r| r.kind() == "repository"));
        assert!(resources.iter().any(|r| r.id.as_str() == "git://file/a.txt"));
    }
}
