//! `sysroot home update` in the real nested session: the account starts with
//! its baseline recorded, a local edit and an image change merge, niri loads the
//! merged file by itself, Noctalia restarts with it, a failing check and a
//! conflict both leave the live file untouched, and resolution is explicit.

use std::time::Duration;

use kedra_container_tests::Result;
use kedra_container_tests::docker::Exec;
use kedra_container_tests::scenario::Context;
use serde_json::Value;

use crate::native::{Desk, ensure, fail};

const NIRI: &str = ".config/niri/config.kdl";
const NOCTALIA: &str = ".config/noctalia/config.toml";
const PALETTE: &str = ".config/noctalia/palettes/Adwaita.json";
const BASELINE: &str = "/usr/share/sysroot/home/default";
const CLOCK: &str = "format = \"{:%a %b %-d  %H:%M}\"\n";
const THICKNESS: &str = "thickness = 36\n";

struct Home<'d, 'a, 'b> {
    desk: &'d Desk<'a, 'b>,
    home: String,
}

impl Home<'_, '_, '_> {
    fn docker_id(&self) -> &str {
        &self.desk.context.environment.id
    }

    fn read(&self, path: &str) -> Result<String> {
        let bytes = self.desk.context.docker.read_file(self.docker_id(), path)?;
        String::from_utf8(bytes).map_err(|_| fail(format!("{path} is not UTF-8")))
    }

    fn live(&self, relative: &str) -> Result<String> {
        self.read(&format!("{}/{relative}", self.home))
    }

    fn write(&self, path: &str, text: &str, owner: &str) -> Result<()> {
        self.desk
            .context
            .docker
            .write_file(self.docker_id(), path, text.as_bytes(), owner, "0644")
    }

    /// Replace one exact occurrence, failing when the text is not there once.
    fn edit(&self, path: &str, owner: &str, before: &str, after: &str) -> Result<()> {
        let text = self.read(path)?;
        ensure(text.matches(before).count() == 1, || {
            format!("{path} does not contain {before:?} exactly once")
        })?;
        self.write(path, &text.replacen(before, after, 1), owner)
    }

    fn edit_live(&self, relative: &str, before: &str, after: &str) -> Result<()> {
        let user = self.desk.session.user.name.clone();
        self.edit(&format!("{}/{relative}", self.home), &user, before, after)
    }

    /// Change the installed baseline as a new image would.
    fn edit_baseline(&self, relative: &str, before: &str, after: &str) -> Result<()> {
        self.edit(&format!("{BASELINE}/{relative}"), "root", before, after)
    }

    /// `sysroot home update --json`: its exit status and each file's result.
    fn update(&self) -> Result<(bool, Value)> {
        let output = self
            .desk
            .try_run(&["sysroot", "home", "update", "--json"])?;
        let report: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            fail(format!(
                "update did not print JSON ({error}): {}{}",
                output.stdout_text(),
                output.stderr_text()
            ))
        })?;
        Ok((output.exit == 0, report))
    }

    fn workspaces(&self) -> Result<Vec<String>> {
        let value = self.desk.json(&["niri", "msg", "--json", "workspaces"])?;
        Ok(value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|workspace| workspace["name"].as_str().map(str::to_owned))
            .collect())
    }

    fn wait_workspace(&self, name: &str, present: bool) -> Result<()> {
        self.desk.wait(
            &format!("niri workspace {name} present={present}"),
            Duration::from_secs(20),
            || Ok(self.workspaces()?.iter().any(|item| item == name) == present),
        )
    }

    fn noctalia_pid(&self) -> Result<String> {
        let output = self.desk.run(&[
            "systemctl",
            "--user",
            "show",
            "kedra-noctalia.service",
            "--property=MainPID",
            "--value",
        ])?;
        Ok(output.stdout_text().trim().to_owned())
    }
}

fn result<'v>(report: &'v Value, path: &str) -> &'v str {
    report["files"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|file| file["path"] == path)
        .and_then(|file| file["result"].as_str())
        .unwrap_or_default()
}

fn expect(report: &Value, expected: &[(&str, &str)]) -> Result<()> {
    for (path, wanted) in expected {
        ensure(result(report, path) == *wanted, || {
            format!("{path} should be {wanted}: {report}")
        })?;
    }
    Ok(())
}

fn recorded_at_creation(home: &Home<'_, '_, '_>) -> Result<()> {
    let status = home.desk.json(&["sysroot", "home", "status", "--json"])?;
    let files = status["files"].as_array().cloned().unwrap_or_default();
    ensure(files.len() == 3, || {
        format!("unexpected managed files: {status}")
    })?;
    for file in files {
        ensure(
            file["applied"] == "current"
                && file["live_matches_baseline"] == true
                && file["conflict"] == false,
            || format!("a new account should start updated: {file}"),
        )?;
    }
    Ok(())
}

