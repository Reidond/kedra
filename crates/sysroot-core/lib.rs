//! Pure release/home models and capability information.
//!
//! ```
//! assert_eq!(sysroot_core::PROJECT, "Kedra");
//! assert!(!sysroot_core::DEPLOYMENT_AVAILABLE);
//! ```

/// Qualified bootc observations and non-mutating deployment journal transitions.
pub mod deployment;
/// Narrow Noctalia effective-settings review and disposition transitions.
pub mod noctalia;
/// Signed release records, freshness checkpoints and replay validation.
pub mod release;

/// Human-facing operating system name.
pub const PROJECT: &str = "Kedra";
/// Bootstrap never claims operational OS management.
pub const DEPLOYMENT_AVAILABLE: bool = false;
/// Capability output; installed deployment status remains a separate future schema.
pub const STATUS_JSON: &str = r#"{"schema_version":1,"project":"Kedra","stage":"development","source_planning_available":true,"source_archive_available":true,"release_verification_available":true,"deployment_available":false,"home_management_available":false,"home_review_available":true,"home_review_platform":"linux","home_review_application":"noctalia","agent_launchers_available":true,"agent_launchers_platform":"linux","bundled_agents_qualified":false}"#;

/// Return the blocking research packet for a future command.
pub fn research_gate(command: &str) -> Option<&'static str> {
    match command {
        "update" | "deploy" | "rollback" => Some("R01, R02, R04, R08, R10"),
        "home" => Some("R03, R04"),
        "codex" | "claude" => Some("R05, R06, R11"),
        "setup" | "doctor" => Some("R06, R07, R10"),
        "context" => Some("R05, R09, R10"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_is_not_advertised() {
        assert!(STATUS_JSON.contains("\"deployment_available\":false"));
    }

    #[test]
    fn every_operational_command_has_a_gate() {
        for command in [
            "update", "deploy", "rollback", "home", "codex", "claude", "setup", "doctor", "context",
        ] {
            assert!(research_gate(command).is_some(), "missing gate: {command}");
        }
    }

    #[test]
    fn unknown_command_has_no_gate() {
        assert_eq!(research_gate("arbitrary-shell"), None);
    }
}
