//! Versioned scenario documents: strict parsing, static validation and execution.
//!
//! A scenario is data, not a program. Each step is exactly one operation:
//! `exec` runs one command (an explicit argv, never a shell or interpreter)
//! and asserts on its result; `eventually` repeats a read-only observation
//! until it passes or its deadline expires; `assert` compares known values;
//! `action` calls a registered native action; `include` runs a reusable setup
//! with declared inputs and outputs. Anything needing control flow belongs in
//! a native test (native.rs) or a vetted guest probe (lab/probes).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value;

use crate::actions;
use crate::docker::{Docker, Exec, Output};
use crate::environment::Environment;
use crate::session::{Session, TestUser};
use crate::{Error, Result, clip, invalid, parse_duration};

// ---------------------------------------------------------------------------
// Document model. Unknown fields and duplicate keys are rejected on parse.

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub profile: Profile,
    /// Image targets this scenario applies to; all when omitted.
    #[serde(default)]
    pub targets: Vec<String>,
    /// Whole-scenario deadline, e.g. `5m`.
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub fixtures: Vec<Fixture>,
    pub steps: Vec<Step>,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    /// Booted systemd; no graphical session.
    System,
    /// Booted systemd plus the real Kedra session for the test account.
    Desktop,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Fixture {
    /// Disposable wheel account created from /etc/skel.
    TestUser,
    /// The account's running niri/Noctalia session (implies test_user).
    DesktopSession,
}

impl Fixture {
    fn exports(self) -> &'static [&'static str] {
        match self {
            Fixture::TestUser => &["name", "uid", "home"],
            Fixture::DesktopSession => &["wayland_display", "niri_socket", "output"],
        }
    }

    fn key(self) -> &'static str {
        match self {
            Fixture::TestUser => "test_user",
            Fixture::DesktopSession => "desktop_session",
        }
    }
}

