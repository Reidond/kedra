//! Safe projected plans; native file/service execution lives in the Linux bridge.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sysroot_core::noctalia::{Baseline, Key, Settings, State, Value};
use toml_edit::{DocumentMut, Item, Table};

#[cfg(target_os = "linux")]
pub(super) mod linux;
pub(super) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Action {
    Activate { baseline: Baseline },
    Discard { key: Key },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Plan {
    pub schema_version: u32,
    pub expected_state_sha256: String,
    pub action: Action,
    pub observed: Settings,
    pub desired: Settings,
}
pub(super) fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
impl Plan {
    pub fn new(state: &State, action: Action) -> Result<Self> {
        let desired = match &action {
            Action::Activate { baseline } => state.prepare(baseline.clone())?.desired_live(),
            Action::Discard { key } => state.discarded_settings(*key)?,
        };
        Ok(Self {
            schema_version: 1,
            expected_state_sha256: hash(&state.to_bytes()?),
            action,
            observed: state.live_settings(),
            desired,
        })
    }
    pub fn id(&self) -> Result<String> {
        Ok(hash(&serde_json::to_vec(self)?))
    }
    pub fn finish(&self, before: &State, observed: Settings) -> Result<State> {
        if *self != Self::new(before, self.action.clone())? || observed != self.desired {
            return Err("activation plan or observed settings changed".into());
        }
        let mut after = before.clone();
        match &self.action {
            Action::Activate { baseline } => {
                let transition = after.prepare(baseline.clone())?;
                after.accept(transition, observed)?;
            }
            Action::Discard { .. } => {
                after.capture(observed)?;
            }
        }
        Ok(after)
    }
}

/// Modify only fields whose effective values must change. Other bytes stay in
/// the native app file and never enter a review record or source patch.
pub(super) fn patch(original: &[u8], observed: Settings, desired: Settings) -> Result<Vec<u8>> {
    let text = std::str::from_utf8(original).map_err(|_| "Noctalia settings are not UTF-8")?;
    let mut document: DocumentMut = text
        .parse()
        .map_err(|_| "Noctalia settings TOML is malformed")?;
    for (key, section, field) in [
        (Key::ThemeMode, "theme", "mode"),
        (Key::ButtonBorders, "shell", "button_borders"),
        (Key::InputBorders, "shell", "input_borders"),
    ] {
        if observed.get(key) == desired.get(key) {
            continue;
        }
        let table = document
            .as_table_mut()
            .entry(section)
            .or_insert(Item::Table(Table::new()))
            .as_table_like_mut()
            .ok_or("Noctalia setting section is not a table")?;
        let decoration = table
            .get(field)
            .and_then(Item::as_value)
            .map(|value| value.decor().clone());
        let mut value = match desired.get(key) {
            Value::Theme(theme) => toml_edit::Value::from(match theme {
                sysroot_core::noctalia::Theme::Dark => "dark",
                sysroot_core::noctalia::Theme::Light => "light",
                sysroot_core::noctalia::Theme::Auto => "auto",
            }),
            Value::Toggle(value) => toml_edit::Value::from(value),
        };
        if let Some(decoration) = decoration {
            *value.decor_mut() = decoration;
        }
        table.insert(field, Item::Value(value));
    }
    Ok(document.to_string().into_bytes())
}
