//! `sysroot home update`: merge installed baseline changes into live files.
//!
//! For each managed file the last applied baseline B, the live file L and the
//! installed baseline N decide the result: unchanged L takes N, a changed L is
//! merged with `git merge-file`. A conflict leaves L untouched and writes the
//! marked merge to `<file>.kedra-new`; deleting that file marks it resolved. A
//! candidate passes its declared check before it replaces L, declared reloads
//! run once afterwards, and N is recorded as applied only when all succeeded.
use super::files::{self, Image, ManagedFile, Result};
use super::linux::{
    APPLIED, Home, Observed, PENDING, State, conflict_path, git, observe, replace, split, token,
};
use serde::Serialize;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    /// The applied baseline is already the installed one.
    UpToDate,
    /// The image has no baseline for this path.
    BaselineAbsent,
    /// A previous conflict file still exists.
    Unresolved,
    /// The live file already matched; only the record changed.
    Recorded,
    /// The file was missing and now holds the baseline.
    Created,
    /// The unchanged live file now holds the new baseline.
    Updated,
    /// Local edits and baseline changes were merged.
    Merged,
    /// The owner resolved a conflict; the live file is kept as is.
    Resolved,
    /// The merge conflicted; see the conflict file.
    Conflict,
    CheckFailed,
    ReloadFailed,
    Failed,
}

impl Kind {
    fn needs_attention(self) -> bool {
        matches!(
            self,
            Self::Unresolved
                | Self::Conflict
                | Self::CheckFailed
                | Self::ReloadFailed
                | Self::Failed
        )
    }
}

enum Action {
    Nothing(Kind),
    /// Optionally write new live bytes; `verify` checks and reloads the live file.
    Apply {
        write: Option<Vec<u8>>,
        verify: bool,
        kind: Kind,
        baseline: Vec<u8>,
    },
    Conflict {
        content: Vec<u8>,
        baseline: Vec<u8>,
    },
}

type Merge<'a> = &'a dyn Fn(&[u8], &[u8], &[u8]) -> Result<(Vec<u8>, bool)>;

fn apply(write: Option<Vec<u8>>, verify: bool, kind: Kind, baseline: Vec<u8>) -> Action {
    Action::Apply {
        write,
        verify,
        kind,
        baseline,
    }
}

fn conflict(content: Vec<u8>, baseline: Vec<u8>) -> Action {
    Action::Conflict { content, baseline }
}

/// No applied baseline: a matching or missing file is recorded or created; a
/// different one cannot be merged without a base and becomes a conflict.
fn unrecorded(live: Option<Vec<u8>>, new: Vec<u8>) -> Action {
    match live {
        None => apply(Some(new.clone()), false, Kind::Created, new),
        Some(live) if live == new => apply(None, false, Kind::Recorded, new),
        Some(_) => conflict(new.clone(), new),
    }
}

fn three_way(
    live: Vec<u8>,
    base: Vec<u8>,
    new: Vec<u8>,
    resolving: bool,
    merge: Merge<'_>,
) -> Result<Action> {
    let unchanged = if resolving {
        Kind::Resolved
    } else {
        Kind::Recorded
    };
    if live == new {
        return Ok(apply(None, resolving, unchanged, new));
    }
    if live == base {
        return Ok(apply(Some(new.clone()), false, Kind::Updated, new));
    }
    if base == new {
        // Reached only while resolving: the owner's file is the result.
        return Ok(apply(None, true, Kind::Resolved, new));
    }
    let (merged, conflicted) = merge(&live, &base, &new)?;
    Ok(if conflicted {
        conflict(merged, new)
    } else if merged == live {
        apply(None, resolving, unchanged, new)
    } else {
        apply(Some(merged), false, Kind::Merged, new)
    })
}

