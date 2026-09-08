//! Explicit niri text adoption, Git-backed selection and source-only publication.
//! Live file writes and baseline activation are deliberately separate operations.
use super::{TextCommand, export, linux};
use crate::source;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use sysroot_helper::storage::Store;

mod activation;
mod transition;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const RECORD: &str = "niri-text";
const FILE: &str = ".config/niri/config.kdl";
const DESTINATION: &str = "usr/share/sysroot/home/default/.config/niri/config.kdl";
const LIMIT: usize = 131_072;

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn content(bytes: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(bytes).map_err(|_| "managed text must be UTF-8")?;
    if bytes.len() > LIMIT
        || text.lines().count() > 8192
        || text.contains(['\0', '\r'])
        || (!text.is_empty() && !text.ends_with('\n'))
    {
        return Err("managed text must be bounded LF text ending with a newline".into());
    }
    Ok(text.to_owned())
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Baseline {
    target: String,
    source_path: String,
    source_revision: String,
    contents: String,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Change {
    id: String,
    /// Zero-based boundary in the reference, content-bound rather than relocated.
    at: usize,
    before: String,
    after: String,
}
impl Change {
    fn new(reference: &str, at: usize, before: String, after: String) -> Result<Self> {
        let id = hash(&serde_json::to_vec(&(
            hash(reference.as_bytes()),
            at,
            &before,
            &after,
        ))?);
        Ok(Self {
            id,
            at,
            before,
            after,
        })
    }
    fn end(&self) -> usize {
        self.at + self.before.lines().count()
    }
    fn overlaps(&self, other: &Self) -> bool {
        if self.at == self.end() || other.at == other.end() {
            self.at.max(other.at) <= self.end().min(other.end())
        } else {
            self.at.max(other.at) < self.end().min(other.end())
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Publication {
    source_revision: String,
    reference: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct State {
    schema_version: u32,
    instance: String,
    baseline: Baseline,
    /// Accepted source publication is distinct from the image's accepted B.
    reference: String,
    selected: Vec<Change>,
    ignored: Vec<Change>,
    published: Vec<Publication>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pending_activation: Option<String>,
}
impl State {
    fn validate(&self, instance: &str) -> Result<()> {
        if self.schema_version != 1
            || self
                .pending_activation
                .as_ref()
                .is_some_and(|v| !hex(v, 32))
            || self.instance != instance
            || !hex(&self.baseline.source_revision, 40)
            || self.published.len() > 128
            || self.baseline.target.is_empty()
            || self.baseline.target.len() > 63
            || !self
                .baseline
                .target
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || ![
                format!("home/{FILE}"),
                format!("hosts/{}/home/{FILE}", self.baseline.target),
            ]
            .contains(&self.baseline.source_path)
        {
            return Err(
                "text review schema, instance or provenance is invalid; preserve the store".into(),
            );
        }
        content(self.baseline.contents.as_bytes())?;
        content(self.reference.as_bytes())?;
        for published in &self.published {
            if !hex(&published.source_revision, 40) {
                return Err("text publication history is invalid".into());
            }
            content(published.reference.as_bytes())?;
        }
        if self.reference
            != self
                .published
                .last()
                .map_or(&self.baseline.contents, |p| &p.reference)
                .as_str()
        {
            return Err("text reference differs from its recorded publication".into());
        }
        let lines: Vec<_> = self.reference.split_inclusive('\n').collect();
        let changes: Vec<_> = self.selected.iter().chain(&self.ignored).collect();
        if changes.len() > 8192 {
            return Err("too many text decisions".into());
        }
        for (index, change) in changes.iter().enumerate() {
            content(change.before.as_bytes())?;
            content(change.after.as_bytes())?;
            if change.at > lines.len()
                || change.end() > lines.len()
                || lines[change.at..change.end()].concat() != change.before
                || change.before == change.after
                || **change
                    != Change::new(
                        &self.reference,
                        change.at,
                        change.before.clone(),
                        change.after.clone(),
                    )?
                || changes[..index].iter().any(|prior| change.overlaps(prior))
            {
                return Err(
                    "text selection/local policy is corrupt or overlapping; preserve the store"
                        .into(),
                );
            }
        }
        Ok(())
    }
}

/// Every temporary Git object stays in a private scratch repository. Only the
/// approved result is subsequently hashed into the public source repository.
struct Git {
    scratch: export::Scratch,
    repo: PathBuf,
    index: PathBuf,
}
impl Git {
    fn new(parent: &Path) -> Result<Self> {
        let scratch = export::Scratch::new(parent)?;
        let repo = scratch.0.join("objects.git");
        let index = scratch.0.join("index");
        export::git(
            &scratch.0,
            &[
                "init",
                "--bare",
                "--object-format=sha1",
                "--template=",
                "objects.git",
            ],
            None,
            &index,
        )?;
        Ok(Self {
            scratch,
            repo,
            index,
        })
    }
    fn run(&self, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
        Ok(export::git(&self.repo, args, input, &self.index)?)
    }
    fn blob(&self, value: &str) -> Result<String> {
        Ok(
            String::from_utf8(
                self.run(&["hash-object", "-w", "--stdin"], Some(value.as_bytes()))?,
            )?
            .trim()
            .to_owned(),
        )
    }
    fn changes(&self, reference: &str, live: &str) -> Result<Vec<Change>> {
        // Git's no-index stdin path compares the live bytes in memory. Do not
        // hash the complete live configuration into any Git object database.
        // Git 2.55.0 diff-no-index.c documents '-' and exit statuses 0/1.
        std::fs::write(self.scratch.0.join("reference"), reference)?;
        let mut command = Command::new("/usr/bin/timeout");
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        let mut child = command
            .args(["--kill-after=2s", "15s", "/usr/bin/git", "--no-pager"])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .args([
                "-c",
                "core.attributesfile=/dev/null",
                "diff",
                "--no-index",
                "--text",
                "--no-ext-diff",
                "--no-textconv",
                "--no-color",
                "--unified=0",
                "--",
                "reference",
                "-",
            ])
            .current_dir(&self.scratch.0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let written = child
            .stdin
            .take()
            .ok_or("Git comparison input is unavailable")?
            .write_all(live.as_bytes());
        if let Err(error) = written {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error.into());
        }
        let output = child.wait_with_output()?;
        if !matches!(output.status.code(), Some(0 | 1)) || output.stdout.len() > LIMIT * 3 {
            return Err("Git could not compare the bounded live configuration".into());
        }
        let patch = String::from_utf8(output.stdout)?;
        parse_changes(reference, &patch)
    }
    fn apply(&self, reference: &str, changes: &[Change]) -> Result<String> {
        if changes.is_empty() {
            return Ok(reference.to_owned());
        }
        self.run(&["read-tree", "--empty"], None)?;
        let blob = self.blob(reference)?;
        self.run(
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                "100644",
                &blob,
                "managed",
            ],
            None,
        )?;
        let mut ordered = changes.to_vec();
        ordered.sort_by_key(|c| c.at);
        let mut patch = String::from("--- a/managed\n+++ b/managed\n");
        let mut delta = 0isize;
        for change in ordered {
            let old = change.before.lines().count();
            let new = change.after.lines().count();
            let old_start = change.at + usize::from(old != 0);
            let new_start = change.at as isize + delta + isize::from(new != 0);
            if new_start < 0 {
                return Err("selected patch coordinates are invalid".into());
            }
            patch.push_str(&format!("@@ -{old_start},{old} +{new_start},{new} @@\n"));
            for line in change.before.split_inclusive('\n') {
                patch.push('-');
                patch.push_str(line);
            }
            for line in change.after.split_inclusive('\n') {
                patch.push('+');
                patch.push_str(line);
            }
            delta += new as isize - old as isize;
        }
        self.run(
            &[
                "apply",
                "--cached",
                "--unidiff-zero",
                "--whitespace=nowarn",
                "-",
            ],
            Some(patch.as_bytes()),
        )?;
        content(&self.run(&["show", ":managed"], None)?)
    }
    fn merge(&self, current: &str, base: &str, selected: &str) -> Result<String> {
        use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};
        use std::os::fd::AsRawFd;
        let mut files = Vec::new();
        let mut paths = Vec::new();
        for value in [current, base, selected] {
            let mut file = std::fs::File::from(memfd_create(
                "kedra-text-merge",
                MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
            )?);
            file.write_all(value.as_bytes())?;
            fcntl_add_seals(
                &file,
                SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL,
            )?;
            paths.push(format!(
                "/proc/{}/fd/{}",
                std::process::id(),
                file.as_raw_fd()
            ));
            files.push(file);
        }
        let result = export::git(
            &self.scratch.0,
            &[
                "merge-file",
                "--diff3",
                "-p",
                &paths[0],
                &paths[1],
                &paths[2],
            ],
            None,
            &self.index,
        )
        .map_err(|_| "text changes conflict with source or local policy; nothing was exported")?;
        content(&result)
    }
}

struct Hunk {
    at: usize,
    old: usize,
    new: usize,
    before: Vec<String>,
    after: Vec<String>,
}
fn range(value: &str, prefix: char) -> Result<(usize, usize)> {
    let value = value.strip_prefix(prefix).ok_or("Git range is malformed")?;
    let (start, count) = value.split_once(',').unwrap_or((value, "1"));
    Ok((start.parse()?, count.parse()?))
}
fn finish(reference: &str, hunk: Hunk, changes: &mut Vec<Change>) -> Result<()> {
    if hunk.before.len() != hunk.old || hunk.after.len() != hunk.new {
        return Err("Git patch line counts disagree".into());
    }
    if hunk.old == hunk.new && hunk.old > 1 {
        for (offset, (before, after)) in hunk.before.into_iter().zip(hunk.after).enumerate() {
            changes.push(Change::new(reference, hunk.at + offset, before, after)?);
        }
    } else {
        changes.push(Change::new(
            reference,
            hunk.at,
            hunk.before.concat(),
            hunk.after.concat(),
        )?);
    }
    Ok(())
}
fn parse_changes(reference: &str, patch: &str) -> Result<Vec<Change>> {
    let mut result = Vec::new();
    let mut current: Option<Hunk> = None;
    for line in patch.split_inclusive('\n') {
        if line.starts_with("@@ ") {
            if let Some(hunk) = current.take() {
                finish(reference, hunk, &mut result)?;
            }
            let mut parts = line.split_whitespace();
            let _ = parts.next();
            let (old_start, old) = range(parts.next().ok_or("missing Git old range")?, '-')?;
            let (_, new) = range(parts.next().ok_or("missing Git new range")?, '+')?;
            let at = if old == 0 {
                old_start
            } else {
                old_start.checked_sub(1).ok_or("invalid Git range")?
            };
            current = Some(Hunk {
                at,
                old,
                new,
                before: Vec::new(),
                after: Vec::new(),
            });
        } else if let Some(hunk) = &mut current {
            if let Some(value) = line.strip_prefix('-') {
                hunk.before.push(value.to_owned());
            } else if let Some(value) = line.strip_prefix('+') {
                hunk.after.push(value.to_owned());
            } else {
                return Err("unsupported Git patch body".into());
            }
        }
    }
    if let Some(hunk) = current {
        finish(reference, hunk, &mut result)?;
    }
    Ok(result)
}

#[derive(Deserialize)]
struct Manifest {
    schema_version: u32,
    source_revision: String,
    target: Target,
    files: Vec<Payload>,
}
#[derive(Deserialize)]
struct Target {
    id: String,
}
#[derive(Deserialize)]
struct Payload {
    source_path: String,
    destination: String,
    sha256: String,
    home_baseline: bool,
    mode: String,
}
fn installed() -> Result<Baseline> {
    let manifest: Manifest = serde_json::from_slice(&linux::read_regular(
        Path::new("/usr/share/sysroot/source.json"),
        0,
        1_048_576,
    )?)?;
    let files: Vec<_> = manifest
        .files
        .iter()
        .filter(|f| f.destination == DESTINATION && f.home_baseline)
        .collect();
    if manifest.schema_version != 1 || files.len() != 1 || files[0].mode != "100644" {
        return Err("installed niri baseline lacks unique ordinary-file provenance".into());
    }
    let bytes = linux::read_regular(&Path::new("/").join(DESTINATION), 0, LIMIT)?;
    if hash(&bytes) != files[0].sha256 {
        return Err("installed niri baseline hash differs".into());
    }
    Ok(Baseline {
        target: manifest.target.id,
        source_path: files[0].source_path.clone(),
        source_revision: manifest.source_revision,
        contents: content(&bytes)?,
    })
}
fn live_path() -> Result<PathBuf> {
    let requested_home = PathBuf::from(std::env::var_os("HOME").ok_or("HOME is unavailable")?);
    if !requested_home.is_absolute() {
        return Err("HOME must be absolute".into());
    }
    let home = requested_home.canonicalize()?;
    if let Some(value) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty())
        && Path::new(&value) != home.join(".config")
        && Path::new(&value) != requested_home.join(".config")
    {
        return Err("niri text review supports the default home profile only".into());
    }
    if let Some(value) = std::env::var_os("NIRI_CONFIG").filter(|v| !v.is_empty())
        && Path::new(&value) != home.join(FILE)
        && Path::new(&value) != requested_home.join(FILE)
    {
        return Err(
            "NIRI_CONFIG selects a different file; it is not adopted by this adapter".into(),
        );
    }
    Ok(home.join(FILE))
}
fn live() -> Result<String> {
    content(&linux::read_regular(
        &live_path()?,
        rustix::process::geteuid().as_raw(),
        LIMIT,
    )?)
}

fn source_content(
    repo: &Path,
    state: &State,
    revision: Option<&str>,
) -> Result<(source::Plan, String)> {
    let plan = match revision {
        Some(revision) => source::plan_revision(repo, &state.baseline.target, revision)?,
        None => source::plan(repo, &state.baseline.target)?,
    };
    for anchor in std::iter::once(&state.baseline.source_revision)
        .chain(state.published.iter().map(|p| &p.source_revision))
    {
        if source::git(
            repo,
            &["merge-base", "--is-ancestor", anchor, &plan.source_revision],
        )
        .is_err()
        {
            return Err(
                "source lacks accepted baseline/publication ancestry; fetch and review first"
                    .into(),
            );
        }
    }
    let file = plan
        .files
        .iter()
        .find(|f| f.destination == DESTINATION && f.home_baseline)
        .ok_or("source niri baseline is missing")?;
    if file.source_path != state.baseline.source_path || file.mode != "100644" {
        return Err(
            "source niri target/override provenance changed; explicit rebind is required".into(),
        );
    }
    let contents = content(&source::git(repo, &["cat-file", "blob", &file.git_blob])?)?;
    Ok((plan, contents))
}

fn select(state: &mut State, git: &Git, id: &str, local: bool) -> Result<()> {
    let observed = git.changes(&state.reference, &live()?)?;
    let change = observed
        .into_iter()
        .find(|c| c.id == id)
        .ok_or("change is stale or missing; review file status again")?;
    let (destination, excluded) = if local {
        (&mut state.ignored, &state.selected)
    } else {
        (&mut state.selected, &state.ignored)
    };
    if excluded.iter().any(|c| c.overlaps(&change)) {
        return Err(
            "selection and local-only decisions overlap; clear the other disposition first".into(),
        );
    }
    destination.retain(|c| !(c.at == change.at && c.before == change.before));
    if destination.iter().any(|c| c.overlaps(&change)) {
        return Err(
            "existing selection uses a different overlapping range; unstage it first".into(),
        );
    }
    destination.push(change);
    destination.sort_by_key(|c| c.at);
    Ok(())
}

pub(super) fn run(
    store: &mut Store,
    parent: &Path,
    instance: &str,
    path: &str,
    command: &TextCommand,
) -> Result<()> {
    if path != FILE {
        return Err(
            "ordinary text adoption currently supports only .config/niri/config.kdl".into(),
        );
    }
    let record = store.read(RECORD)?;
    let revision = record.as_ref().map(|r| r.revision);
    let mut state = if matches!(
        command,
        TextCommand::Init {
            reviewed_safe: true
        }
    ) {
        if record.is_some() {
            return Err("niri text is already adopted; existing state is never reset".into());
        }
        let baseline = installed()?;
        // Validate only this explicitly approved path before any decisions persist.
        live()?;
        State {
            schema_version: 1,
            instance: instance.to_owned(),
            reference: baseline.contents.clone(),
            baseline,
            selected: Vec::new(),
            ignored: Vec::new(),
            published: Vec::new(),
            pending_activation: None,
        }
    } else {
        let record = record.ok_or("initialize home review, then use home file init --reviewed-safe after reviewing niri for secrets")?;
        serde_json::from_slice::<State>(&record.bytes)
            .map_err(|_| "niri text state is malformed; preserve the store")?
    };
    state.validate(instance)?;
    if activation::handles(command) {
        return activation::run(store, parent, &state, command);
    }
    if state.pending_activation.is_some()
        && !matches!(command, TextCommand::Status | TextCommand::Selection)
    {
        return Err("niri activation is pending; inspect home file recover first".into());
    }
    if let TextCommand::Plan { repo, commit } = command {
        return transition::preview(&state, parent, repo, commit.as_deref());
    }
    let before = serde_json::to_vec(&state)?;
    let git = Git::new(parent)?;
    let mut source_result = None;
    match command {
        TextCommand::Init {
            reviewed_safe: false,
        } => return Err("explicit reviewed-safe acknowledgement is required".into()),
        TextCommand::Stage { change } => select(&mut state, &git, change, false)?,
        TextCommand::KeepLocal { change } => select(&mut state, &git, change, true)?,
        TextCommand::Unstage { change } => {
            if !state.selected.iter().any(|c| c.id == *change) {
                return Err("selected change ID is missing".into());
            }
            state.selected.retain(|c| c.id != *change);
        }
        TextCommand::ClearLocal { change } => {
            if !state.ignored.iter().any(|c| c.id == *change) {
                return Err("local-only change ID is missing".into());
            }
            state.ignored.retain(|c| c.id != *change);
        }
        TextCommand::Export { repo, output } => {
            if state.selected.is_empty() {
                return Err("no selected text changes to export".into());
            }
            let repo = export::checkout(repo)?;
            let (plan, current) = source_content(&repo, &state, None)?;
            let selected = git.apply(&state.reference, &state.selected)?;
            let merged = git.merge(&current, &state.reference, &selected)?;
            let index = git.scratch.0.join("source-index");
            export::git(&repo, &["read-tree", &plan.source_revision], None, &index)?;
            let blob = String::from_utf8(export::git(
                &repo,
                &["hash-object", "-w", "--stdin"],
                Some(merged.as_bytes()),
                &index,
            )?)?;
            export::git(
                &repo,
                &[
                    "update-index",
                    "--cacheinfo",
                    "100644",
                    blob.trim(),
                    &state.baseline.source_path,
                ],
                None,
                &index,
            )?;
            let patch = export::git(
                &repo,
                &[
                    "diff",
                    "--cached",
                    "--text",
                    "--full-index",
                    "--no-ext-diff",
                    "--no-textconv",
                    "--no-color",
                    "--src-prefix=a/",
                    "--dst-prefix=b/",
                    &plan.source_revision,
                    "--",
                    &state.baseline.source_path,
                ],
                None,
                &index,
            )?;
            if String::from_utf8(source::git(&repo, &["rev-parse", "HEAD"])?)?.trim()
                != plan.source_revision
            {
                return Err("source advanced during export; review and retry".into());
            }
            if patch.len() > LIMIT * 2 {
                return Err("selected patch exceeds output bound".into());
            }
            let prepared = export::Prepared {
                source_revision: plan.source_revision,
                source_path: state.baseline.source_path.clone(),
                patch,
            };
            prepared.write_new(output)?;
            source_result = Some(
                serde_json::json!({"patch_file":output,"base_revision":prepared.source_revision,"source_path":prepared.source_path,"patch_sha256":hash(&prepared.patch),"already_in_source":prepared.patch.is_empty(),"commit_created":false}),
            );
        }
        TextCommand::RecordSource { repo, commit } => {
            if state.selected.is_empty() || !hex(commit, 40) {
                return Err("record-source requires selected changes and a full commit ID".into());
            }
            let repo = export::checkout(repo)?;
            let (_, current) = source_content(&repo, &state, Some(commit))?;
            let selected = git.apply(&state.reference, &state.selected)?;
            if git.merge(&current, &state.reference, &selected)? != current {
                return Err("source commit does not contain the exact selected changes".into());
            }
            // Reanchor only known disjoint policy ranges across this exact
            // selected publication. Never search for a similar relocated line.
            state.ignored = state
                .ignored
                .iter()
                .map(|ignored| {
                    let shift: isize = state
                        .selected
                        .iter()
                        .filter(|change| change.end() <= ignored.at)
                        .map(|change| {
                            change.after.lines().count() as isize
                                - change.before.lines().count() as isize
                        })
                        .sum();
                    let at = ignored
                        .at
                        .checked_add_signed(shift)
                        .ok_or("local policy position overflow")?;
                    Change::new(&selected, at, ignored.before.clone(), ignored.after.clone())
                })
                .collect::<Result<Vec<_>>>()?;
            state.reference = selected.clone();
            state.published.push(Publication {
                source_revision: commit.clone(),
                reference: selected,
            });
            state.selected.clear();
            source_result = Some(
                serde_json::json!({"recorded_commit":commit,"deployment_performed":false,"registry_publication_verified":false}),
            );
        }
        _ => (),
    }
    state.validate(instance)?;
    let after = serde_json::to_vec(&state)?;
    if revision.is_none() || before != after {
        store.compare_exchange(RECORD, revision, &after)?;
    }
    let mut response = serde_json::json!({"schema_version":1,"path":FILE,"accepted_baseline":{"source_revision":state.baseline.source_revision,"source_path":state.baseline.source_path,"target":state.baseline.target},"reference_sha256":hash(state.reference.as_bytes()),"selection":state.selected,"local_only":state.ignored,"publication_count":state.published.len(),"live_file_changed":false,"checkout_changed":false,"activation_available":false});
    if !matches!(
        command,
        TextCommand::Selection | TextCommand::Export { .. } | TextCommand::RecordSource { .. }
    ) {
        let observed = git.changes(&state.reference, &live()?)?;
        response["changes"] = serde_json::json!(observed.iter().map(|change| serde_json::json!({"change":change,"selected":state.selected.contains(change),"local_only":state.ignored.contains(change),"visible_change":!state.ignored.contains(change)})).collect::<Vec<_>>());
    }
    if let Some(result) = source_result {
        response["source"] = result;
    }
    response["pending_activation"] = serde_json::json!(state.pending_activation);
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