/// Reusable steps with explicit inputs and outputs (setups/*.yaml).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub inputs: Vec<String>,
    /// Output name -> local variable exported to the caller.
    #[serde(default)]
    pub outputs: BTreeMap<String, String>,
    pub steps: Vec<Step>,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Step {
    Exec(Command),
    Eventually(Eventually),
    Assert(Vec<Comparison>),
    Action(ActionCall),
    Include(Include),
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Identity {
    #[default]
    Root,
    /// The test account without display variables.
    User,
    /// The test account inside its graphical session.
    Session,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Command {
    /// Optional label shown in reports.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, rename = "as")]
    pub identity: Identity,
    pub argv: Vec<String>,
    /// Extra environment variables for this command.
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub timeout: Option<String>,
    #[serde(default)]
    pub expect: Expect,
    /// Variable name -> what to capture from the result.
    #[serde(default)]
    pub capture: BTreeMap<String, Capture>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// Exit status; 0 when omitted.
    #[serde(default)]
    pub exit: Option<i64>,
    /// Exact stdout after trimming one trailing newline.
    #[serde(default)]
    pub stdout: Option<String>,
    /// `true`: stdout must not be empty; `false`: it must be empty.
    #[serde(default)]
    pub stdout_nonempty: Option<bool>,
    #[serde(default)]
    pub stdout_contains: Vec<String>,
    #[serde(default)]
    pub stdout_lacks: Vec<String>,
    #[serde(default)]
    pub stderr_contains: Vec<String>,
    #[serde(default)]
    pub stderr_lacks: Vec<String>,
    /// Assertions on stdout parsed as JSON.
    #[serde(default)]
    pub json: Vec<JsonCheck>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonCheck {
    pub select: String,
    #[serde(default)]
    pub equals: Option<Value>,
    #[serde(default)]
    pub subset: Option<Value>,
    /// The selected path must not exist (distinct from `equals: null`).
    #[serde(default)]
    pub absent: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub enum Capture {
    /// `trim` (text without surrounding whitespace) or `raw`.
    Stdout(StdoutCapture),
    /// A selector into stdout parsed as JSON; the value keeps its type.
    Json(String),
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StdoutCapture {
    Trim,
    Raw,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Eventually {
    #[serde(default)]
    pub name: Option<String>,
    pub timeout: String,
    #[serde(default)]
    pub interval: Option<String>,
    /// A read-only observation; it is repeated, so it must not change state.
    pub observe: Command,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub value: Value,
    #[serde(default)]
    pub equals: Option<Value>,
    #[serde(default)]
    pub differs: Option<Value>,
    #[serde(default)]
    pub one_of: Option<Vec<Value>>,
    /// `value` must be text containing this text.
    #[serde(default)]
    pub contains: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCall {
    pub name: String,
    #[serde(default, rename = "with")]
    pub inputs: BTreeMap<String, Value>,
    /// Action output -> local variable.
    #[serde(default)]
    pub capture: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Include {
    pub setup: String,
    #[serde(default, rename = "with")]
    pub inputs: BTreeMap<String, Value>,
    /// Setup output -> local variable.
    #[serde(default)]
    pub capture: BTreeMap<String, String>,
}

// ---------------------------------------------------------------------------
// Loading and static validation, before any container exists.

fn yaml_options() -> serde_saphyr::Options {
    let mut options = serde_saphyr::Options::default();
    options.duplicate_keys = serde_saphyr::DuplicateKeyPolicy::Error;
    options.merge_keys = serde_saphyr::MergeKeyPolicy::Error;
    options.strict_booleans = true;
    options.reject_unsupported_tags = true;
    options
}

fn read_documents<T: for<'de> Deserialize<'de>>(directory: &Path) -> Result<Vec<(PathBuf, T)>> {
    let mut paths: Vec<PathBuf> = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "yaml"))
            .collect(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    paths.sort();
    let mut errors = Vec::new();
    let mut documents = Vec::new();
    for path in paths {
        let text = fs::read_to_string(&path)?;
        match serde_saphyr::from_str_with_options::<T>(&text, yaml_options()) {
            Ok(document) => documents.push((path, document)),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    if errors.is_empty() {
        Ok(documents)
    } else {
        Err(invalid(errors.join("\n")))
    }
}

/// All scenarios and setups, parsed and validated together.
pub struct Suite {
    pub scenarios: Vec<Scenario>,
    pub setups: BTreeMap<String, Setup>,
}

const INTERPRETERS: &[&str] = &[
    "sh", "bash", "dash", "zsh", "fish", "ksh", "csh", "tcsh", "python", "python3", "perl", "ruby",
    "node", "lua", "env", "eval", "exec", "xargs",
];

fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        && !text.as_bytes()[0].is_ascii_digit()
}

/// A bounded JSON selector: `$`, then `.key` or `[index]` segments.
#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    Key(String),
    Index(usize),
}

pub fn parse_selector(text: &str) -> Result<Vec<Segment>> {
    let rest = text
        .strip_prefix('$')
        .ok_or_else(|| invalid(format!("selector {text:?} must start with $")))?;
    let mut segments = Vec::new();
    let mut rest = rest;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let end = after.find(['.', '[']).unwrap_or(after.len());
            let key = &after[..end];
            if key.is_empty()
                || !key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
            {
                return Err(invalid(format!("unsupported key in selector {text:?}")));
            }
            segments.push(Segment::Key(key.to_owned()));
            rest = &after[end..];
        } else if let Some(after) = rest.strip_prefix('[') {
            let end = after
                .find(']')
                .ok_or_else(|| invalid(format!("unclosed index in selector {text:?}")))?;
            let index = after[..end]
                .parse::<usize>()
                .map_err(|_| invalid(format!("only numeric indexes are supported in {text:?}")))?;
            segments.push(Segment::Index(index));
            rest = &after[end + 1..];
        } else {
            return Err(invalid(format!("unsupported selector syntax in {text:?}")));
        }
        if segments.len() > 16 {
            return Err(invalid(format!("selector {text:?} is too deep")));
        }
    }
    Ok(segments)
}

fn select<'a>(value: &'a Value, segments: &[Segment]) -> Option<&'a Value> {
    segments
        .iter()
        .try_fold(value, |current, segment| match segment {
            Segment::Key(key) => current.as_object()?.get(key),
            Segment::Index(index) => current.as_array()?.get(*index),
        })
}

/// `${...}` references in a string, in order.
fn references(text: &str) -> Result<Vec<String>> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let end = after
            .find('}')
            .ok_or_else(|| invalid(format!("unclosed reference in {text:?}")))?;
        found.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    Ok(found)
}

fn value_references(value: &Value, into: &mut Vec<String>) -> Result<()> {
    match value {
        Value::String(text) => into.extend(references(text)?),
        Value::Array(items) => {
            for item in items {
                value_references(item, into)?;
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                value_references(item, into)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Names visible while validating one step list.
struct Scope<'a> {
    where_: String,
    variables: BTreeSet<String>,
    fixtures: BTreeSet<Fixture>,
    inputs: BTreeSet<String>,
    setups: &'a BTreeMap<String, Setup>,
    profile: Option<Profile>,
}

impl Scope<'_> {
    fn check_reference(&self, reference: &str) -> Result<()> {
        if let Some(rest) = reference.strip_prefix("fixtures.") {
            let (fixture, field) = rest.split_once('.').ok_or_else(|| {
                invalid(format!(
                    "{}: reference ${{{reference}}} needs a field",
                    self.where_
                ))
            })?;
            let known = self
                .fixtures
                .iter()
                .find(|f| f.key() == fixture)
                .ok_or_else(|| {
                    invalid(format!(
                        "{}: ${{{reference}}} uses an undeclared fixture",
                        self.where_
                    ))
                })?;
            if !known.exports().contains(&field) {
                return Err(invalid(format!(
                    "{}: fixture {fixture} exports {:?}, not {field}",
                    self.where_,
                    known.exports()
                )));
            }
            return Ok(());
        }
        if let Some(input) = reference.strip_prefix("inputs.") {
            if self.inputs.contains(input) {
                return Ok(());
            }
            return Err(invalid(format!(
                "{}: undeclared input ${{{reference}}}",
                self.where_
            )));
        }
        if self.variables.contains(reference) {
            return Ok(());
        }
        Err(invalid(format!(
            "{}: ${{{reference}}} is not captured by an earlier step",
            self.where_
        )))
    }

    fn check_text(&self, text: &str) -> Result<()> {
        for reference in references(text)? {
            self.check_reference(&reference)?;
        }
        Ok(())
    }

    fn check_value(&self, value: &Value) -> Result<()> {
        let mut found = Vec::new();
        value_references(value, &mut found)?;
        for reference in found {
            self.check_reference(&reference)?;
        }
        Ok(())
    }

    fn declare(&mut self, name: &str) -> Result<()> {
        if !is_identifier(name) {
            return Err(invalid(format!(
                "{}: invalid variable name {name:?}",
                self.where_
            )));
        }
        if !self.variables.insert(name.to_owned()) {
            return Err(invalid(format!(
                "{}: variable {name} is captured twice",
                self.where_
            )));
        }
        Ok(())
    }

    fn check_command(&mut self, command: &Command) -> Result<()> {
        if command.argv.is_empty() {
            return Err(invalid(format!("{}: argv is empty", self.where_)));
        }
        for argument in &command.argv {
            let base = argument.rsplit('/').next().unwrap_or(argument);
            if INTERPRETERS.contains(&base) {
                return Err(invalid(format!(
                    "{}: {argument:?} runs a shell or interpreter; use an action, probe or native test",
                    self.where_
                )));
            }
            self.check_text(argument)?;
        }
        if command.identity == Identity::Session && self.profile == Some(Profile::System) {
            return Err(invalid(format!(
                "{}: `as: session` needs the desktop profile",
                self.where_
            )));
        }
        if command.identity != Identity::Root && !self.fixtures.contains(&Fixture::TestUser) {
            return Err(invalid(format!(
                "{}: `as: {:?}` needs the test_user fixture",
                self.where_, command.identity
            )));
        }
        for (name, value) in &command.env {
            if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(invalid(format!(
                    "{}: invalid environment name {name:?}",
                    self.where_
                )));
            }
            self.check_text(value)?;
        }
        if let Some(stdin) = &command.stdin {
            self.check_text(stdin)?;
        }
        if let Some(timeout) = &command.timeout {
            parse_duration(timeout)?;
        }
        let expect = &command.expect;
        for text in expect
            .stdout
            .iter()
            .chain(&expect.stdout_contains)
            .chain(&expect.stdout_lacks)
            .chain(&expect.stderr_contains)
            .chain(&expect.stderr_lacks)
        {
            self.check_text(text)?;
        }
        for check in &expect.json {
            parse_selector(&check.select)
                .map_err(|error| invalid(format!("{}: {error}", self.where_)))?;
            let modes = usize::from(check.equals.is_some())
                + usize::from(check.subset.is_some())
                + usize::from(check.absent);
            if modes != 1 {
                return Err(invalid(format!(
                    "{}: json check {:?} needs exactly one of equals, subset or absent",
                    self.where_, check.select
                )));
            }
            for value in check.equals.iter().chain(&check.subset) {
                self.check_value(value)?;
            }
        }
        for (name, capture) in &command.capture {
            if let Capture::Json(selector) = capture {
                parse_selector(selector)
                    .map_err(|error| invalid(format!("{}: {error}", self.where_)))?;
            }
            self.declare(name)?;
        }
        Ok(())
    }

    fn check_steps(&mut self, steps: &[Step], stack: &mut Vec<String>) -> Result<()> {
        if steps.is_empty() {
            return Err(invalid(format!("{}: no steps", self.where_)));
        }
        let base = self.where_.clone();
        for (index, step) in steps.iter().enumerate() {
            self.where_ = format!("{base} step {}", index + 1);
            match step {
                Step::Exec(command) => self.check_command(command)?,
                Step::Eventually(eventually) => self.check_eventually(eventually)?,
                Step::Assert(comparisons) => self.check_assert(comparisons)?,
                Step::Action(call) => self.check_action(call)?,
                Step::Include(include) => self.check_include(include, stack)?,
            }
        }
        self.where_ = base;
        Ok(())
    }

    fn check_eventually(&mut self, eventually: &Eventually) -> Result<()> {
        parse_duration(&eventually.timeout)?;
        if let Some(interval) = &eventually.interval {
            parse_duration(interval)?;
        }
        self.check_command(&eventually.observe)
    }

    fn check_assert(&self, comparisons: &[Comparison]) -> Result<()> {
        if comparisons.is_empty() {
            return Err(invalid(format!("{}: empty assert", self.where_)));
        }
        for comparison in comparisons {
            let modes = usize::from(comparison.equals.is_some())
                + usize::from(comparison.differs.is_some())
                + usize::from(comparison.one_of.is_some())
                + usize::from(comparison.contains.is_some());
            if modes != 1 {
                return Err(invalid(format!(
                    "{}: assert needs exactly one of equals, differs, one_of or contains",
                    self.where_
                )));
            }
            if let Some(text) = &comparison.contains {
                self.check_text(text)?;
            }
            self.check_value(&comparison.value)?;
            for value in comparison
                .equals
                .iter()
                .chain(&comparison.differs)
                .chain(comparison.one_of.iter().flatten())
            {
                self.check_value(value)?;
            }
        }
        Ok(())
    }

    fn check_action(&mut self, call: &ActionCall) -> Result<()> {
        let spec = actions::spec(&call.name)
            .ok_or_else(|| invalid(format!("{}: unknown action {:?}", self.where_, call.name)))?;
        if spec.needs_session && self.profile == Some(Profile::System) {
            return Err(invalid(format!(
                "{}: action {} needs the desktop profile",
                self.where_, call.name
            )));
        }
        actions::validate(spec, &call.inputs)
            .map_err(|error| invalid(format!("{}: {error}", self.where_)))?;
        for value in call.inputs.values() {
            self.check_value(value)?;
        }
        for (output, local) in &call.capture {
            if !spec.outputs.contains(&output.as_str()) {
                return Err(invalid(format!(
                    "{}: action {} has no output {output}",
                    self.where_, call.name
                )));
            }
            self.declare(local)?;
        }
        Ok(())
    }

    fn check_include(&mut self, include: &Include, stack: &mut Vec<String>) -> Result<()> {
        let setup = self.setups.get(&include.setup).ok_or_else(|| {
            invalid(format!(
                "{}: unknown setup {:?}",
                self.where_, include.setup
            ))
        })?;
        if stack.contains(&include.setup) {
            return Err(invalid(format!(
                "{}: recursive include {} -> {}",
                self.where_,
                stack.join(" -> "),
                include.setup
            )));
        }
        let given: BTreeSet<&String> = include.inputs.keys().collect();
        let declared: BTreeSet<&String> = setup.inputs.iter().collect();
        if given != declared {
            return Err(invalid(format!(
                "{}: setup {} takes inputs {:?}, got {:?}",
                self.where_, include.setup, declared, given
            )));
        }
        for value in include.inputs.values() {
            self.check_value(value)?;
        }
        for (output, local) in &include.capture {
            if !setup.outputs.contains_key(output) {
                return Err(invalid(format!(
                    "{}: setup {} has no output {output}",
                    self.where_, include.setup
                )));
            }
            self.declare(local)?;
        }
        self.check_setup_body(include, setup, stack)
    }

    /// Validate the setup body in its own scope with this caller's fixtures.
    fn check_setup_body(
        &self,
        include: &Include,
        setup: &Setup,
        stack: &mut Vec<String>,
    ) -> Result<()> {
        stack.push(include.setup.clone());
        let mut inner = Scope {
            where_: format!("{} (setup {})", self.where_, include.setup),
            variables: BTreeSet::new(),
            fixtures: self.fixtures.clone(),
            inputs: setup.inputs.iter().cloned().collect(),
            setups: self.setups,
            profile: self.profile,
        };
        inner.check_steps(&setup.steps, stack)?;
        for local in setup.outputs.values() {
            if !inner.variables.contains(local) {
                return Err(invalid(format!(
                    "{}: setup {} exports {local}, which it never captures",
                    inner.where_, include.setup
                )));
            }
        }
        stack.pop();
        Ok(())
    }
}

impl Scenario {
    /// Effective fixtures: the desktop profile always has an account and session.
    pub fn effective_fixtures(&self) -> BTreeSet<Fixture> {
        let mut fixtures: BTreeSet<Fixture> = self.fixtures.iter().copied().collect();
        if self.profile == Profile::Desktop {
            fixtures.insert(Fixture::TestUser);
            fixtures.insert(Fixture::DesktopSession);
        }
        if fixtures.contains(&Fixture::DesktopSession) {
            fixtures.insert(Fixture::TestUser);
        }
        fixtures
    }

    pub fn deadline(&self) -> Duration {
        self.timeout
            .as_deref()
            .and_then(|text| parse_duration(text).ok())
            .unwrap_or(Duration::from_secs(600))
    }

    pub fn applies_to(&self, target: &str) -> bool {
        self.targets.is_empty() || self.targets.iter().any(|item| item == target)
    }
}

/// Parse and validate every scenario and setup under `root`.
pub fn load(root: &Path, known_targets: &[String]) -> Result<Suite> {
    let setups_list: Vec<(PathBuf, Setup)> = read_documents(&root.join("setups"))?;
    let scenario_list: Vec<(PathBuf, Scenario)> = read_documents(&root.join("scenarios"))?;
    let mut errors = Vec::new();
    let mut setups = BTreeMap::new();
    for (path, mut setup) in setups_list {
        setup.path = path.clone();
        check_setup_header(&path, &setup, &mut errors);
        if let Some(previous) = setups.insert(setup.name.clone(), setup) {
            errors.push(format!(
                "{}: duplicate setup name {}",
                path.display(),
                previous.name
            ));
        }
    }
    let mut names = BTreeSet::new();
    let mut scenarios = Vec::new();
    for (path, mut scenario) in scenario_list {
        scenario.path = path.clone();
        let display = path.display().to_string();
        check_scenario_header(&display, &scenario, known_targets, &mut names, &mut errors);
        let mut scope = Scope {
            where_: display.clone(),
            variables: BTreeSet::new(),
            fixtures: scenario.effective_fixtures(),
            inputs: BTreeSet::new(),
            setups: &setups,
            profile: Some(scenario.profile),
        };
        if let Err(error) = scope.check_steps(&scenario.steps, &mut Vec::new()) {
            errors.push(error.to_string());
        }
        scenarios.push(scenario);
    }
    // Setups that no scenario includes still get validated on their own.
    for setup in setups.values() {
        let mut scope = Scope {
            where_: setup.path.display().to_string(),
            variables: BTreeSet::new(),
            fixtures: [Fixture::TestUser, Fixture::DesktopSession]
                .into_iter()
                .collect(),
            inputs: setup.inputs.iter().cloned().collect(),
            setups: &setups,
            profile: None,
        };
        if let Err(error) = scope.check_steps(&setup.steps, &mut vec![setup.name.clone()]) {
            errors.push(error.to_string());
        }
    }
    if errors.is_empty() {
        Ok(Suite { scenarios, setups })
    } else {
        Err(invalid(errors.join("\n")))
    }
}

/// A setup's version, name and input names; its steps are checked separately.
fn check_setup_header(path: &Path, setup: &Setup, errors: &mut Vec<String>) {
    if setup.version != 1 {
        errors.push(format!(
            "{}: unsupported version {}",
            path.display(),
            setup.version
        ));
    }
    if !is_identifier(&setup.name) {
        errors.push(format!(
            "{}: invalid setup name {:?}",
            path.display(),
            setup.name
        ));
    }
    let mut seen = BTreeSet::new();
    for input in &setup.inputs {
        if !is_identifier(input) || !seen.insert(input) {
            errors.push(format!(
                "{}: invalid or duplicate input {input:?}",
                path.display()
            ));
        }
    }
}

/// Everything in a scenario except its steps; `names` collects the names seen so far.
fn check_scenario_header(
    display: &str,
    scenario: &Scenario,
    known_targets: &[String],
    names: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    if scenario.version != 1 {
        errors.push(format!(
            "{display}: unsupported version {}",
            scenario.version
        ));
    }
    if !is_identifier(&scenario.name) {
        errors.push(format!(
            "{display}: invalid scenario name {:?}",
            scenario.name
        ));
    }
    if !names.insert(scenario.name.clone()) {
        errors.push(format!(
            "{display}: duplicate scenario name {}",
            scenario.name
        ));
    }
    for target in &scenario.targets {
        if !known_targets.contains(target) {
            errors.push(format!("{display}: unknown target {target:?}"));
        }
    }
    if let Some(timeout) = &scenario.timeout
        && let Err(error) = parse_duration(timeout)
    {
        errors.push(format!("{display}: {error}"));
    }
    let mut seen = BTreeSet::new();
    for fixture in &scenario.fixtures {
        if !seen.insert(*fixture) {
            errors.push(format!("{display}: duplicate fixture {}", fixture.key()));
        }
    }
    if scenario.profile == Profile::System && scenario.fixtures.contains(&Fixture::DesktopSession) {
        errors.push(format!("{display}: desktop_session needs profile: desktop"));
    }
}

// ---------------------------------------------------------------------------
// Execution.

/// Everything one scenario or native test runs against.
pub struct Context<'a> {
    pub docker: &'a Docker,
    pub environment: &'a Environment,
    pub user: Option<TestUser>,
    pub session: Option<Session>,
    pub artifacts: PathBuf,
    pub target: String,
    /// Independently selected composition identity for source-specific native cases.
    pub composition_identity: Option<String>,
    pub derivation_identity: Option<String>,
}

impl Context<'_> {
    /// A command as `identity`, with the session environment where needed.
    pub fn command(&self, identity: Identity, argv: Vec<String>) -> Result<Exec> {
        Ok(match identity {
            Identity::Root => Exec::new(argv),
            Identity::User => self
                .user
                .as_ref()
                .ok_or_else(|| invalid("no test account in this environment"))?
                .exec(argv),
            Identity::Session => self
                .session
                .as_ref()
                .ok_or_else(|| invalid("no desktop session in this environment"))?
                .exec(argv),
        })
    }

    pub fn fixture_value(&self, fixture: &str, field: &str) -> Result<Value> {
        let missing = || invalid(format!("fixture {fixture}.{field} is not available"));
        match (fixture, field) {
            ("test_user", "name") => Ok(Value::String(
                self.user.as_ref().ok_or_else(missing)?.name.clone(),
            )),
            ("test_user", "uid") => Ok(Value::from(self.user.as_ref().ok_or_else(missing)?.uid)),
            ("test_user", "home") => Ok(Value::String(
                self.user.as_ref().ok_or_else(missing)?.home.clone(),
            )),
            ("desktop_session", "wayland_display") => Ok(Value::String(
                self.session
                    .as_ref()
                    .ok_or_else(missing)?
                    .wayland_display
                    .clone(),
            )),
            ("desktop_session", "niri_socket") => Ok(Value::String(
                self.session
                    .as_ref()
                    .ok_or_else(missing)?
                    .niri_socket
                    .clone(),
            )),
            ("desktop_session", "output") => Ok(Value::String("winit".into())),
            _ => Err(missing()),
        }
    }
}

/// One executed step, for reports.
#[derive(Clone, Debug, serde::Serialize)]
pub struct StepRecord {
    pub location: String,
    pub operation: &'static str,
    pub label: Option<String>,
    pub duration_ms: u128,
    pub passed: bool,
    pub detail: String,
}

/// A step failure with its location and the last observed result.
#[derive(Debug)]
pub struct StepFailure {
    pub location: String,
    pub message: String,
}

impl std::fmt::Display for StepFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

struct Frame {
    variables: BTreeMap<String, Value>,
    inputs: BTreeMap<String, Value>,
}

/// One attempt of an `eventually` observation.
enum Observation {
    /// The expectation held before the deadline.
    Passed(Output),
    /// Not satisfied yet; the last observed result.
    Pending(String),
    /// A permanent error or a completion after the deadline.
    Failed(String),
}

pub struct Runner<'a, 'b> {
    pub context: &'a Context<'b>,
    pub setups: &'a BTreeMap<String, Setup>,
    pub records: Vec<StepRecord>,
    pub deadline: Instant,
}

fn render(value: &Value) -> Result<String> {
    Ok(match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        other => return Err(invalid(format!("cannot interpolate {other} into text"))),
    })
}

