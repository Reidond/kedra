//! Shared read-only bootstrap information, not a deployment protocol.
//!
//! ```
//! assert_eq!(sysroot_core::PROJECT, "Kedra");
//! assert!(!sysroot_core::DEPLOYMENT_AVAILABLE);
//! ```

/// Human-facing operating system name.
pub const PROJECT: &str = "Kedra";
/// Bootstrap never claims operational OS management.
pub const DEPLOYMENT_AVAILABLE: bool = false;
/// Stable bootstrap output; production status needs its own researched schema.
pub const STATUS_JSON: &str = r#"{"schema_version":1,"project":"Kedra","stage":"bootstrap","deployment_available":false,"home_management_available":false,"agent_launchers_available":false}"#;

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