fn decide(observed: Observed, merge: Merge<'_>) -> Result<Action> {
    let Some(new) = observed.baseline else {
        return Ok(Action::Nothing(Kind::BaselineAbsent));
    };
    let resolving = observed.pending.is_some();
    if resolving && observed.conflict_file {
        return Ok(Action::Nothing(Kind::Unresolved));
    }
    let Some(base) = observed.pending.or(observed.applied) else {
        return Ok(unrecorded(observed.live, new));
    };
    if base == new && !resolving {
        return Ok(Action::Nothing(Kind::UpToDate));
    }
    match observed.live {
        Some(live) => three_way(live, base, new, resolving, merge),
        None if base == new => Ok(apply(None, true, Kind::Resolved, new)),
        None => Ok(conflict(new.clone(), new)),
    }
}

/// `git merge-file` on private copies; true when the result has conflicts.
fn merge(state: &State, live: &[u8], base: &[u8], new: &[u8]) -> Result<(Vec<u8>, bool)> {
    let scratch = state.scratch()?;
    for (name, bytes) in [("live", live), ("base", base), ("new", new)] {
        scratch.write(name, bytes)?;
    }
    let (code, merged) = git(
        scratch.path(),
        &[
            "merge-file",
            "-p",
            "--diff3",
            "-L",
            "live",
            "-L",
            "last applied baseline",
            "-L",
            "new baseline",
            "live",
            "base",
            "new",
        ],
    )?;
    Ok((merged, code != 0))
}

