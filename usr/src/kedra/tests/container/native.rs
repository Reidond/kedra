//! Native tests: Rust code using the same environment and fixture API as
//! scenarios, for flows that need control flow or input sequencing.

use std::time::{Duration, Instant};

use kedra_container_tests::docker::{Exec, Output};
use kedra_container_tests::scenario::{Context, Fixture, Profile};
use kedra_container_tests::session::{self, Session};
use kedra_container_tests::{Error, Result};
use serde_json::Value;

pub struct NativeTest {
    pub name: &'static str,
    pub profile: Profile,
    pub fixtures: &'static [Fixture],
    /// Image targets this test applies to; all when empty.
    pub targets: &'static [&'static str],
    pub run: fn(&Context<'_>) -> Result<()>,
}

pub const TESTS: &[NativeTest] = &[
    NativeTest {
        name: "home_review_cycle",
        profile: Profile::Desktop,
        fixtures: &[],
        targets: &[],
        run: home_review_cycle,
    },
    NativeTest {
        name: "toolkit_file_choosers",
        profile: Profile::Desktop,
        fixtures: &[],
        targets: &[],
        run: toolkit_file_choosers,
    },
];

fn fail(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn ensure(condition: bool, message: impl FnOnce() -> String) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(fail(message()))
    }
}

#[derive(Clone, Copy)]
enum Toolkit {
    Gtk3,
    Gtk4,
}

/// Session-scoped helpers for one native test.
struct Desk<'a, 'b> {
    context: &'a Context<'b>,
    session: &'a Session,
}

impl<'a, 'b> Desk<'a, 'b> {
    fn new(context: &'a Context<'b>) -> Result<Self> {
        let session = context
            .session
            .as_ref()
            .ok_or_else(|| fail("native test needs the desktop session"))?;
        Ok(Desk { context, session })
    }

    fn exec(&self, argv: &[&str]) -> Exec {
        self.session.exec(argv.iter().copied())
    }

    fn try_run(&self, argv: &[&str]) -> Result<Output> {
        self.context
            .environment
            .exec(self.context.docker, &self.exec(argv))
    }

    fn run(&self, argv: &[&str]) -> Result<Output> {
        self.context
            .environment
            .run(self.context.docker, &self.exec(argv))
    }

    fn json(&self, argv: &[&str]) -> Result<Value> {
        let output = self.run(argv)?;
        serde_json::from_slice(&output.stdout).map_err(|error| {
            fail(format!(
                "{argv:?} did not print JSON ({error}): {}",
                output.stdout_text()
            ))
        })
    }

    fn noctalia(&self, argv: &[&str]) -> Result<()> {
        let mut command = vec!["noctalia", "msg"];
        command.extend_from_slice(argv);
        self.run(&command).map(|_| ())
    }

    fn wait(
        &self,
        what: &str,
        limit: Duration,
        mut done: impl FnMut() -> Result<bool>,
    ) -> Result<()> {
        let deadline = Instant::now() + limit;
        loop {
            if done()? {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Error::Timeout {
                    what: what.into(),
                    after: limit,
                });
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// Type through niri's virtual keyboard. Measured in the nested lab
    /// session (niri 26.04, 2026-09-27): GTK 3 accepts wlrctl's standard
    /// keymap but not wtype's per-invocation keymap, while GTK 4 needs wtype's
    /// real Return keysym ("\n" from wlrctl is not activation for GTK 4).
    fn type_text(&self, toolkit: Toolkit, text: &str) -> Result<()> {
        match toolkit {
            Toolkit::Gtk3 => self.run(&["wlrctl", "keyboard", "type", text]).map(|_| ()),
            Toolkit::Gtk4 => {
                for (index, line) in text.split('\n').enumerate() {
                    if index > 0 {
                        self.run(&["wtype", "-k", "Return"])?;
                    }
                    if !line.is_empty() {
                        self.run(&["wtype", "--", line])?;
                    }
                }
                Ok(())
            }
        }
    }

    fn screenshot(&self, name: &str) -> Result<()> {
        let path = self
            .context
            .artifacts
            .join("screenshots")
            .join(format!("{name}.png"));
        session::screenshot(
            self.context.docker,
            self.context.environment,
            self.session,
            &path,
        )
        .map(|_| ())
    }
}

fn text<'v>(value: &'v Value, pointer: &str) -> &'v str {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn flag(value: &Value, pointer: &str) -> Option<bool> {
    value.pointer(pointer).and_then(Value::as_bool)
}

