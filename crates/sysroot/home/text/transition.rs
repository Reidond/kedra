//! Advisory source preflight. No home file or accepted review state is written.
use super::{
    Baseline, Change, DESTINATION, Git, Publication, Result, State, content, export, hash, live,
    source,
};
use std::path::Path;
use std::process::{Command, Stdio};

fn ancestor(repo: &Path, earlier: &str, later: &str) -> Result<bool> {
    let mut command = Command::new("git");
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    let status = command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .args(["--no-replace-objects", "-C"])
        .arg(repo)
        .args(["merge-base", "--is-ancestor", earlier, later])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err("Git could not establish publication ancestry".into()),
    }
}

fn reanchor(
    git: &Git,
    old: &str,
    new: &str,
    decisions: &[Change],
    keep_local: bool,
) -> Result<Vec<Change>> {
    let changes = git.changes(old, new)?;
    let lines: Vec<_> = new.split_inclusive('\n').collect();
    let mut result = Vec::new();
    for decision in decisions {
        let mut shift = 0isize;
        let mut inside = 0isize;
        let mut adopted_insertion = false;
        for change in &changes {
            let delta =
                change.after.lines().count() as isize - change.before.lines().count() as isize;
            if decision.overlaps(change) {
                if decision.before.is_empty()
                    && change.before.is_empty()
                    && decision.at == change.at
                    && decision.after == change.after
                {
                    adopted_insertion = true;
                    continue;
                }
                if change.at < decision.at
                    || change.end() > decision.end()
                    || decision.before.is_empty()
                    || (change.before.is_empty()
                        && (change.at == decision.at || change.at == decision.end()))
                {
                    return Err("source changes overlap a decision boundary ambiguously; review that selection/local rule first".into());
                }
                inside += delta;
            } else if change.end() <= decision.at {
                shift += delta;
            }
        }
        if adopted_insertion {
            continue;
        }
        let at = decision
            .at
            .checked_add_signed(shift)
            .ok_or("decision position overflow")?;
        let length = decision
            .before
            .lines()
            .count()
            .checked_add_signed(inside)
            .ok_or("decision range overflow")?;
        let end = at.checked_add(length).ok_or("decision range overflow")?;
        let before = lines
            .get(at..end)
            .ok_or("decision range differs after source changes")?
            .concat();
        if before == decision.after {
            continue;
        }
        if !keep_local && before != decision.before {
            return Err("the source baseline conflicts with a pinned selection; review that selection first".into());
        }
        result.push(Change::new(new, at, before, decision.after.clone())?);
    }
    Ok(result)
}

pub(super) fn preview(
    state: &State,
    parent: &Path,
    repo: &Path,
    commit: Option<&str>,
) -> Result<()> {
    let repo = export::checkout(repo)?;
    let plan = match commit {
        Some(commit) => source::plan_revision(&repo, &state.baseline.target, commit)?,
        None => source::plan(&repo, &state.baseline.target)?,
    };
    if String::from_utf8(source::git(
        &repo,
        &["rev-parse", "--is-shallow-repository"],
    )?)?
    .trim()
        != "false"
    {
        return Err(
            "text reconciliation needs complete source ancestry; fetch the required history first"
                .into(),
        );
    }
    let file = plan
        .files
        .iter()
        .find(|file| file.destination == DESTINATION && file.home_baseline)
        .ok_or("candidate source has no niri baseline")?;
    if file.source_path != state.baseline.source_path || file.mode != "100644" {
        return Err(
            "candidate source changes niri override/path/type; explicit rebind is required".into(),
        );
    }
    for anchor in std::iter::once(&state.baseline.source_revision)
        .chain(state.published.iter().map(|p| &p.source_revision))
    {
        let resolved = source::git(
            &repo,
            &["rev-parse", "--verify", &format!("{anchor}^{{commit}}")],
        )
        .map_err(|_| "required accepted/publication source history is missing")?;
        if String::from_utf8(resolved)?.trim() != anchor {
            return Err("source anchor does not resolve to the recorded commit".into());
        }
    }
    let baseline = Baseline {
        target: plan.target.id,
        source_path: file.source_path.clone(),
        source_revision: plan.source_revision,
        contents: content(&source::git(&repo, &["cat-file", "blob", &file.git_blob])?)?,
    };
    let mut retired = 0;
    for (index, publication) in state.published.iter().enumerate() {
        if ancestor(
            &repo,
            &publication.source_revision,
            &baseline.source_revision,
        )? {
            retired = index + 1;
        }
    }
    let common = if retired == 0 {
        &state.baseline.contents
    } else {
        &state.published[retired - 1].reference
    };
    let observed = live()?;
    let git = Git::new(parent)?;
    let observed_changes = git.changes(&state.reference, &observed)?;
    let active_local: Vec<_> = state
        .ignored
        .iter()
        .filter(|decision| observed_changes.contains(decision))
        .cloned()
        .collect();
    // Only currently matching exact-local decisions override incoming content.
    // A later changed local value takes the ordinary conflict/review path.
    let incoming_local = reanchor(
        &git,
        &state.reference,
        &baseline.contents,
        &active_local,
        true,
    )?;
    let adjusted_incoming = git.apply(&baseline.contents, &incoming_local)?;
    let common_local = reanchor(&git, &state.reference, common, &active_local, true)?;
    let adjusted_common = git.apply(common, &common_local)?;
    let desired = git.merge(&observed, &adjusted_common, &adjusted_incoming)?;
    let mut future = Vec::new();
    for publication in &state.published[retired..] {
        future.push(Publication {
            source_revision: publication.source_revision.clone(),
            reference: git.merge(&baseline.contents, common, &publication.reference)?,
        });
    }
    let reference = future
        .last()
        .map_or_else(|| baseline.contents.clone(), |p| p.reference.clone());
    let selected = reanchor(&git, &state.reference, &reference, &state.selected, false)?;
    let ignored = reanchor(&git, &state.reference, &reference, &state.ignored, true)?;
    let after = State {
        schema_version: 1,
        instance: state.instance.clone(),
        baseline,
        reference,
        selected,
        ignored,
        published: future,
    };
    after.validate(&state.instance)?;
    let state_hash = hash(&serde_json::to_vec(state)?);
    let live_hash = hash(observed.as_bytes());
    let desired_hash = hash(desired.as_bytes());
    let plan_id = hash(&serde_json::to_vec(&(
        &state_hash,
        &live_hash,
        &desired_hash,
        &after,
    ))?);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema_version":1,"plan_id":plan_id,"source_only_preview":true,
            "accepted_baseline_revision":state.baseline.source_revision,
            "candidate_baseline_revision":after.baseline.source_revision,
            "source_path":after.baseline.source_path,"observed_sha256":live_hash,"desired_sha256":desired_hash,
            "retired_publications":retired,"pending_publications":after.published.len(),
            "current_changes":observed_changes,"proposed_changes":git.changes(&state.reference, &desired)?,
            "selection_after":after.selected,"local_only_after":after.ignored,
            "live_file_changed":false,"review_state_changed":false,"installed_image_checked":false,
            "native_validation_performed":false,"apply_available":false
        }))?
    );
    Ok(())
}