/// A local edit and a baseline change in other lines merge in both files.
fn merge_and_reload(home: &Home<'_, '_, '_>) -> Result<()> {
    let original = home.live(NIRI)?;
    home.write(
        &format!("{}/{NIRI}", home.home),
        &format!("{original}workspace \"kedra-local\"\n"),
        &home.desk.session.user.name,
    )?;
    home.wait_workspace("kedra-local", true)?;
    home.edit_live(NOCTALIA, CLOCK, "format = \"{:%H:%M}\"\n")?;
    let baseline = home.read(&format!("{BASELINE}/{NIRI}"))?;
    home.write(
        &format!("{BASELINE}/{NIRI}"),
        &format!("workspace \"kedra-image\"\n{baseline}"),
        "root",
    )?;
    home.edit_baseline(NOCTALIA, THICKNESS, "thickness = 40\n")?;
    let before = home.noctalia_pid()?;

    let (success, report) = home.update()?;
    ensure(success, || format!("update failed: {report}"))?;
    expect(
        &report,
        &[
            (NIRI, "merged"),
            (NOCTALIA, "merged"),
            (PALETTE, "up_to_date"),
        ],
    )?;
    let niri = home.live(NIRI)?;
    ensure(
        niri.starts_with("workspace \"kedra-image\"\n")
            && niri.ends_with("workspace \"kedra-local\"\n"),
        || format!("niri merge lost a side:\n{niri}"),
    )?;
    home.wait_workspace("kedra-image", true)?;
    home.wait_workspace("kedra-local", true)?;
    let noctalia = home.live(NOCTALIA)?;
    ensure(
        noctalia.contains("thickness = 40\n") && noctalia.contains("format = \"{:%H:%M}\"\n"),
        || format!("Noctalia merge lost a side:\n{noctalia}"),
    )?;
    let after = home.noctalia_pid()?;
    ensure(after != before && after != "0", || {
        format!("Noctalia did not restart (MainPID {before} -> {after})")
    })?;
    home.desk.wait(
        "Noctalia IPC after the restart",
        Duration::from_secs(30),
        || {
            Ok(home
                .desk
                .try_run(&["noctalia", "msg", "log-level-status"])?
                .exit
                == 0)
        },
    )
}

/// An invalid baseline fails niri's own validator before anything is written.
fn check_refusal(home: &Home<'_, '_, '_>) -> Result<()> {
    let live = home.live(NIRI)?;
    let baseline = home.read(&format!("{BASELINE}/{NIRI}"))?;
    // Away from the local edit at the end, so the merge itself is clean.
    home.edit_baseline(
        NIRI,
        "workspace \"kedra-image\"\n",
        "workspace \"kedra-image\"\nkedra-not-a-node {\n",
    )?;
    let (success, report) = home.update()?;
    ensure(!success, || "an invalid candidate was accepted".into())?;
    expect(&report, &[(NIRI, "check_failed"), (NOCTALIA, "up_to_date")])?;
    ensure(home.live(NIRI)? == live, || {
        "a failing check changed the live niri file".into()
    })?;
    home.write(&format!("{BASELINE}/{NIRI}"), &baseline, "root")?;
    let (success, report) = home.update()?;
    ensure(success, || format!("update failed: {report}"))?;
    expect(&report, &[(NIRI, "up_to_date")])
}

/// Both sides change one line: the live file and niri stay as they are, the
/// marked merge goes beside the file and deleting it marks the conflict resolved.
fn conflict_and_resolution(home: &Home<'_, '_, '_>) -> Result<()> {
    home.edit_live(
        NIRI,
        "workspace \"kedra-image\"\n",
        "workspace \"kedra-mine\"\n",
    )?;
    home.wait_workspace("kedra-mine", true)?;
    home.edit_baseline(
        NIRI,
        "workspace \"kedra-image\"\n",
        "workspace \"kedra-theirs\"\n",
    )?;
    let live = home.live(NIRI)?;
    let (success, report) = home.update()?;
    ensure(!success, || "a conflicting update reported success".into())?;
    expect(&report, &[(NIRI, "conflict")])?;
    ensure(home.live(NIRI)? == live, || {
        "a conflict changed the live niri file".into()
    })?;
    let conflict = home.live(&format!("{NIRI}.kedra-new"))?;
    ensure(
        conflict.contains("<<<<<<< live\nworkspace \"kedra-mine\"\n")
            && conflict.contains(">>>>>>> new baseline\n"),
        || format!("unexpected conflict file:\n{conflict}"),
    )?;
    std::thread::sleep(Duration::from_secs(2));
    ensure(
        !home.workspaces()?.iter().any(|name| name == "kedra-theirs"),
        || "niri loaded the conflicting baseline".into(),
    )?;
    let (_, report) = home.update()?;
    expect(&report, &[(NIRI, "unresolved")])?;

    home.edit_live(
        NIRI,
        "workspace \"kedra-mine\"\n",
        "workspace \"kedra-theirs\"\n",
    )?;
    let conflict = format!("{}/{NIRI}.kedra-new", home.home);
    home.desk.run(&["rm", "--", &conflict])?;
    let (success, report) = home.update()?;
    ensure(success, || format!("update failed: {report}"))?;
    expect(&report, &[(NIRI, "resolved")])?;
    home.wait_workspace("kedra-theirs", true)?;
    home.wait_workspace("kedra-mine", false)?;
    let status = home.desk.json(&["sysroot", "home", "status", "--json"])?;
    for file in status["files"].as_array().into_iter().flatten() {
        ensure(
            file["applied"] == "current" && file["conflict"] == false,
            || format!("unexpected state after resolution: {file}"),
        )?;
    }
    Ok(())
}

pub fn home_files_update(context: &Context<'_>) -> Result<()> {
    let desk = Desk::new(context)?;
    let home = Home {
        desk: &desk,
        home: desk.session.user.home.clone(),
    };
    recorded_at_creation(&home)?;
    merge_and_reload(&home)?;
    check_refusal(&home)?;
    conflict_and_resolution(&home)?;
    // Only the declared files were touched; the state directory stays private.
    let state = format!("{}/.local/state/kedra/home", home.home);
    let mode = context.docker.run(
        &context.environment.id,
        &Exec::new(["stat", "-c", "%a", state.as_str()]),
    )?;
    ensure(mode.stdout_text().trim() == "700", || {
        format!("{state} has mode {}", mode.stdout_text().trim())
    })
}