/// R03/R04 against the live Noctalia writer: durable field review, local-only
/// and app-owned choices, writer stop/start, stale-plan refusal, native
/// discard with preserved file metadata, killed-CLI recovery and niri line
/// review. Formerly the middle of the desktop VM's check.sh.
fn home_review_cycle(context: &Context<'_>) -> Result<()> {
    let desk = Desk::new(context)?;
    let home = &desk.session.user.home;
    let state = format!("{home}/kedra-noctalia-review");
    let review = |args: &[&str]| -> Result<Value> {
        let mut argv = vec!["sysroot", "home", "--state", state.as_str()];
        argv.extend_from_slice(args);
        desk.json(&argv)
    };
    desk.run(&["sysroot", "home", "--state", &state, "init"])?;
    let live = |review: &dyn Fn(&[&str]) -> Result<Value>| -> Result<String> {
        Ok(text(&review(&["status"])?, "/fields/0/live/value").to_owned())
    };
    let original = live(&review)?;
    let (selected, later) = match original.as_str() {
        "light" => ("dark", "auto"),
        "dark" => ("light", "auto"),
        "auto" => ("light", "dark"),
        other => return Err(fail(format!("unexpected native theme mode {other:?}"))),
    };
    let set_and_observe = |mode: &str| -> Result<()> {
        desk.noctalia(&["theme-mode-set", mode])?;
        desk.wait(
            &format!("theme mode {mode} in the review"),
            Duration::from_secs(20),
            || Ok(live(&review)? == mode),
        )
    };

    // Durable review of a GUI-written value.
    set_and_observe(selected)?;
    review(&["stage", "theme.mode"])?;
    set_and_observe(later)?;
    let status = review(&["status"])?;
    ensure(
        text(&status, "/fields/0/selected/value") == selected,
        || format!("staged value lost: {status}"),
    )?;
    let selection = review(&["selection"])?;
    ensure(
        text(&selection, "/selection/0/after/value") == selected
            && flag(&selection, "/activation_performed") == Some(false)
            && flag(&selection, "/checkout_changed") == Some(false),
        || format!("unexpected selection: {selection}"),
    )?;
    review(&["unstage", "theme.mode"])?;
    let kept = review(&["keep-local", "theme.mode"])?;
    ensure(
        flag(&kept, "/fields/0/local_only") == Some(true)
            && flag(&kept, "/fields/0/visible_change") == Some(false),
        || format!("keep-local did not hide the change: {kept}"),
    )?;
    set_and_observe(selected)?;
    let status = review(&["status"])?;
    ensure(
        flag(&status, "/fields/0/visible_change") == Some(true)
            && flag(&status, "/fields/0/local_only") == Some(false),
        || format!("a new edit after keep-local stayed hidden: {status}"),
    )?;
    let owned = review(&["app-own", "theme.mode"])?;
    ensure(
        flag(&owned, "/fields/0/app_owned") == Some(true)
            && flag(&owned, "/fields/0/visible_change") == Some(false),
        || format!("app-own did not hide the change: {owned}"),
    )?;
    review(&["clear-local", "theme.mode"])?;
    set_and_observe(&original)?;

    // The review reads the persisted value while the writer is stopped.
    set_and_observe(selected)?;
    desk.run(&["systemctl", "--user", "stop", "kedra-noctalia.service"])?;
    let uid = desk.session.user.uid.to_string();
    let lingering = context.environment.exec(
        context.docker,
        &Exec::new(["pgrep", "-u", uid.as_str(), "-x", "noctalia"]),
    )?;
    ensure(lingering.exit == 1, || {
        "Noctalia kept running after its service stopped".into()
    })?;
    ensure(live(&review)? == selected, || {
        "stopped-writer review lost the persisted value".into()
    })?;
    desk.run(&["systemctl", "--user", "start", "kedra-noctalia.service"])?;
    desk.wait(
        "Noctalia IPC after restart",
        Duration::from_secs(30),
        || Ok(desk.try_run(&["noctalia", "msg", "log-level-status"])?.exit == 0),
    )?;
    set_and_observe(&original)?;

    // Stale plans are refused; a current discard keeps native file metadata.
    set_and_observe(selected)?;
    review(&["stage", "theme.mode"])?;
    set_and_observe(later)?;
    let plan = review(&["plan", "--discard", "theme.mode"])?;
    let plan_id = text(&plan, "/plan_id").to_owned();
    ensure(
        !plan_id.is_empty()
            && text(&plan, "/plan/observed/theme_mode") == later
            && text(&plan, "/plan/desired/theme_mode") == selected,
        || format!("unexpected activation plan: {plan}"),
    )?;
    set_and_observe(&original)?;
    let stale = desk.try_run(&[
        "sysroot",
        "home",
        "--state",
        &state,
        "discard",
        "theme.mode",
        "--plan",
        &plan_id,
    ])?;
    ensure(stale.exit != 0, || "a stale home plan was applied".into())?;
    set_and_observe(later)?;
    let settings = format!("{home}/.local/state/noctalia/settings.toml");
    // Ownership and mode; SELinux label preservation needs the VM (no labels here).
    let metadata = || -> Result<String> {
        Ok(desk
            .run(&["stat", "-c", "%u:%g:%a", &settings])?
            .stdout_text())
    };
    let before = metadata()?;
    let discarded = review(&["discard", "theme.mode", "--plan", &plan_id])?;
    ensure(
        flag(&discarded, "/operation_completed") == Some(true)
            && discarded.pointer("/pending") == Some(&Value::Null)
            && text(&discarded, "/fields/0/live/value") == selected
            && text(&discarded, "/fields/0/selected/value") == selected,
        || format!("unexpected discard result: {discarded}"),
    )?;
    ensure(metadata()? == before, || {
        "native discard changed settings.toml ownership or mode".into()
    })?;
    desk.run(&["systemctl", "--user", "is-active", "kedra-noctalia.service"])?;
    let recovered = review(&["recover"])?;
    ensure(
        recovered.pointer("/pending") == Some(&Value::Null)
            && text(&recovered, "/journal/phase") == "completed"
            && flag(&recovered, "/native_file_contents_stored") == Some(false),
        || format!("unexpected recovery state: {recovered}"),
    )?;
    let leftovers = desk.run(&[
        "find",
        &format!("{home}/.local/state/noctalia"),
        "-maxdepth",
        "1",
        "-name",
        ".sysroot-activation-*",
    ])?;
    ensure(leftovers.stdout_text().trim().is_empty(), || {
        "activation temporaries were left behind".into()
    })?;
    review(&["unstage", "theme.mode"])?;
    set_and_observe(&original)?;

    // A CLI killed at a real file publication, then its recovery UI.
    let probe = "/usr/libexec/kedra-lab/probes/recovery.py";
    let recovery = context.environment.run(
        context.docker,
        &desk
            .exec(&["python3", probe, &state, selected, later, &original])
            .timeout(Duration::from_secs(300)),
    )?;
    for marker in [
        "KEDRA_R04_KILLED_CLI_ABORT",
        "KEDRA_R04_KILLED_CLI_RESUME",
        "KEDRA_R04_KILLED_CLI_KEEP_CURRENT",
    ] {
        ensure(recovery.stdout_text().contains(marker), || {
            format!("recovery probe did not report {marker}")
        })?;
    }
    set_and_observe(&original)?;

    // niri line review against the account's actual niri file, reconciled
    // with the exact source revision the image records.
    let bundle =
        kedra_container_tests::image::source_bundle(context.docker, &context.environment.id)?;
    let bundle_path = "/var/lib/kedra-lab/source.bundle";
    context.docker.write_file(
        &context.environment.id,
        bundle_path,
        &std::fs::read(&bundle)?,
        "root",
        "0644",
    )?;
    let niri = context.environment.run(
        context.docker,
        &desk
            .exec(&[
                "python3",
                "/usr/libexec/kedra-lab/probes/niri-review.py",
                &state,
                bundle_path,
            ])
            .timeout(Duration::from_secs(300)),
    )?;
    for marker in [
        "KEDRA_HOME_NIRI_WITHOUT_NOCTALIA_ADOPTION_PASS",
        "KEDRA_R04_NATIVE_NIRI_DISCARD_PASS",
        "KEDRA_R04_NATIVE_NIRI_RECOVERY_PASS",
        "KEDRA_R04_NIRI_INSTALLED_BASELINE_PASS",
        "KEDRA_R03_NATIVE_NIRI_LINES_PASS",
    ] {
        ensure(niri.stdout_text().contains(marker), || {
            format!("niri review probe did not report {marker}")
        })?;
    }
    let default = desk.json(&["sysroot", "home", "status"])?;
    ensure(
        default.pointer("/pending_activation") == Some(&Value::Null)
            && flag(&default, "/activation_performed") == Some(false),
        || format!("unexpected default home state: {default}"),
    )?;
    for directory in [
        format!("{home}/.local/state/sysroot"),
        format!("{home}/.local/state/sysroot/home"),
    ] {
        let mode = desk.run(&["stat", "-c", "%a", &directory])?.stdout_text();
        ensure(mode.trim() == "700", || {
            format!("{directory} has mode {}", mode.trim())
        })?;
    }
    Ok(())
}

