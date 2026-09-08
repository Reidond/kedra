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

#[cfg(test)]
mod tests {
    use super::*;
    use sysroot_core::noctalia::Theme;
    #[test]
    fn native_patch_preserves_private_unselected_data_and_comments() {
        let before = Settings {
            theme_mode: Theme::Dark,
            button_borders: true,
            input_borders: true,
        };
        let after = Settings {
            theme_mode: Theme::Light,
            button_borders: false,
            ..before
        };
        let source = b"# retained\n[theme]\nmode = 'dark' # retained theme comment\nprivate = 'synthetic-private-value'\n[shell]\nbutton_borders = true\ninput_borders = true # unchanged\n[other]\nvalue = 123\n";
        let patched = String::from_utf8(patch(source, before, after).unwrap()).unwrap();
        assert!(patched.contains("# retained theme comment"));
        assert!(patched.contains("private = 'synthetic-private-value'"));
        assert!(patched.contains("input_borders = true # unchanged\n[other]\nvalue = 123\n"));
        assert_eq!(
            sysroot_core::noctalia::project("5.0.1", &patched).unwrap(),
            after
        );
        assert_eq!(patch(source, before, before).unwrap(), source);
    }
    #[test]
    fn projected_plan_contains_no_native_extra_fields_and_refuses_stale_acceptance() {
        let settings = Settings {
            theme_mode: Theme::Dark,
            button_borders: true,
            input_borders: true,
        };
        let baseline = Baseline {
            target: "desktop".into(),
            source_path: "home/.config/noctalia/config.toml".into(),
            source_revision: "a".repeat(40),
            settings,
        };
        let mut state = State::new("b".repeat(32), baseline.clone(), settings).unwrap();
        let plan = Plan::new(
            &state,
            Action::Activate {
                baseline: Baseline {
                    source_revision: "c".repeat(40),
                    settings: Settings {
                        theme_mode: Theme::Light,
                        ..settings
                    },
                    ..baseline
                },
            },
        )
        .unwrap();
        assert_eq!(plan.id().unwrap().len(), 64);
        assert_eq!(
            plan.finish(&state, plan.desired)
                .unwrap()
                .accepted_baseline()
                .source_revision,
            "c".repeat(40)
        );
        state
            .capture(Settings {
                theme_mode: Theme::Auto,
                ..settings
            })
            .unwrap();
        assert!(plan.finish(&state, plan.desired).is_err());
        assert!(patch(b"[theme", settings, plan.desired).is_err());
    }
}