fn describe(output: &Output) -> String {
    format!(
        "exit {} after {} ms; stdout: {}; stderr: {}",
        output.exit,
        output.duration.as_millis(),
        clip(output.stdout_text().trim(), 1500),
        clip(output.stderr_text().trim(), 1500)
    )
}

/// Subset semantics: every key of `expected` exists in `actual` with a
/// subset-matching value; arrays match element-wise with equal length.
fn is_subset(expected: &Value, actual: &Value) -> bool {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| is_subset(value, found))),
        (Value::Array(expected), Value::Array(actual)) => {
            expected.len() == actual.len()
                && expected
                    .iter()
                    .zip(actual)
                    .all(|(left, right)| is_subset(left, right))
        }
        _ => expected == actual,
    }
}

impl Runner<'_, '_> {
    fn lookup(&self, frame: &Frame, reference: &str) -> Result<Value> {
        if let Some(rest) = reference.strip_prefix("fixtures.") {
            let (fixture, field) = rest.split_once('.').unwrap_or((rest, ""));
            return self.context.fixture_value(fixture, field);
        }
        if let Some(input) = reference.strip_prefix("inputs.") {
            return frame
                .inputs
                .get(input)
                .cloned()
                .ok_or_else(|| invalid(format!("input {input} is missing")));
        }
        frame
            .variables
            .get(reference)
            .cloned()
            .ok_or_else(|| invalid(format!("variable {reference} was not captured")))
    }