/// Real GTK 3 and libadwaita (portal) file choosers on Wayland, driven by
/// typing through niri's virtual keyboard, each selecting the generated file.
/// Formerly driven by QEMU key events in the desktop VM. The GTK 3 Xwayland and
/// Qt 5/6 KDE choosers stay in the VM workflow: in the nested lab session
/// neither xwayland-satellite nor Qt 6.11 clients received key events from
/// niri's virtual keyboard (2026-09-27, niri 26.04).
fn toolkit_file_choosers(context: &Context<'_>) -> Result<()> {
    let desk = Desk::new(context)?;
    let home = desk.session.user.home.clone();
    let result_path = format!("{home}/toolkit-result.json");
    desk.run(&["systemctl", "--user", "start", "xdg-desktop-portal.service"])?;
    for (case, toolkit) in [
        ("gtk3-wayland", Toolkit::Gtk3),
        ("libadwaita", Toolkit::Gtk4),
    ] {
        desk.run(&["rm", "-f", &result_path])?;
        let unit = format!("kedra-toolkit-{case}");
        let backend = "--setenv=GDK_BACKEND=wayland";
        desk.run(&[
            "systemd-run",
            "--user",
            &format!("--unit={unit}"),
            "--collect",
            "--service-type=exec",
            backend,
            "/usr/bin/python3",
            "/usr/libexec/kedra-lab/probes/toolkit-app.py",
            case,
            &result_path,
        ])?;
        for stage in ["ready", "dialog", "selected"] {
            let mut last = String::new();
            let reached = desk.wait(&format!("{case} {stage}"), Duration::from_secs(45), || {
                let state = desk.try_run(&[
                    "systemctl",
                    "--user",
                    "show",
                    &format!("{unit}.service"),
                    "--property=ActiveState",
                    "--value",
                ])?;
                last = desk.try_run(&["cat", &result_path])?.stdout_text();
                if let Ok(document) = serde_json::from_str::<Value>(&last) {
                    match text(&document, "/stage") {
                        "failed" => return Err(fail(format!("{case} failed: {last}"))),
                        current if current == stage => return Ok(true),
                        _ => {}
                    }
                }
                let active = state.stdout_text();
                if !matches!(active.trim(), "active" | "activating") {
                    return Err(fail(format!(
                        "{case} exited before {stage} ({}); last result: {last}",
                        active.trim()
                    )));
                }
                Ok(false)
            });
            if let Err(error) = reached {
                let _ = desk.screenshot(&format!("toolkit-{case}-{stage}-timeout"));
                return Err(error);
            }
            desk.screenshot(&format!("toolkit-{case}-{stage}"))?;
            match stage {
                "ready" => {
                    // A new window reports ready before niri hands it keyboard focus.
                    std::thread::sleep(Duration::from_millis(1500));
                    desk.type_text(toolkit, "\n")?
                }
                "dialog" => {
                    // "/" opens the GTK chooser's location entry; let it take
                    // focus before typing the rest of the path.
                    desk.type_text(toolkit, "/")?;
                    std::thread::sleep(Duration::from_millis(700));
                    let path = format!("{home}/toolkit-sample.txt");
                    desk.type_text(toolkit, path.trim_start_matches('/'))?;
                    // GTK debounces location edits before enabling Open.
                    std::thread::sleep(Duration::from_secs(1));
                    desk.screenshot(&format!("toolkit-{case}-submitted"))?;
                    desk.type_text(toolkit, "\n")?;
                    if case == "libadwaita" {
                        // The portal chooser selects the file on the first
                        // Return; a second confirmation opens it, unless the
                        // first one already completed.
                        std::thread::sleep(Duration::from_secs(2));
                        let current = desk.try_run(&["cat", &result_path])?.stdout_text();
                        if !current.contains("\"selected\"") && !current.contains("\"failed\"") {
                            desk.type_text(toolkit, "\n")?;
                        }
                    }
                }
                _ => {}
            }
        }
        desk.run(&["systemctl", "--user", "stop", &format!("{unit}.service")])?;
    }
    Ok(())
}
