use super::{Action, Plan, Result, hash, patch};
use crate::home::{
    Command, Recovery,
    linux::{self as capture, RECORD, load},
};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::Path;
use sysroot_core::noctalia::{Settings, State};
use sysroot_helper::{
    replace_file::{Directory, Receipt},
    storage::Store,
};

mod native;
const JOURNAL: &str = "noctalia-activation";
const FILE: &str = "settings.toml";

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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    plan: Plan,
    before: String,
    receipt: Receipt,
    phase: Phase,
}
trait Application {
    fn observe(&self) -> Result<Settings>;
    fn stop(&self) -> Result<()>;
    fn start(&self) -> Result<()>;
}
fn save_journal(store: &mut Store, revision: u64, journal: &Journal) -> Result<u64> {
    Ok(store
        .compare_exchange(JOURNAL, Some(revision), &serde_json::to_vec(journal)?)?
        .revision)
}
fn pending(store: &Store, instance: &str) -> Result<(u64, State, u64, Journal, State)> {
    let (revision, state) = load(store, instance)?;
    let record = store
        .read(JOURNAL)?
        .ok_or("activation journal is missing; preserve the store")?;
    let journal: Journal =
        serde_json::from_slice(&record.bytes).map_err(|_| "activation journal is malformed")?;
    let before = State::from_bytes(journal.before.as_bytes(), instance)?;
    let mut reserved = before.clone();
    reserved.reserve_activation(&journal.receipt.token)?;
    if journal.schema_version != 1
        || reserved != state
        || journal.plan != Plan::new(&before, journal.plan.action.clone())?
        || matches!(
            journal.phase,
            Phase::Completed | Phase::Aborted | Phase::KeptCurrent
        )
    {
        return Err(
            "activation journal and review reservation disagree; preserve the store".into(),
        );
    }
    Ok((revision, state, record.revision, journal, before))
}

/// Validate pending reservation consistency without opening native files or apps.
pub(in crate::home) fn assessment_pending(
    store: &Store,
    instance: &str,
    state: Option<&State>,
) -> Result<bool> {
    let Some(state) = state else {
        if store.read(JOURNAL)?.is_some() {
            return Err(
                "Noctalia journal exists without an adopted state; preserve the store".into(),
            );
        }
        return Ok(false);
    };
    if state.pending_activation().is_some() {
        pending(store, instance)?;
        return Ok(true);
    }
    if let Some(record) = store.read(JOURNAL)? {
        let journal: Journal =
            serde_json::from_slice(&record.bytes).map_err(|_| "activation journal is malformed")?;
        let before = State::from_bytes(journal.before.as_bytes(), instance)?;
        if journal.schema_version != 1
            || journal.plan != Plan::new(&before, journal.plan.action.clone())?
            || !matches!(
                journal.phase,
                Phase::Completed | Phase::Aborted | Phase::KeptCurrent
            )
        {
            return Err("activation journal has no matching reservation".into());
        }
    }
    Ok(false)
}
fn conclude(
    store: &mut Store,
    revision: u64,
    journal_revision: u64,
    journal: &mut Journal,
    after: &State,
    phase: Phase,
) -> Result<()> {
    journal.phase = phase;
    store.compare_exchange_batch(&[
        (RECORD, Some(revision), &after.to_bytes()?),
        (
            JOURNAL,
            Some(journal_revision),
            &serde_json::to_vec(journal)?,
        ),
    ])?;
    Ok(())
}
fn token() -> Result<String> {
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(hash(&bytes)[..32].to_owned())
}

fn apply(
    store: &mut Store,
    instance: &str,
    directory: &Directory,
    app: &impl Application,
    plan: Plan,
    approved: &str,
) -> Result<()> {
    let (revision, before) = load(store, instance)?;
    if plan.id()? != approved
        || Plan::new(&before, plan.action.clone())? != plan
        || app.observe()? != plan.observed
    {
        return Err("settings or plan changed; review a new home plan".into());
    }
    if plan.desired == plan.observed {
        let after = plan.finish(&before, app.observe()?)?;
        store.compare_exchange(RECORD, Some(revision), &after.to_bytes()?)?;
        return Ok(());
    }
    let prior = store.read(JOURNAL)?.map(|r| r.revision);
    app.stop()?;
    let preparation = (|| -> Result<Receipt> {
        if app.observe()? != plan.observed {
            return Err("settings changed while stopping Noctalia; review a new plan".into());
        }
        let original = directory.read(FILE)?;
        let candidate = patch(
            original.as_deref().unwrap_or_default(),
            plan.observed,
            plan.desired,
        )?;
        directory.prepare(FILE, &token()?, original.as_deref(), &candidate)
    })();
    let receipt = match preparation {
        Ok(receipt) => receipt,
        Err(error) => {
            app.start()?;
            return Err(error);
        }
    };
    let mut state = before.clone();
    state.reserve_activation(&receipt.token)?;
    let journal = Journal {
        schema_version: 1,
        plan,
        before: String::from_utf8(before.to_bytes()?)?,
        receipt,
        phase: Phase::Prepared,
    };
    if let Err(error) = store.compare_exchange_batch(&[
        (RECORD, Some(revision), &state.to_bytes()?),
        (JOURNAL, prior, &serde_json::to_vec(&journal)?),
    ]) {
        directory.abort(&journal.receipt)?;
        app.start()?;
        return Err(error.into());
    }
    // Every later failure retains the reservation/checkpoint for explicit recovery.
    resume(store, instance, directory, app)
}