    /// Interpolate references into text; missing references fail.
    fn text(&self, frame: &Frame, text: &str) -> Result<String> {
        let mut result = String::new();
        let mut rest = text;
        while let Some(start) = rest.find("${") {
            result.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let end = after
                .find('}')
                .ok_or_else(|| invalid("unclosed reference"))?;
            result.push_str(&render(&self.lookup(frame, &after[..end])?)?);
            rest = &after[end + 1..];
        }
        result.push_str(rest);
        Ok(result)
    }

    /// Resolve a YAML value: a string that is exactly one reference keeps the
    /// referenced value's type; other strings interpolate as text.
    fn value(&self, frame: &Frame, value: &Value) -> Result<Value> {
        Ok(match value {
            Value::String(text) => {
                let refs = references(text)?;
                if refs.len() == 1 && text == &format!("${{{}}}", refs[0]) {
                    self.lookup(frame, &refs[0])?
                } else {
                    Value::String(self.text(frame, text)?)
                }
            }
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|item| self.value(frame, item))
                    .collect::<Result<_>>()?,
            ),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, item)| Ok((key.clone(), self.value(frame, item)?)))
                    .collect::<Result<_>>()?,
            ),
            other => other.clone(),
        })
    }

    fn remaining(&self) -> Result<Duration> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(Error::Timeout {
                what: "scenario".into(),
                after: Duration::ZERO,
            });
        }
        Ok(left)
    }

    fn run_command(
        &self,
        frame: &Frame,
        command: &Command,
        budget: Option<Duration>,
    ) -> Result<Output> {
        if budget.is_some_and(|left| left.is_zero()) {
            return Err(Error::Timeout {
                what: "observation".into(),
                after: Duration::ZERO,
            });
        }
        let argv = command
            .argv
            .iter()
            .map(|argument| self.text(frame, argument))
            .collect::<Result<Vec<_>>>()?;
        let mut exec = self.context.command(command.identity, argv)?;
        for (name, value) in &command.env {
            let value = self.text(frame, value)?;
            exec.env.retain(|(existing, _)| existing != name);
            exec.env.push((name.clone(), value));
        }
        if let Some(stdin) = &command.stdin {
            exec.stdin = Some(self.text(frame, stdin)?.into_bytes());
        }
        let limit = command
            .timeout
            .as_deref()
            .map(parse_duration)
            .transpose()?
            .unwrap_or(Duration::from_secs(60))
            .min(self.remaining()?)
            .min(budget.unwrap_or(Duration::MAX));
        exec.timeout = limit;
        self.context.environment.exec(self.context.docker, &exec)
    }

    /// Check `output` against `expect`; `Err` carries expected vs actual.
    fn check(&self, frame: &Frame, expect: &Expect, output: &Output) -> Result<(), String> {
        let wanted = expect.exit.unwrap_or(0);
        if output.exit != wanted {
            return Err(format!("expected exit {wanted}, got {}", describe(output)));
        }
        let stdout = output.stdout_text();
        let stderr = output.stderr_text();
        if let Some(exact) = &expect.stdout {
            let exact = self.text(frame, exact).map_err(|error| error.to_string())?;
            let actual = stdout.strip_suffix('\n').unwrap_or(&stdout);
            if actual != exact {
                return Err(format!(
                    "expected stdout {exact:?}, got {:?}",
                    clip(actual, 1500)
                ));
            }
        }
        if let Some(nonempty) = expect.stdout_nonempty
            && nonempty == stdout.trim().is_empty()
        {
            let wanted = if nonempty { "non-empty" } else { "empty" };
            return Err(format!("expected {wanted} stdout; {}", describe(output)));
        }
        self.check_contains(frame, "stdout", &stdout, &expect.stdout_contains, output)?;
        self.check_lacks(frame, "stdout", &stdout, &expect.stdout_lacks, output)?;
        self.check_contains(frame, "stderr", &stderr, &expect.stderr_contains, output)?;
        self.check_lacks(frame, "stderr", &stderr, &expect.stderr_lacks, output)?;
        if !expect.json.is_empty() {
            self.check_json(frame, &expect.json, &stdout)?;
        }
        Ok(())
    }

    /// Every needle must occur in `text`, the named output stream.
    fn check_contains(
        &self,
        frame: &Frame,
        stream: &str,
        text: &str,
        needles: &[String],
        output: &Output,
    ) -> Result<(), String> {
        for needle in needles {
            let needle = self
                .text(frame, needle)
                .map_err(|error| error.to_string())?;
            if !text.contains(&needle) {
                return Err(format!("{stream} lacks {needle:?}; {}", describe(output)));
            }
        }
        Ok(())
    }

    /// No needle may occur in `text`, the named output stream.
    fn check_lacks(
        &self,
        frame: &Frame,
        stream: &str,
        text: &str,
        needles: &[String],
        output: &Output,
    ) -> Result<(), String> {
        for needle in needles {
            let needle = self
                .text(frame, needle)
                .map_err(|error| error.to_string())?;
            if text.contains(&needle) {
                return Err(format!(
                    "{stream} unexpectedly contains {needle:?}; {}",
                    describe(output)
                ));
            }
        }
        Ok(())
    }

    /// Parse stdout as JSON and check every selected value against `checks`.
    fn check_json(&self, frame: &Frame, checks: &[JsonCheck], stdout: &str) -> Result<(), String> {
        let document: Value = serde_json::from_str(stdout).map_err(|error| {
            format!(
                "stdout is not JSON ({error}): {}",
                clip(stdout.trim(), 1500)
            )
        })?;
        for check in checks {
            self.check_json_selection(frame, check, &document)?;
        }
        Ok(())
    }

    fn check_json_selection(
        &self,
        frame: &Frame,
        check: &JsonCheck,
        document: &Value,
    ) -> Result<(), String> {
        let segments = parse_selector(&check.select).map_err(|error| error.to_string())?;
        let found = select(document, &segments);
        if check.absent {
            if let Some(found) = found {
                return Err(format!("{} should be absent, found {found}", check.select));
            }
            return Ok(());
        }
        let found = found.ok_or_else(|| {
            format!(
                "{} is missing in {}",
                check.select,
                clip(&document.to_string(), 1500)
            )
        })?;
        if let Some(expected) = &check.equals {
            let expected = self
                .value(frame, expected)
                .map_err(|error| error.to_string())?;
            if &expected != found {
                return Err(format!(
                    "{}: expected {expected}, got {found}",
                    check.select
                ));
            }
        }
        if let Some(expected) = &check.subset {
            let expected = self
                .value(frame, expected)
                .map_err(|error| error.to_string())?;
            if !is_subset(&expected, found) {
                return Err(format!(
                    "{}: {found} does not contain {expected}",
                    check.select
                ));
            }
        }
        Ok(())
    }

    fn capture(&self, frame: &mut Frame, command: &Command, output: &Output) -> Result<()> {
        for (name, capture) in &command.capture {
            let value = match capture {
                Capture::Stdout(StdoutCapture::Raw) => Value::String(output.stdout_text()),
                Capture::Stdout(StdoutCapture::Trim) => {
                    Value::String(output.stdout_text().trim().to_owned())
                }
                Capture::Json(selector) => {
                    let document: Value =
                        serde_json::from_str(&output.stdout_text()).map_err(|error| {
                            invalid(format!("capture {name}: stdout is not JSON: {error}"))
                        })?;
                    select(&document, &parse_selector(selector)?)
                        .cloned()
                        .ok_or_else(|| invalid(format!("capture {name}: {selector} is missing")))?
                }
            };
            frame.variables.insert(name.clone(), value);
        }
        Ok(())
    }

    fn record(
        &mut self,
        location: &str,
        operation: &'static str,
        label: Option<&String>,
        started: Instant,
        outcome: &Result<String, String>,
    ) {
        self.records.push(StepRecord {
            location: location.to_owned(),
            operation,
            label: label.cloned(),
            duration_ms: started.elapsed().as_millis(),
            passed: outcome.is_ok(),
            detail: match outcome {
                Ok(detail) | Err(detail) => clip(detail, 4000),
            },
        });
    }

    fn steps(
        &mut self,
        frame: &mut Frame,
        steps: &[Step],
        location: &str,
    ) -> Result<(), StepFailure> {
        for (index, step) in steps.iter().enumerate() {
            let here = format!("{location} step {}", index + 1);
            let started = Instant::now();
            let fail = |message: String| StepFailure {
                location: here.clone(),
                message,
            };
            let (operation, label, outcome): (
                &'static str,
                Option<&String>,
                Result<String, String>,
            ) = match step {
                Step::Exec(command) => {
                    ("exec", command.name.as_ref(), self.run_exec(frame, command))
                }
                Step::Eventually(eventually) => {
                    let limit =
                        parse_duration(&eventually.timeout).map_err(|e| fail(e.to_string()))?;
                    let interval = eventually
                        .interval
                        .as_deref()
                        .map(parse_duration)
                        .transpose()
                        .map_err(|e| fail(e.to_string()))?
                        .unwrap_or(Duration::from_secs(1));
                    let outcome = self.run_eventually(frame, &eventually.observe, limit, interval);
                    ("eventually", eventually.name.as_ref(), outcome)
                }
                Step::Assert(comparisons) => ("assert", None, self.run_assert(frame, comparisons)),
                Step::Action(call) => ("action", Some(&call.name), self.run_action(frame, call)),
                Step::Include(include) => {
                    let detail = self.run_include(frame, include, &here)?;
                    ("include", Some(&include.setup), Ok(detail))
                }
            };
            self.record(&here, operation, label, started, &outcome);
            if let Err(message) = outcome {
                return Err(fail(message));
            }
        }
        Ok(())
    }

    fn run_exec(&self, frame: &mut Frame, command: &Command) -> Result<String, String> {
        let output = self
            .run_command(frame, command, None)
            .map_err(|error| error.to_string())?;
        self.check(frame, &command.expect, &output)?;
        self.capture(frame, command, &output)
            .map(|_| describe(&output))
            .map_err(|error| error.to_string())
    }

    /// Repeat `observe` until it passes, fails permanently or `limit` expires.
    fn run_eventually(
        &self,
        frame: &mut Frame,
        observe: &Command,
        limit: Duration,
        interval: Duration,
    ) -> Result<String, String> {
        let until = Instant::now() + limit;
        let mut attempts = 0;
        loop {
            attempts += 1;
            let last = match self.observe_once(frame, observe, until, limit) {
                Observation::Passed(output) => {
                    return self
                        .capture(frame, observe, &output)
                        .map(|_| format!("passed after {attempts} attempts; {}", describe(&output)))
                        .map_err(|error| error.to_string());
                }
                Observation::Failed(message) => return Err(message),
                Observation::Pending(last) => last,
            };
            if Instant::now() + interval >= until {
                return Err(format!(
                    "not satisfied within {limit:?} ({attempts} attempts); last observation: {last}"
                ));
            }
            std::thread::sleep(interval);
        }
    }

    fn observe_once(
        &self,
        frame: &Frame,
        observe: &Command,
        until: Instant,
        limit: Duration,
    ) -> Observation {
        let left = until.saturating_duration_since(Instant::now());
        let output = match self.run_command(frame, observe, Some(left)) {
            Ok(output) => output,
            // Permanent errors (bad identity, engine failure) fail immediately.
            Err(error @ (Error::Invalid(_) | Error::Docker(_) | Error::Interrupted)) => {
                return Observation::Failed(error.to_string());
            }
            Err(error) => return Observation::Pending(error.to_string()),
        };
        if let Err(message) = self.check(frame, &observe.expect, &output) {
            return Observation::Pending(message);
        }
        if Instant::now() >= until {
            return Observation::Failed(format!(
                "observation completed after {limit:?} deadline; {}",
                describe(&output)
            ));
        }
        Observation::Passed(output)
    }

    fn run_assert(&self, frame: &Frame, comparisons: &[Comparison]) -> Result<String, String> {
        for comparison in comparisons {
            self.compare(frame, comparison)?;
        }
        Ok(format!("{} comparisons", comparisons.len()))
    }

    fn compare(&self, frame: &Frame, comparison: &Comparison) -> Result<(), String> {
        let value = self
            .value(frame, &comparison.value)
            .map_err(|e| e.to_string())?;
        if let Some(expected) = &comparison.equals {
            let expected = self.value(frame, expected).map_err(|e| e.to_string())?;
            if value != expected {
                return Err(format!("expected {expected}, got {value}"));
            }
        }
        if let Some(other) = &comparison.differs {
            let other = self.value(frame, other).map_err(|e| e.to_string())?;
            if value == other {
                return Err(format!("expected a value other than {other}"));
            }
        }
        if let Some(needle) = &comparison.contains {
            let needle = self.text(frame, needle).map_err(|e| e.to_string())?;
            match &value {
                Value::String(text) if text.contains(&needle) => {}
                Value::String(text) => {
                    return Err(format!(
                        "{:?} does not contain {needle:?}",
                        clip(text, 1500)
                    ));
                }
                other => return Err(format!("{other} is not text")),
            }
        }
        if let Some(choices) = &comparison.one_of {
            let choices = choices
                .iter()
                .map(|choice| self.value(frame, choice))
                .collect::<Result<Vec<_>>>()
                .map_err(|e| e.to_string())?;
            if !choices.contains(&value) {
                return Err(format!("{value} is not one of {choices:?}"));
            }
        }
        Ok(())
    }

    /// Resolve `with` inputs of an action or include.
    fn resolve_inputs(
        &self,
        frame: &Frame,
        inputs: &BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>> {
        inputs
            .iter()
            .map(|(key, value)| Ok((key.clone(), self.value(frame, value)?)))
            .collect()
    }

    fn run_action(&self, frame: &mut Frame, call: &ActionCall) -> Result<String, String> {
        let inputs = self
            .resolve_inputs(frame, &call.inputs)
            .map_err(|e| e.to_string())?;
        let outputs = actions::run(
            self.context,
            &call.name,
            &inputs,
            self.remaining().map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        for (output, local) in &call.capture {
            let value = outputs
                .get(output)
                .cloned()
                .ok_or_else(|| format!("action {} did not produce {output}", call.name))?;
            frame.variables.insert(local.clone(), value);
        }
        Ok(serde_json::to_string(&outputs).unwrap_or_default())
    }

    /// Run a setup's steps in a fresh frame and copy its captured outputs back.
    fn run_include(
        &mut self,
        frame: &mut Frame,
        include: &Include,
        here: &str,
    ) -> Result<String, StepFailure> {
        let fail = |message: String| StepFailure {
            location: here.to_owned(),
            message,
        };
        let setups = self.setups;
        let setup = setups
            .get(&include.setup)
            .ok_or_else(|| fail(format!("unknown setup {}", include.setup)))?;
        let inputs = self
            .resolve_inputs(frame, &include.inputs)
            .map_err(|e| fail(e.to_string()))?;
        let mut inner = Frame {
            variables: BTreeMap::new(),
            inputs,
        };
        self.steps(
            &mut inner,
            &setup.steps,
            &format!("{here} (setup {})", setup.name),
        )?;
        for (output, local) in &include.capture {
            let variable = &setup.outputs[output];
            let value =
                inner.variables.get(variable).cloned().ok_or_else(|| {
                    fail(format!("setup {} did not capture {variable}", setup.name))
                })?;
            frame.variables.insert(local.clone(), value);
        }
        Ok(format!("setup {}", setup.name))
    }

    /// Run a scenario's steps within its deadline.
    pub fn run(&mut self, scenario: &Scenario) -> Result<(), StepFailure> {
        let mut frame = Frame {
            variables: BTreeMap::new(),
            inputs: BTreeMap::new(),
        };
        let location = scenario
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| scenario.name.clone());
        self.steps(&mut frame, &scenario.steps, &location)
    }
}

/// The scenario directory shipped with this crate.
pub fn default_root() -> PathBuf {
    crate::harness_dir()
}
