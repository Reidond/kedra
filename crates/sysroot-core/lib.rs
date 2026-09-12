//! Pure release/home models and capability information.

/// Reviewed build/runtime compatibility, embedded from the shared repository contract.
pub mod compatibility;
/// Qualified bootc observations and non-mutating deployment journal transitions.
pub mod deployment;
/// Identity authenticated by an OCI image signature; independent of legacy release files.
pub mod image;
/// Narrow Noctalia effective-settings review and disposition transitions.
pub mod noctalia;
/// Signed release records, freshness checkpoints and replay validation.
pub mod release;

/// Human-facing operating system name.
pub const PROJECT: &str = "Kedra";
/// Static implementation inventory, not installed-machine or credential observations.
/// Schema 2 replaces the ambiguous bootstrap availability flags with scoped capabilities.
pub const STATUS_JSON: &str = r#"{
  "schema_version": 2,
  "project": "Kedra",
  "kind": "implementation_capabilities",
  "installed_state_checked": false,
  "capabilities": {
    "source": {
      "implemented": true, "platform": "portable", "operations": ["plan", "archive"],
      "requires": ["Git", "Kedra checkout"]
    },
    "release": {
      "implemented": true, "platform": "portable",
      "operations": ["key", "verify", "channel", "history", "unpack", "assemble"],
      "requires": ["public key and release inputs for the selected operation"]
    },
    "deployment": {
      "implemented": true, "platform": "linux", "operations": ["check", "enroll", "status", "stage", "rollback"],
      "requires": ["installed Kedra helper and trust", "administrator authorization", "enrollment for stage and rollback"]
    },
    "home": {
      "implemented": true, "platform": "linux",
      "noctalia_fields": ["theme.mode", "shell.button_borders", "shell.input_borders"],
      "niri_file": ".config/niri/config.kdl",
      "requires": ["ordinary user", "installed baselines", "explicit adoption for managed changes", "qualified native applications for activation", "source checkout for export and niri reconciliation"]
    },
    "agent_launchers": {
      "implemented": true, "platform": "linux", "agents": ["codex", "claude"],
      "requires": ["verified Kedra checkout", "selected official runtime", "official login for authenticated work"]
    },
    "doctor": {
      "implemented": true, "platform": "linux",
      "requires": ["installed Kedra desktop", "ordinary logged-in desktop user"]
    }
  },
  "qualification": {
    "complete": false,
    "scope": "recorded_project_evidence",
    "broader_home_groups": "incomplete",
    "codex_bundled_runtime": "native_vm_pass",
    "claude_bundled_runtime": "not_packaged_pending_owner_terms",
    "agent_authentication": "not_qualified"
  },
  "installed_checks": {
    "published_channel": "sysroot update check",
    "deployment": "sysroot update status",
    "caller_home": "sysroot update status --home",
    "desktop": "sysroot doctor"
  }
}"#;

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
