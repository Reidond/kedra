//! Journaled discard of one exact text change; accepted B and S/I/P stay intact.
use super::{Git, RECORD, Result, State, TextCommand, content, hash, hex, live, live_path};
use crate::home::Recovery;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use sysroot_helper::{
    replace_file::{Directory, Receipt},
    storage::Store,
};

const JOURNAL: &str = "niri-text-activation";
const FILE: &str = "config.kdl";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Prepared,
    Published,
    Validated,
    Aborting,
    Completed,
    Aborted,
    KeptCurrent,
}
impl Phase {
    fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Aborted | Self::KeptCurrent)
    }
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    state_sha256: String,
    plan_id: String,
    observed_sha256: String,
    desired_sha256: String,
    receipt: Receipt,
    phase: Phase,
}
struct Plan {
    id: String,
    observed: String,
    desired: String,
    restored: String,
}
fn state_hash(state: &State) -> Result<String> {
    let mut state = state.clone();
    state.pending_activation = None;
    Ok(hash(&serde_json::to_vec(&state)?))
}
fn token() -> Result<String> {
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(hash(&bytes)[..32].to_owned())
}
fn plan(state: &State, parent: &Path, id: &str) -> Result<Plan> {
    if state.pending_activation.is_some() {
        return Err("niri recovery must finish before planning".into());
    }
    let observed = live()?;
    let changes = Git::new(parent)?.changes(&state.reference, &observed)?;
    let change = changes
        .iter()
        .find(|c| c.id == id)
        .ok_or("change is stale; review file status again")?;
    let selections: Vec<_> = state
        .selected
        .iter()
        .filter(|s| s.overlaps(change))
        .collect();
    let restored = match selections.as_slice() {
        [] => change.before.clone(),
        [selected] if selected.at == change.at && selected.before == change.before => {
            selected.after.clone()
        }
        _ => {
            return Err(
                "discard crosses a pinned selection boundary; review the selection first".into(),
            );
        }
    };
    let shift: isize = changes
        .iter()
        .take_while(|c| c.id != id)
        .map(|c| c.after.lines().count() as isize - c.before.lines().count() as isize)
        .sum();
    let at = change
        .at
        .checked_add_signed(shift)
        .ok_or("discard range overflow")?;
    let end = at
        .checked_add(change.after.lines().count())
        .ok_or("discard range overflow")?;
    let lines: Vec<_> = observed.split_inclusive('\n').collect();
    if lines
        .get(at..end)
        .ok_or("discard range is missing")?
        .concat()
        != change.after
    {
        return Err("discard precondition differs from the displayed change".into());
    }
    // Exact replacement of the reviewed range; no relocation or whole-live Git object.
    let desired = content(
        format!(
            "{}{}{}",
            lines[..at].concat(),
            restored,
            lines[end..].concat()
        )
        .as_bytes(),
    )?;
    let id = hash(&serde_json::to_vec(&(
        state_hash(state)?,
        id,
        hash(observed.as_bytes()),
        hash(desired.as_bytes()),
        "activate-managed-niri-file",
    ))?);
    Ok(Plan {
        id,
        observed,
        desired,
        restored,
    })
}
fn validate(path: &Path) -> Result<()> {
    let status = Command::new("/usr/bin/timeout")
        .args([
            "--kill-after=2s",
            "20s",
            "/usr/bin/niri",
            "validate",
            "--config",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(
            "native niri validation failed; configuration details were not captured".into(),
        );
    }
    Ok(())
}
fn line(reader: &mut BufReader<UnixStream>) -> Result<serde_json::Value> {
    let mut bytes = Vec::new();
    reader.take(1_048_577).read_until(b'\n', &mut bytes)?;
    if bytes.len() > 1_048_576 || !bytes.ends_with(b"\n") {
        return Err("niri IPC response exceeds its bound or is incomplete".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "niri IPC response is malformed".into())
}
fn request(reader: &mut BufReader<UnixStream>, value: serde_json::Value) -> Result<()> {
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.push(b'\n');
    reader.get_mut().write_all(&bytes)?;
    if line(reader)? != serde_json::json!({"Ok":"Handled"}) {
        return Err("niri refused the configuration request".into());
    }
    Ok(())
}
fn connect() -> Result<BufReader<UnixStream>> {
    let socket =
        std::env::var_os("NIRI_SOCKET").ok_or("run this command inside the niri session")?;
    let stream = UnixStream::connect(socket)?;
    let peer = rustix::net::sockopt::socket_peercred(&stream)?;
    if peer.uid != rustix::process::geteuid()
        || std::fs::read_link(format!("/proc/{}/exe", peer.pid.as_raw()))?
            != Path::new("/usr/bin/niri")
    {
        return Err("NIRI_SOCKET does not belong to this user's installed niri process".into());
    }
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    Ok(BufReader::new(stream))
}
fn reload(path: &Path) -> Result<()> {
    validate(path)?;
    let mut reader = connect()?;
    request(&mut reader, serde_json::json!("EventStream"))?;
    // The initial event reports the previous attempt, not our requested reload.
    config_event(&mut reader)?;
    let mut action = connect()?;
    request(
        &mut action,
        serde_json::json!({"Action":{"LoadConfigFile":{"path":path}}}),
    )?;
    if config_event(&mut reader)? {
        return Err("niri reported a failed reload; use home file recover".into());
    }
    Ok(())
}
fn config_event(reader: &mut BufReader<UnixStream>) -> Result<bool> {
    for _ in 0..64 {
        let event = line(reader)?;
        if let Some(failed) = event
            .get("ConfigLoaded")
            .and_then(|v| v.get("failed"))
            .and_then(serde_json::Value::as_bool)
        {
            return Ok(failed);
        }
    }
    Err("niri did not report configuration load status within the event bound".into())
}
fn save(store: &mut Store, revision: u64, journal: &Journal) -> Result<u64> {
    Ok(store
        .compare_exchange(JOURNAL, Some(revision), &serde_json::to_vec(journal)?)?
        .revision)
}
fn pending(store: &Store, state: &State) -> Result<(u64, Journal)> {
    let record = store
        .read(JOURNAL)?
        .ok_or("niri journal is missing; preserve the state")?;
    let journal: Journal =
        serde_json::from_slice(&record.bytes).map_err(|_| "niri journal is malformed")?;
    if journal.schema_version != 1
        || journal.phase.terminal()
        || state.pending_activation.as_deref() != Some(&journal.receipt.token)
        || journal.state_sha256 != state_hash(state)?
        || [
            &journal.plan_id,
            &journal.observed_sha256,
            &journal.desired_sha256,
        ]
        .iter()
        .any(|h| !hex(h, 64))
    {
        return Err("niri journal disagrees with reserved review state".into());
    }
    Ok((record.revision, journal))
}
fn conclude(
    store: &mut Store,
    state: &State,
    jr: u64,
    journal: &mut Journal,
    phase: Phase,
) -> Result<()> {
    let record = store.read(RECORD)?.ok_or("niri review state disappeared")?;
    if record.bytes != serde_json::to_vec(state)? {
        return Err("reserved niri state changed".into());
    }
    let mut after = state.clone();
    after.pending_activation = None;
    journal.phase = phase;
    store.compare_exchange_batch(&[
        (RECORD, Some(record.revision), &serde_json::to_vec(&after)?),
        (JOURNAL, Some(jr), &serde_json::to_vec(journal)?),
    ])?;
    Ok(())
}
fn resume(store: &mut Store, state: &State, directory: &Directory, path: &Path) -> Result<()> {
    let (mut jr, mut journal) = pending(store, state)?;
    if journal.phase == Phase::Aborting {
        return Err("abort is pending; use home file recover abort".into());
    }
    if journal.phase == Phase::Prepared {
        if !directory.is_published(&journal.receipt)? {
            directory.publish(&journal.receipt)?;
        }
        journal.phase = Phase::Published;
        jr = save(store, jr, &journal)?;
    }
    if hash(live()?.as_bytes()) != journal.desired_sha256 {
        return Err("live niri file changed; checkpoint retained for recovery".into());
    }
    reload(path)?;
    if hash(live()?.as_bytes()) != journal.desired_sha256 {
        return Err("niri file changed during reload; checkpoint retained".into());
    }
    journal.phase = Phase::Validated;
    jr = save(store, jr, &journal)?;
    directory.finish_validated(&journal.receipt)?;
    conclude(store, state, jr, &mut journal, Phase::Completed)
}
pub(super) fn handles(command: &TextCommand) -> bool {
    matches!(
        command,
        TextCommand::DiscardPlan { .. } | TextCommand::Discard { .. } | TextCommand::Recover { .. }
    )
}
pub(super) fn run(
    store: &mut Store,
    parent: &Path,
    state: &State,
    command: &TextCommand,
) -> Result<()> {
    if let TextCommand::Recover { action: None, .. } = command {
        let journal = store
            .read(JOURNAL)?
            .map(|r| serde_json::from_slice::<serde_json::Value>(&r.bytes))
            .transpose()?;
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"pending_activation":state.pending_activation,"journal":journal,"live_file_changed":false})
            )?
        );
        return Ok(());
    }
    let path = live_path()?;
    let directory = Directory::open(path.parent().ok_or("niri parent is missing")?)?;
    let _coordination = directory.coordinate()?;
    match command {
        TextCommand::DiscardPlan { change } => {
            let plan = plan(state, parent, change)?;
            directory.validate_bytes(&token()?, plan.desired.as_bytes(), validate)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"plan_id":plan.id,"change":change,"restored_text":plan.restored,
                "observed_sha256":hash(plan.observed.as_bytes()),"desired_sha256":hash(plan.desired.as_bytes()),
                "will_activate_managed_file":true,"native_validation_performed":true,"live_file_changed":false,"review_state_changed":false})
                )?
            );
            return Ok(());
        }
        TextCommand::Discard {
            change,
            plan: approved,
            activate_managed_file: true,
        } => {
            let plan = plan(state, parent, change)?;
            if plan.id != *approved {
                return Err("discard plan is stale; review a new discard-plan".into());
            }
            let _ = connect()?;
            directory.validate_bytes(&token()?, plan.desired.as_bytes(), validate)?;
            if plan.observed == plan.desired {
                reload(&path)?;
                if live()? != plan.desired {
                    return Err("niri file changed during reload".into());
                }
            } else {
                let receipt = directory.prepare(
                    FILE,
                    &token()?,
                    Some(plan.observed.as_bytes()),
                    plan.desired.as_bytes(),
                )?;
                let journal = Journal {
                    schema_version: 1,
                    state_sha256: state_hash(state)?,
                    plan_id: plan.id,
                    observed_sha256: hash(plan.observed.as_bytes()),
                    desired_sha256: hash(plan.desired.as_bytes()),
                    receipt,
                    phase: Phase::Prepared,
                };
                let mut reserved = state.clone();
                reserved.pending_activation = Some(journal.receipt.token.clone());
                let revision = store
                    .read(RECORD)?
                    .ok_or("niri state disappeared")?
                    .revision;
                let prior = store.read(JOURNAL)?.map(|r| r.revision);
                if let Err(error) = store.compare_exchange_batch(&[
                    (RECORD, Some(revision), &serde_json::to_vec(&reserved)?),
                    (JOURNAL, prior, &serde_json::to_vec(&journal)?),
                ]) {
                    directory.abort(&journal.receipt)?;
                    return Err(error.into());
                }
                resume(store, &reserved, &directory, &path)?;
            }
        }
        TextCommand::Recover {
            action: Some(action),
            activate_managed_file: true,
        } => {
            let _ = connect()?;
            if matches!(action, Recovery::Resume) {
                resume(store, state, &directory, &path)?;
            } else {
                let (mut jr, mut journal) = pending(store, state)?;
                if matches!(action, Recovery::Abort) {
                    if journal.phase == Phase::Validated {
                        return Err("validated cleanup has begun; resume or keep-current".into());
                    }
                    journal.phase = Phase::Aborting;
                    jr = save(store, jr, &journal)?;
                    directory.abort(&journal.receipt)?;
                    if hash(live()?.as_bytes()) != journal.observed_sha256 {
                        return Err("aborted niri file differs from the checkpoint".into());
                    }
                }
                let observed = live()?;
                reload(&path)?;
                if live()? != observed {
                    return Err("niri file changed during recovery reload".into());
                }
                let phase = if matches!(action, Recovery::Abort) {
                    Phase::Aborted
                } else {
                    Phase::KeptCurrent
                };
                conclude(store, state, jr, &mut journal, phase)?;
            }
        }
        _ => {
            return Err(
                "explicit --activate-managed-file is required to load this file into the session"
                    .into(),
            );
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"completed":true,"managed_file_loaded":true,"accepted_baseline_changed":false,"selection_and_local_policy_preserved":true})
        )?
    );
    Ok(())
}
