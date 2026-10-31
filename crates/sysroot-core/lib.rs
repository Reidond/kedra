//! Pure release models shared by the CLI and the installed helper.

/// Reviewed build/runtime compatibility, embedded from the shared repository contract.
pub mod compatibility;
/// Qualified bootc observations and non-mutating stage preflight.
pub mod deployment;
/// Identity authenticated by an OCI image signature.
pub mod image;
/// Committed source layout.
pub mod layout;
/// Bounded requests shared by the ordinary CLI and the installed helper.
pub mod protocol;
/// Installed release scope and release-key identity.
pub mod release;
/// Closed target/architecture/repository table shared by every scope check.
pub mod targets;

/// Human-facing operating system name.
pub const PROJECT: &str = "Kedra";

/// Return the blocking research packet for a future command.
pub fn research_gate(command: &str) -> Option<&'static str> {
    match command {
        "update" | "deploy" | "rollback" => Some("R01, R02, R04, R08, R10"),
        "home" => Some("R03, R04"),
        "codex" | "claude" => Some("R05, R06, R11"),
        "context" => Some("R05, R09, R10"),
        _ => None,
    }
}
