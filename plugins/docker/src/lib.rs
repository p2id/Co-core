//! Docker plugin — Phase 14, STUB.
//!
//! Not yet implemented. Planned shape (do not deviate without an ADR):
//! - `scan()` calls the Docker Engine API (`/containers/json`,
//!   `/images/json`) and represents each as `docker://container/<name>`
//!   / `docker://image/<id>`.
//! - `discover_relations()` emits `runs_on` (container -> image) and
//!   `depends_on` from `depends_on:` in an associated Compose file, if
//!   present.
//! - `execute("docker.update_image", ...)` is the operation used by the
//!   MVP scenario in Section 26/27 of the master context.
//! - `compensation_support("docker.update_image")` = `Supported`
//!   (restore previous image tag) as in Section 17's example.
//!
//! Left unimplemented intentionally rather than faked (Section 33 rule 9).

use forge_core::{CompensationSupport, ForgeError};
use forge_plugin::Plugin;
use forge_resource::{Observation, Relation, Resource};
use serde_json::Value;

pub struct DockerPlugin;

impl Plugin for DockerPlugin {
    fn name(&self) -> &str {
        "docker"
    }
    fn scan(&self) -> Result<Vec<Resource>, ForgeError> {
        Err(ForgeError::Storage("docker plugin: not yet implemented (Phase 14)".into()))
    }
    fn discover_relations(&self, _resources: &[Resource]) -> Result<Vec<Relation>, ForgeError> {
        Err(ForgeError::Storage("docker plugin: not yet implemented (Phase 14)".into()))
    }
    fn observe(&self, _resource_id: &str) -> Result<Vec<Observation>, ForgeError> {
        Err(ForgeError::Storage("docker plugin: not yet implemented (Phase 14)".into()))
    }
    fn execute(&self, _kind: &str, _params: &Value) -> Result<Value, ForgeError> {
        Err(ForgeError::Storage("docker plugin: not yet implemented (Phase 14)".into()))
    }
    fn compensation_support(&self, kind: &str) -> CompensationSupport {
        match kind {
            "docker.update_image" => CompensationSupport::Supported,
            _ => CompensationSupport::Unsupported,
        }
    }
}
