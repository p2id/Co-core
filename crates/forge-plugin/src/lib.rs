//! Plugin boundary (Sections 22-23, ADR 0005).
//!
//! v0.1 status: this crate defines the `Plugin` trait and the
//! `HostCapabilityGate`, which every host-function call (filesystem, git,
//! docker, process, network) must pass through. The first-party
//! filesystem/git/docker plugins in `plugins/` implement this trait
//! in-process for now; Phase 15 swaps the implementation to load
//! `wasm32-wasip1` modules through `wasmtime` behind the *same* trait, so
//! `forge-cli` and `forge-exec` do not need to change. This staged
//! approach lets the deterministic Core/state-machine work (Phases 1-11)
//! proceed without blocking on the WASM sandbox.
//!
//! Regardless of in-process or WASM backing, every plugin call MUST go
//! through `HostCapabilityGate::check`, which defaults to DENY
//! (Section 23) using `forge_policy::Policy`.

use forge_core::ForgeError;
use forge_policy::{Capability, Decision, Policy};
use forge_resource::{Observation, Relation, Resource};
use serde_json::Value;

/// A single request a plugin makes of the host — every one of these must
/// be capability-checked before it runs (Section 23).
#[derive(Debug, Clone)]
pub enum HostRequest {
    FilesystemRead { path: String },
    FilesystemWrite { path: String },
    ProcessSpawn { command: String },
    NetworkConnect { host: String },
    DockerInspect { container: String },
    DockerExecute { container: String, command: String },
}

impl HostRequest {
    pub fn required_capability(&self) -> Capability {
        let name = match self {
            HostRequest::FilesystemRead { .. } => "filesystem.read",
            HostRequest::FilesystemWrite { .. } => "filesystem.write",
            HostRequest::ProcessSpawn { .. } => "process.spawn",
            HostRequest::NetworkConnect { .. } => "network.connect",
            HostRequest::DockerInspect { .. } => "docker.inspect",
            HostRequest::DockerExecute { .. } => "docker.execute",
        };
        Capability(name.to_string())
    }
}

/// The single choke point every plugin host-call passes through.
/// Section 23: "Default: DENY. Plugin asks for capability. Core/policy
/// decides: ALLOW / DENY / PROMPT." (v0.1 treats PROMPT as
/// `Decision::RequireApproval`, surfaced to the CLI caller.)
pub struct HostCapabilityGate<'a> {
    policy: &'a Policy,
}

impl<'a> HostCapabilityGate<'a> {
    pub fn new(policy: &'a Policy) -> Self {
        Self { policy }
    }

    pub fn check(&self, request: &HostRequest) -> Result<(), ForgeError> {
        match self.policy.capability_decision(&request.required_capability()) {
            Decision::Allow => Ok(()),
            Decision::Deny | Decision::RequireApproval => Err(ForgeError::CapabilityDenied(
                request.required_capability().0,
            )),
        }
    }
}

/// A plugin translates an external system into Forge primitives. It must
/// not redefine State/Change/Plan/Execution/Evidence (Section 22) — it
/// only ever returns `Resource`/`Observation`/`Relation` and executes
/// already-planned operations by `kind`.
pub trait Plugin {
    fn name(&self) -> &str;

    /// Discover current resources this plugin knows about.
    fn scan(&self) -> Result<Vec<Resource>, ForgeError>;

    /// Discover relations between resources this plugin knows about.
    fn discover_relations(&self, resources: &[Resource]) -> Result<Vec<Relation>, ForgeError>;

    /// Re-observe a single resource (used by Section 14 revalidation and
    /// Section 16 verification — both need a fresh, unopinionated read).
    fn observe(&self, resource_id: &str) -> Result<Vec<Observation>, ForgeError>;

    /// Execute one action `kind` (e.g. "docker.update_image") with the
    /// given parameters. Returns plugin-defined output for the
    /// `ActionResult.output` field.
    fn execute(&self, kind: &str, params: &Value) -> Result<Value, ForgeError>;

    /// Whether this plugin can compensate a given action kind, and how.
    /// Must be declared honestly — Section 17: never invent compensation
    /// that does not exist.
    fn compensation_support(&self, kind: &str) -> forge_core::CompensationSupport;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_denies_filesystem_write_request() {
        let policy = Policy::default();
        let gate = HostCapabilityGate::new(&policy);
        let result = gate.check(&HostRequest::FilesystemWrite {
            path: "/etc/passwd".into(),
        });
        assert!(result.is_err());
    }

    #[test]
    fn granted_capability_is_allowed() {
        let mut policy = Policy::default();
        policy
            .granted_capabilities
            .insert(Capability("filesystem.read".into()));
        let gate = HostCapabilityGate::new(&policy);
        let result = gate.check(&HostRequest::FilesystemRead {
            path: "/tmp/x".into(),
        });
        assert!(result.is_ok());
    }
}
