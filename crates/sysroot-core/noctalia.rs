//! A bounded Noctalia 5.0.1 projection and pure review state machine.
//!
//! Input is `noctalia config export full`, not a home/state directory walk.
//! Only three explicitly supported safe fields survive projection. Filesystem
//! activation, writer coordination and source Git operations are separate layers.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const APP_VERSION: &str = "5.0.1";
pub const MAX_EXPORT: usize = 1024 * 1024;
const MAX_STATE: usize = 65_536;
const KEYS: [Key; 3] = [Key::ThemeMode, Key::ButtonBorders, Key::InputBorders];

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    UnsupportedVersion,
    MalformedProjection,
    CorruptState,
    ScopeChanged,
    PolicyOverlap,
    NoChange,
    SourceChanged,
    Conflict(Key),
    StaleTransition,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::UnsupportedVersion => "Noctalia version is not qualified for this projection",
            Self::MalformedProjection => "Noctalia effective export is malformed or lacks supported fields",
            Self::CorruptState => "home review state is corrupt, noncanonical or an unsupported version; do not reset it",
            Self::ScopeChanged => "home target/source scope changed; explicit review is required",
            Self::PolicyOverlap => "selected and local-only/app-owned decisions overlap",
            Self::NoChange => "there is no new change to select or ignore",
            Self::SourceChanged => "source commit does not contain the selected snapshot",
            Self::Conflict(_) => "new baseline conflicts with local or pending decisions; live state is unchanged",
            Self::StaleTransition => "review or live values changed after preparation; prepare again",
        })
    }
}
impl std::error::Error for Error {}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Key {
    #[serde(rename = "theme.mode")]
    ThemeMode,
    #[serde(rename = "shell.button_borders")]
    ButtonBorders,
    #[serde(rename = "shell.input_borders")]
    InputBorders,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    Auto,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Value {
    Theme(Theme),
    Toggle(bool),
}
impl Value {
    fn matches(self, key: Key) -> bool {
        matches!(
            (key, self),
            (Key::ThemeMode, Self::Theme(_))
                | (Key::ButtonBorders | Key::InputBorders, Self::Toggle(_))
        )
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub theme_mode: Theme,
    pub button_borders: bool,
    pub input_borders: bool,
}
impl Settings {
    pub fn get(self, key: Key) -> Value {
        match key {
            Key::ThemeMode => Value::Theme(self.theme_mode),
            Key::ButtonBorders => Value::Toggle(self.button_borders),
            Key::InputBorders => Value::Toggle(self.input_borders),
        }
    }
    fn set(&mut self, key: Key, value: Value) -> Result<(), Error> {
        match (key, value) {
            (Key::ThemeMode, Value::Theme(value)) => self.theme_mode = value,
            (Key::ButtonBorders, Value::Toggle(value)) => self.button_borders = value,
            (Key::InputBorders, Value::Toggle(value)) => self.input_borders = value,
            _ => return Err(Error::CorruptState),
        }
        Ok(())
    }
}

/// Parse only the safe projection. Never retain raw export data or parser excerpts.
pub fn project(version: &str, export: &str) -> Result<Settings, Error> {
    if version != APP_VERSION {
        return Err(Error::UnsupportedVersion);
    }
    if export.len() > MAX_EXPORT {
        return Err(Error::MalformedProjection);
    }
    #[derive(Deserialize)]
    struct Export {
        theme: ThemeSection,
        shell: ShellSection,
    }
    #[derive(Deserialize)]
    struct ThemeSection {
        mode: Theme,
    }
    #[derive(Deserialize)]
    struct ShellSection {
        button_borders: bool,
        input_borders: bool,
    }
    // Other fields are intentionally discarded before any persistence or Git capture.
    let parsed: Export = toml::from_str(export).map_err(|_| Error::MalformedProjection)?;
    Ok(Settings {
        theme_mode: parsed.theme.mode,
        button_borders: parsed.shell.button_borders,
        input_borders: parsed.shell.input_borders,
    })
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Baseline {
    pub target: String,
    pub source_path: String,
    pub source_revision: String,
    pub settings: Settings,
}
fn revision(value: &str) -> bool {
    matches!(value.len(), 40 | 64)
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl Baseline {
    fn validate(&self) -> Result<(), Error> {
        if self.target.is_empty()
            || self.target.len() > 63
            || !self.target.as_bytes()[0].is_ascii_lowercase()
            || !self
                .target
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !revision(&self.source_revision)
            || !(self.source_path == "home/.config/noctalia/config.toml"
                || self.source_path
                    == format!("hosts/{}/home/.config/noctalia/config.toml", self.target))
        {
            return Err(Error::CorruptState);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Ignored {
    base: Value,
    value: Value,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Publication {
    value: Value,
    source_revision: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct State {
    schema_version: u32,
    app_version: String,
    instance: String,
    baseline: Baseline,
    live: Settings,
    selected: BTreeMap<Key, Value>,
    ignored: BTreeMap<Key, Ignored>,
    app_owned: BTreeSet<Key>,
    published: BTreeMap<Key, Vec<Publication>>,
}
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
pub struct Selection {
    pub key: Key,
    pub before: Value,
    pub after: Value,
}
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Row {
    pub key: Key,
    pub baseline: Value,
    pub live: Value,
    pub selected: Option<Value>,
    pub committed_pending: Option<Value>,
    pub local_only: bool,
    pub app_owned: bool,
    pub visible_change: bool,
}
#[derive(Debug)]
pub struct Transition {
    expected_state: String,
    next: State,
}
impl Transition {
    pub fn desired_live(&self) -> Settings {
        self.next.live
    }
    pub fn baseline(&self) -> &Baseline {
        &self.next.baseline
    }
}

impl State {
    /// Accepted image/source provenance; selection and publication do not advance it.
    pub fn accepted_baseline(&self) -> &Baseline {
        &self.baseline
    }

    /// Source history that must remain present before exporting or recording more edits.
    pub fn source_anchors(&self) -> Result<Vec<&str>, Error> {
        self.validate()?;
        let mut anchors = vec![self.baseline.source_revision.as_str()];
        for chain in self.published.values() {
            if let Some(publication) = chain.last() {
                anchors.push(publication.source_revision.as_str());
            }
        }
        Ok(anchors)
    }

    pub fn new(instance: String, baseline: Baseline, live: Settings) -> Result<Self, Error> {
        let state = Self {
            schema_version: 1,
            app_version: APP_VERSION.into(),
            instance,
            baseline,
            live,
            selected: BTreeMap::new(),
            ignored: BTreeMap::new(),
            app_owned: BTreeSet::new(),
            published: BTreeMap::new(),
        };
        state.validate()?;
        Ok(state)
    }
    fn review_base(&self, key: Key) -> Value {
        self.published
            .get(&key)
            .and_then(|chain| chain.last())
            .map_or(self.baseline.settings.get(key), |p| p.value)
    }
    fn validate(&self) -> Result<(), Error> {
        if self.schema_version != 1
            || self.instance.len() != 32
            || !self
                .instance
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(Error::CorruptState);
        }
        if self.app_version != APP_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        self.baseline.validate()?;
        for (&key, chain) in &self.published {
            if chain.is_empty() || chain.len() > 64 {
                return Err(Error::CorruptState);
            }
            let mut previous = self.baseline.settings.get(key);
            let mut commits = BTreeSet::new();
            for item in chain {
                if !item.value.matches(key)
                    || !revision(&item.source_revision)
                    || item.value == previous
                    || !commits.insert(&item.source_revision)
                {
                    return Err(Error::CorruptState);
                }
                previous = item.value;
            }
        }
        for (&key, &value) in &self.selected {
            if !value.matches(key) || value == self.review_base(key) {
                return Err(Error::CorruptState);
            }
            if self.ignored.contains_key(&key) || self.app_owned.contains(&key) {
                return Err(Error::PolicyOverlap);
            }
        }
        for (&key, policy) in &self.ignored {
            if self.app_owned.contains(&key) {
                return Err(Error::PolicyOverlap);
            }
            if policy.base != self.review_base(key)
                || !policy.value.matches(key)
                || policy.base == policy.value
                || self.live.get(key) != policy.value
            {
                return Err(Error::CorruptState);
            }
        }
        Ok(())
    }
    /// Canonical internal state rejects duplicate map keys and missing-field resets.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.validate()?;
        let mut bytes = serde_json::to_vec(self).map_err(|_| Error::CorruptState)?;
        bytes.push(b'\n');
        if bytes.len() > MAX_STATE {
            return Err(Error::CorruptState);
        }
        Ok(bytes)
    }
    pub fn from_bytes(bytes: &[u8], instance: &str) -> Result<Self, Error> {
        if bytes.len() > MAX_STATE {
            return Err(Error::CorruptState);
        }
        let state: Self = serde_json::from_slice(bytes).map_err(|_| Error::CorruptState)?;
        state.validate()?;
        if state.instance != instance {
            return Err(Error::ScopeChanged);
        }
        if state.to_bytes()? != bytes {
            return Err(Error::CorruptState);
        }
        Ok(state)
    }
    fn fingerprint(&self) -> Result<String, Error> {
        Ok(Sha256::digest(self.to_bytes()?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect())
    }
    pub fn capture(&mut self, live: Settings) -> Result<Vec<Key>, Error> {
        self.validate()?;
        let expired: Vec<_> = self
            .ignored
            .iter()
            .filter(|(key, policy)| live.get(**key) != policy.value)
            .map(|(key, _)| *key)
            .collect();
        for key in &expired {
            self.ignored.remove(key);
        }
        self.live = live;
        Ok(expired)
    }
    pub fn stage(&mut self, key: Key) -> Result<(), Error> {
        self.validate()?;
        if self.ignored.contains_key(&key) || self.app_owned.contains(&key) {
            return Err(Error::PolicyOverlap);
        }
        if self.live.get(key) == self.review_base(key) {
            return Err(Error::NoChange);
        }
        self.selected.insert(key, self.live.get(key));
        Ok(())
    }
    pub fn unstage(&mut self, key: Key) -> Result<(), Error> {
        self.validate()?;
        self.selected.remove(&key);
        Ok(())
    }
    pub fn ignore_exact(&mut self, key: Key) -> Result<(), Error> {
        self.validate()?;
        if self.selected.contains_key(&key) || self.app_owned.contains(&key) {
            return Err(Error::PolicyOverlap);
        }
        let base = self.review_base(key);
        let value = self.live.get(key);
        if base == value {
            return Err(Error::NoChange);
        }
        self.ignored.insert(key, Ignored { base, value });
        Ok(())
    }
    pub fn own(&mut self, key: Key) -> Result<(), Error> {
        self.validate()?;
        if self.selected.contains_key(&key) {
            return Err(Error::PolicyOverlap);
        }
        self.ignored.remove(&key);
        self.app_owned.insert(key);
        Ok(())
    }
    pub fn clear_local_policy(&mut self, key: Key) -> Result<(), Error> {
        self.validate()?;
        self.ignored.remove(&key);
        self.app_owned.remove(&key);
        Ok(())
    }
    pub fn selection(&self) -> Result<Vec<Selection>, Error> {
        self.validate()?;
        Ok(self
            .selected
            .iter()
            .map(|(&key, &after)| Selection {
                key,
                before: self.review_base(key),
                after,
            })
            .collect())
    }
    /// Caller must observe this commit in the separate source repository. No push
    /// or deployment is implied; source settings must contain the pinned selection.
    pub fn record_source_commit(&mut self, commit: &str, source: Settings) -> Result<(), Error> {
        self.validate()?;
        if !revision(commit) || self.selected.is_empty() {
            return Err(Error::SourceChanged);
        }
        if self
            .selected
            .iter()
            .any(|(&key, &value)| source.get(key) != value)
        {
            return Err(Error::SourceChanged);
        }
        let mut next = self.clone();
        for (&key, &value) in &self.selected {
            next.published.entry(key).or_default().push(Publication {
                value,
                source_revision: commit.into(),
            });
        }
        next.selected.clear();
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn rows(&self) -> Result<Vec<Row>, Error> {
        self.validate()?;
        Ok(KEYS
            .iter()
            .map(|&key| {
                let app_owned = self.app_owned.contains(&key);
                let local_only = self.ignored.contains_key(&key);
                let selected = self.selected.get(&key).copied();
                Row {
                    key,
                    baseline: self.baseline.settings.get(key),
                    live: self.live.get(key),
                    selected,
                    committed_pending: self
                        .published
                        .get(&key)
                        .and_then(|p| p.last())
                        .map(|p| p.value),
                    local_only,
                    app_owned,
                    visible_change: !app_owned
                        && !local_only
                        && self.live.get(key) != selected.unwrap_or(self.review_base(key)),
                }
            })
            .collect())
    }
    /// Plan without changing accepted baseline or live state. Published prefixes
    /// prevent a later GUI write from conflicting with an intermediate deployment.
    pub fn prepare(&self, baseline: Baseline) -> Result<Transition, Error> {
        self.validate()?;
        baseline.validate()?;
        if baseline.target != self.baseline.target
            || baseline.source_path != self.baseline.source_path
        {
            return Err(Error::ScopeChanged);
        }
        let mut next = self.clone();
        for key in KEYS {
            let old = self.baseline.settings.get(key);
            let new = baseline.settings.get(key);
            let live = self.live.get(key);
            let staged_adopted = self.selected.get(&key) == Some(&new);
            let prefix = self
                .published
                .get(&key)
                .and_then(|p| p.iter().rposition(|p| p.value == new));
            let known = new == old || staged_adopted || prefix.is_some();
            let owned = self.app_owned.contains(&key);
            if !owned
                && !known
                && (self.published.contains_key(&key)
                    || self.selected.get(&key).is_some_and(|value| *value != new)
                    || self.ignored.contains_key(&key) && live != new)
            {
                return Err(Error::Conflict(key));
            }
            let anchor = if staged_adopted || prefix.is_some() {
                new
            } else {
                old
            };
            let merged = if owned || new == anchor {
                live
            } else if live == anchor || live == new {
                new
            } else {
                return Err(Error::Conflict(key));
            };
            next.live.set(key, merged)?;
            if staged_adopted {
                next.selected.remove(&key);
                next.published.remove(&key);
            } else if let Some(prefix) = prefix {
                let chain = next.published.get_mut(&key).ok_or(Error::CorruptState)?;
                chain.drain(..=prefix);
                if chain.is_empty() {
                    next.published.remove(&key);
                }
            }
        }
        next.baseline = baseline;
        for key in KEYS {
            if let Some(policy) = next.ignored.get(&key)
                && policy.value == next.review_base(key)
            {
                next.ignored.remove(&key);
            }
        }
        next.validate()?;
        Ok(Transition {
            expected_state: self.fingerprint()?,
            next,
        })
    }
    /// Only call after the separate activator has coordinated writers, validated,
    /// durably applied and read back the desired settings. This method does no I/O.
    pub fn accept(&mut self, transition: Transition, observed: Settings) -> Result<(), Error> {
        if self.fingerprint()? != transition.expected_state || observed != transition.next.live {
            return Err(Error::StaleTransition);
        }
        *self = transition.next;
        Ok(())
    }
}