#[derive(Serialize)]
struct Report {
    path: String,
    result: Kind,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

impl Report {
    fn new(path: &str, result: Kind, detail: Option<String>) -> Self {
        Self {
            path: path.to_owned(),
            result,
            detail,
        }
    }
}

/// A file whose live content is settled; its baseline is recorded after reload.
struct Settled {
    path: String,
    baseline: Vec<u8>,
    kind: Kind,
    reload: Option<Vec<String>>,
}

enum Step {
    Done(Report),
    Settled(Settled),
}

/// Run the declared check on a candidate placed beside the live file.
fn check_candidate(
    home: &Home,
    path: &str,
    bytes: &[u8],
    check: &[String],
) -> Result<Option<String>> {
    let directory = home
        .directory(path, true)?
        .ok_or("managed directory is unavailable")?;
    let mut failure = None;
    directory.validate_bytes(&token()?, bytes, |candidate| {
        let outcome = files::execute(check, Some(candidate))?;
        if !outcome.success {
            failure = Some(outcome.output);
        }
        Ok(())
    })?;
    Ok(failure)
}

fn check_live(home: &Home, file: &ManagedFile) -> Result<Option<String>> {
    let Some(check) = &file.check else {
        return Ok(None);
    };
    if home.read(&file.path)?.is_none() {
        return Ok(None);
    }
    let outcome = files::execute(check, Some(&home.path(&file.path)))?;
    Ok((!outcome.success).then_some(outcome.output))
}

fn write_conflict(
    home: &Home,
    state: &State,
    path: &str,
    content: &[u8],
    baseline: &[u8],
) -> Result<Report> {
    let name = conflict_path(split(path).1);
    let directory = home
        .directory(path, true)?
        .ok_or("managed directory is unavailable")?;
    if directory.read(&name)?.is_some() {
        return Err(format!(
            "~/{} exists and was not written by an update; move it away",
            conflict_path(path)
        )
        .into());
    }
    replace(&directory, &name, None, content)?;
    state.write(PENDING, path, baseline)?;
    let detail = format!(
        "merge ~/{} into ~/{path}, delete it, then run sysroot home update",
        conflict_path(path)
    );
    Ok(Report::new(path, Kind::Conflict, Some(detail)))
}

fn step(image: &Image, home: &Home, state: &State, file: &ManagedFile) -> Result<Step> {
    let observed = observe(image, home, Some(state), &file.path)?;
    let live = observed.live.clone();
    let action = decide(observed, &|live, base, new| merge(state, live, base, new))?;
    let (write, verify, kind, baseline) = match action {
        Action::Nothing(kind) => return Ok(Step::Done(Report::new(&file.path, kind, None))),
        Action::Conflict { content, baseline } => {
            return write_conflict(home, state, &file.path, &content, &baseline).map(Step::Done);
        }
        Action::Apply {
            write,
            verify,
            kind,
            baseline,
        } => (write, verify, kind, baseline),
    };
    let failure = match (&write, &file.check) {
        (Some(bytes), Some(check)) => check_candidate(home, &file.path, bytes, check)?,
        (None, _) if verify => check_live(home, file)?,
        _ => None,
    };
    if let Some(output) = failure {
        return Ok(Step::Done(Report::new(
            &file.path,
            Kind::CheckFailed,
            Some(output),
        )));
    }
    if let Some(bytes) = &write {
        let directory = home
            .directory(&file.path, true)?
            .ok_or("managed directory is unavailable")?;
        replace(&directory, split(&file.path).1, live.as_deref(), bytes)?;
    }
    Ok(Step::Settled(Settled {
        path: file.path.clone(),
        baseline,
        kind,
        reload: (write.is_some() || verify)
            .then(|| file.reload.clone())
            .flatten(),
    }))
}

/// Run each distinct reload once, then record the baselines it covered.
fn reload_and_record(state: &State, settled: Vec<Settled>) -> Vec<Report> {
    let mut failures: Vec<(Vec<String>, Option<String>)> = Vec::new();
    for argv in settled.iter().filter_map(|file| file.reload.as_ref()) {
        if failures.iter().any(|(done, _)| done == argv) {
            continue;
        }
        let failure = match files::execute(argv, None) {
            Ok(outcome) => (!outcome.success).then_some(outcome.output),
            Err(error) => Some(error.to_string()),
        };
        failures.push((argv.clone(), failure));
    }
    settled
        .into_iter()
        .map(|file| {
            let failure = file.reload.as_ref().and_then(|argv| {
                failures
                    .iter()
                    .find(|(done, _)| done == argv)
                    .and_then(|(_, failure)| failure.clone())
            });
            if let Some(output) = failure {
                return Report::new(&file.path, Kind::ReloadFailed, Some(output));
            }
            match state
                .write(APPLIED, &file.path, &file.baseline)
                .and_then(|()| state.remove(PENDING, &file.path))
            {
                Ok(()) => Report::new(&file.path, file.kind, None),
                Err(error) => Report::new(&file.path, Kind::Failed, Some(error.to_string())),
            }
        })
        .collect()
}

fn print(reports: &[Report], json: bool) -> Result<()> {
    if json {
        let report = serde_json::json!({"schema_version": 1, "files": reports});
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }
    for report in reports {
        let result = serde_json::to_value(report.result)?;
        print!("{}: {}", report.path, result.as_str().unwrap_or_default());
        match &report.detail {
            Some(detail) => println!(": {detail}"),
            None => println!(),
        }
    }
    Ok(())
}

pub(super) fn run(
    image: &Image,
    home: &Home,
    state: &State,
    paths: &[String],
    json: bool,
) -> Result<()> {
    let _lock = state.lock()?;
    let files = files::select(image.files()?, paths)?;
    let mut reports = Vec::new();
    let mut settled = Vec::new();
    for file in &files {
        match step(image, home, state, file) {
            Ok(Step::Done(report)) => reports.push(report),
            Ok(Step::Settled(file)) => settled.push(file),
            Err(error) => reports.push(Report::new(
                &file.path,
                Kind::Failed,
                Some(error.to_string()),
            )),
        }
    }
    reports.extend(reload_and_record(state, settled));
    reports.sort_by_key(|report| files.iter().position(|file| file.path == report.path));
    print(&reports, json)?;
    let attention = reports
        .iter()
        .filter(|report| report.result.needs_attention())
        .count();
    if attention > 0 {
        return Err(format!("{attention} managed home file(s) need attention").into());
    }
    Ok(())
}