fn resume(
    store: &mut Store,
    instance: &str,
    directory: &Directory,
    app: &impl Application,
) -> Result<()> {
    let (revision, _, mut jr, mut journal, before) = pending(store, instance)?;
    if journal.phase == Phase::Aborting {
        return Err("abort is already in progress; use home recover abort".into());
    }
    app.stop()?;
    if journal.phase == Phase::Prepared {
        if !directory.is_published(&journal.receipt)? {
            if app.observe()? != journal.plan.observed {
                return Err(
                    "live settings changed before publication; abort or keep-current after review"
                        .into(),
                );
            }
            directory.publish(&journal.receipt)?;
        }
        journal.phase = Phase::Published;
        jr = save_journal(store, jr, &journal)?;
    }
    if app.observe()? != journal.plan.desired {
        return Err(
            "live settings differ from the pending plan; recovery preserves both versions".into(),
        );
    }
    app.start()?;
    if app.observe()? != journal.plan.desired {
        return Err("Noctalia read-back differs after restart; use home recover".into());
    }
    journal.phase = Phase::Validated;
    jr = save_journal(store, jr, &journal)?;
    directory.finish_validated(&journal.receipt)?;
    let after = journal.plan.finish(&before, app.observe()?)?;
    conclude(store, revision, jr, &mut journal, &after, Phase::Completed)
}
fn recover(
    store: &mut Store,
    instance: &str,
    directory: &Directory,
    app: &impl Application,
    action: Recovery,
) -> Result<()> {
    if matches!(action, Recovery::Resume) {
        return resume(store, instance, directory, app);
    }
    let (revision, _, mut jr, mut journal, mut before) = pending(store, instance)?;
    if matches!(action, Recovery::KeepCurrent) {
        let observed = app.observe()?;
        app.start()?;
        if app.observe()? != observed {
            return Err(
                "Noctalia changed during recovery startup; review keep-current again".into(),
            );
        }
        before.capture(observed)?;
        // Deliberately leave any private checkpoint intact. No native file writes.
        return conclude(
            store,
            revision,
            jr,
            &mut journal,
            &before,
            Phase::KeptCurrent,
        );
    }
    app.stop()?;
    if journal.phase == Phase::Validated {
        return Err(
            "validated activation may have discarded its old checkpoint; resume or keep-current"
                .into(),
        );
    }
    journal.phase = Phase::Aborting;
    jr = save_journal(store, jr, &journal)?;
    directory.abort(&journal.receipt)?;
    app.start()?;
    if app.observe()? != journal.plan.observed {
        return Err(
            "restored file has different effective settings; review recovery before continuing"
                .into(),
        );
    }
    conclude(store, revision, jr, &mut journal, &before, Phase::Aborted)
}

pub(in crate::home) fn handles(command: &Command) -> bool {
    matches!(
        command,
        Command::Plan { .. }
            | Command::Apply { .. }
            | Command::Discard { .. }
            | Command::Recover { .. }
    )
}
pub(in crate::home) fn run(store: &mut Store, instance: &str, command: &Command) -> Result<()> {
    if let Command::Recover { action: None } = command {
        let (_, state) = load(store, instance)?;
        let journal = store
            .read(JOURNAL)?
            .map(|r| serde_json::from_slice::<Journal>(&r.bytes))
            .transpose()
            .map_err(|_| "activation journal is malformed")?;
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"pending": state.pending_activation(),
            "journal": journal, "native_file_contents_stored": false})
            )?
        );
        return Ok(());
    }
    let app = native::Native::open()?;
    let directory = Directory::open(&app.directory)?;
    let _application_lock = directory.coordinate()?;
    if let Command::Recover {
        action: Some(action),
    } = command
    {
        let (_, _, _, journal, _) = pending(store, instance)?;
        if matches!(action, Recovery::Resume)
            && let Action::Activate { baseline } = &journal.plan.action
            && *baseline != capture::installed_baseline(Path::new("/"), 0)?
        {
            return Err(
                "the installed image changed; abort or keep-current, then review a new plan".into(),
            );
        }
        recover(store, instance, &directory, &app, *action)?;
    } else {
        let (revision, mut state) = load(store, instance)?;
        let action = match command {
            Command::Plan { discard: Some(key) } | Command::Discard { key, .. } => {
                Action::Discard { key: (*key).into() }
            }
            _ => Action::Activate {
                baseline: capture::installed_baseline(Path::new("/"), 0)?,
            },
        };
        if matches!(command, Command::Plan { .. }) {
            state.capture(app.observe()?)?;
            let plan = Plan::new(&state, action)?;
            store.compare_exchange(RECORD, Some(revision), &state.to_bytes()?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"plan_id": plan.id()?, "plan": plan,
                "activation_performed": false})
                )?
            );
            return Ok(());
        }
        let plan = Plan::new(&state, action)?;
        let approved = match command {
            Command::Apply { plan } | Command::Discard { plan, .. } => plan,
            _ => return Err("unsupported activation operation".into()),
        };
        apply(store, instance, &directory, &app, plan, approved)?;
    }
    let (_, state) = load(store, instance)?;
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"accepted_baseline": state.accepted_baseline(),
        "fields": state.rows()?, "pending": state.pending_activation(), "operation_completed": true})
        )?
    );
    Ok(())
}
