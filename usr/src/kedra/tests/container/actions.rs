//! Registered actions: native operations scenarios may call with validated
//! inputs and declared outputs.

use std::collections::BTreeMap;
use std::time::Duration;

use serde_json::Value;

use crate::docker::Exec;
use crate::scenario::Context;
use crate::{Result, invalid, parse_duration, session};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    TextList,
}

pub struct Spec {
    pub name: &'static str,
    pub summary: &'static str,
    /// (input, kind, required)
    pub inputs: &'static [(&'static str, Kind, bool)],
    pub outputs: &'static [&'static str],
    pub needs_session: bool,
}

pub const ACTIONS: &[Spec] = &[
    Spec {
        name: "screenshot",
        summary: "Save the whole niri output as <artifacts>/screenshots/<name>.png",
        inputs: &[("name", Kind::Text, true)],
        outputs: &["path"],
        needs_session: true,
    },
    Spec {
        name: "type_text",
        summary: "Type text (\\n for Return) into the focused window through niri's virtual keyboard (standard keymap)",
        inputs: &[("text", Kind::Text, true)],
        outputs: &[],
        needs_session: true,
    },
    Spec {
        name: "probe",
        summary: "Run a vetted guest probe from lab/probes as root, the account or its session (default)",
        inputs: &[
            ("name", Kind::Text, true),
            ("args", Kind::TextList, false),
            ("as", Kind::Text, false),
            ("timeout", Kind::Text, false),
        ],
        // `json` is present when stdout is a JSON document.
        outputs: &["stdout", "json"],
        needs_session: false,
    },
    Spec {
        name: "settle",
        summary: "Let an animation or redraw finish before a screenshot (at most 30s)",
        inputs: &[("for", Kind::Text, true)],
        outputs: &[],
        needs_session: false,
    },
];

pub fn spec(name: &str) -> Option<&'static Spec> {
    ACTIONS.iter().find(|spec| spec.name == name)
}

fn is_reference(value: &Value) -> bool {
    matches!(value, Value::String(text) if text.contains("${"))
}

fn probe_names() -> Vec<String> {
    std::fs::read_dir(crate::harness_dir().join("lab/probes"))
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| {
                    entry
                        .file_name()
                        .to_str()?
                        .strip_suffix(".py")
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default()
}

fn safe_name(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Static check: known inputs, required inputs present, literal values typed.
pub fn validate(spec: &Spec, inputs: &BTreeMap<String, Value>) -> Result<()> {
    for key in inputs.keys() {
        if !spec.inputs.iter().any(|(name, _, _)| name == key) {
            return Err(invalid(format!("action {} has no input {key}", spec.name)));
        }
    }
    for (name, kind, required) in spec.inputs {
        let Some(value) = inputs.get(*name) else {
            if *required {
                return Err(invalid(format!("action {} needs input {name}", spec.name)));
            }
            continue;
        };
        if is_reference(value) {
            continue;
        }
        let typed = match kind {
            Kind::Text => value.is_string(),
            Kind::TextList => value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string)),
        };
        if !typed {
            return Err(invalid(format!(
                "action {} input {name} must be {kind:?}",
                spec.name
            )));
        }
    }
    match spec.name {
        "screenshot" => match inputs.get("name") {
            Some(Value::String(name)) if !name.contains("${") && !safe_name(name) => Err(invalid(
                format!("screenshot name {name:?} must be [A-Za-z0-9_-]"),
            )),
            _ => Ok(()),
        },
        "probe" => {
            if let Some(Value::String(name)) = inputs.get("name")
                && !probe_names().contains(name)
            {
                return Err(invalid(format!("unknown probe {name:?}; see lab/probes")));
            }
            if let Some(Value::String(identity)) = inputs.get("as")
                && !["root", "user", "session"].contains(&identity.as_str())
            {
                return Err(invalid("probe `as` must be root, user or session"));
            }
            if let Some(Value::String(timeout)) = inputs.get("timeout") {
                parse_duration(timeout)?;
            }
            Ok(())
        }
        "settle" => match inputs.get("for") {
            Some(Value::String(text)) if !text.contains("${") => {
                if parse_duration(text)? > Duration::from_secs(30) {
                    return Err(invalid(
                        "settle is limited to 30s; observe state with `eventually` instead",
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}

fn text(inputs: &BTreeMap<String, Value>, name: &str) -> Result<String> {
    inputs
        .get(name)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| invalid(format!("input {name} must be text")))
}

fn session<'a>(context: &'a Context<'_>) -> Result<&'a session::Session> {
    context
        .session
        .as_ref()
        .ok_or_else(|| invalid("this action needs the desktop session"))
}

/// Execute a validated action within `remaining` of the scenario deadline.
pub fn run(
    context: &Context<'_>,
    name: &str,
    inputs: &BTreeMap<String, Value>,
    remaining: Duration,
) -> Result<BTreeMap<String, Value>> {
    let spec = spec(name).ok_or_else(|| invalid(format!("unknown action {name}")))?;
    validate(spec, inputs)?;
    let docker = context.docker;
    let environment = context.environment;
    let mut outputs = BTreeMap::new();
    match name {
        "screenshot" => {
            let label = text(inputs, "name")?;
            if !safe_name(&label) {
                return Err(invalid(format!(
                    "screenshot name {label:?} must be [A-Za-z0-9_-]"
                )));
            }
            let destination = context
                .artifacts
                .join("screenshots")
                .join(format!("{label}.png"));
            let path = session::screenshot(docker, environment, session(context)?, &destination)?;
            outputs.insert("path".into(), Value::String(path.display().to_string()));
        }
        "type_text" => {
            let exec =
                session(context)?.exec(["wlrctl", "keyboard", "type", &text(inputs, "text")?]);
            environment.run(
                docker,
                &exec.timeout(remaining.min(Duration::from_secs(60))),
            )?;
        }
        "probe" => {
            let probe = text(inputs, "name")?;
            if !probe_names().contains(&probe) {
                return Err(invalid(format!("unknown probe {probe:?}")));
            }
            let mut argv = vec![
                "python3".to_owned(),
                format!("/usr/libexec/kedra-lab/probes/{probe}.py"),
            ];
            if let Some(args) = inputs.get("args").and_then(Value::as_array) {
                argv.extend(args.iter().filter_map(Value::as_str).map(str::to_owned));
            }
            let identity = inputs
                .get("as")
                .and_then(Value::as_str)
                .unwrap_or("session");
            let exec: Exec = match identity {
                "root" => Exec::new(argv),
                "user" => context
                    .user
                    .as_ref()
                    .ok_or_else(|| invalid("probe needs the test_user fixture"))?
                    .exec(argv),
                _ => session(context)?.exec(argv),
            };
            let limit = match inputs.get("timeout").and_then(Value::as_str) {
                Some(value) => parse_duration(value)?,
                None => Duration::from_secs(600),
            };
            let output = environment.run(docker, &exec.timeout(limit.min(remaining)))?;
            if let Ok(document) = serde_json::from_slice::<Value>(&output.stdout) {
                outputs.insert("json".into(), document);
            }
            outputs.insert("stdout".into(), Value::String(output.stdout_text()));
        }
        "settle" => {
            let pause = parse_duration(&text(inputs, "for")?)?.min(Duration::from_secs(30));
            std::thread::sleep(pause.min(remaining));
        }
        _ => return Err(invalid(format!("action {name} has no implementation"))),
    }
    Ok(outputs)
}
