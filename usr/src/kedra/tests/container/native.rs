//! Native tests: Rust code using the same environment and fixture API as
//! scenarios, for flows that need control flow or input sequencing.

use std::time::{Duration, Instant};

use kedra_container_tests::docker::{Exec, Output};
use kedra_container_tests::scenario::{Context, Fixture, Profile};
use kedra_container_tests::session::{self, Session};
use kedra_container_tests::{Error, Result};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SourceApplicability {
    Ordinary,
    Composition,
    Derivation,
}
impl SourceApplicability {
    pub fn applies(self, composition: bool, derivation: bool) -> bool {
        matches!(
            (self, composition, derivation),
            (Self::Ordinary, false, false)
                | (Self::Composition, true, false)
                | (Self::Derivation, true, true)
        )
    }
}

pub struct NativeTest {
    pub source: SourceApplicability,
    pub name: &'static str,
    pub profile: Profile,
    pub fixtures: &'static [Fixture],
    /// Image targets this test applies to; all when empty.
    pub targets: &'static [&'static str],
    pub run: fn(&Context<'_>) -> Result<()>,
}

pub const TESTS: &[NativeTest] = &[
    NativeTest {
        source: SourceApplicability::Derivation,
        name: "catalog",
        profile: Profile::System,
        fixtures: &[Fixture::TestUser],
        targets: &["qemu-arm64"],
        run: crate::catalog_tests::installed_catalog,
    },
    NativeTest {
        source: SourceApplicability::Derivation,
        name: "native_artifacts",
        profile: Profile::System,
        fixtures: &[Fixture::TestUser],
        targets: &["qemu-arm64"],
        run: crate::native_artifacts_tests::native_artifacts,
    },
    NativeTest {
        source: SourceApplicability::Composition,
        name: "composition_unit",
        profile: Profile::System,
        fixtures: &[],
        targets: &["qemu-arm64"],
        run: composition_unit,
    },
    NativeTest {
        source: SourceApplicability::Ordinary,
        name: "home_files_update",
        profile: Profile::Desktop,
        fixtures: &[],
        targets: &[],
        run: crate::home_files_tests::home_files_update,
    },
    NativeTest {
        source: SourceApplicability::Ordinary,
        name: "toolkit_file_choosers",
        profile: Profile::Desktop,
        fixtures: &[],
        targets: &[],
        run: toolkit_file_choosers,
    },
];

pub(crate) fn fail(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

pub(crate) fn ensure(condition: bool, message: impl FnOnce() -> String) -> Result<()> {
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
pub(crate) struct Desk<'a, 'b> {
    pub(crate) context: &'a Context<'b>,
    pub(crate) session: &'a Session,
}

impl<'a, 'b> Desk<'a, 'b> {
    pub(crate) fn new(context: &'a Context<'b>) -> Result<Self> {
        let session = context
            .session
            .as_ref()
            .ok_or_else(|| fail("native test needs the desktop session"))?;
        Ok(Desk { context, session })
    }

    fn exec(&self, argv: &[&str]) -> Exec {
        self.session.exec(argv.iter().copied())
    }

    pub(crate) fn try_run(&self, argv: &[&str]) -> Result<Output> {
        self.context
            .environment
            .exec(self.context.docker, &self.exec(argv))
    }

    pub(crate) fn run(&self, argv: &[&str]) -> Result<Output> {
        self.context
            .environment
            .run(self.context.docker, &self.exec(argv))
    }

    pub(crate) fn json(&self, argv: &[&str]) -> Result<Value> {
        let output = self.run(argv)?;
        serde_json::from_slice(&output.stdout).map_err(|error| {
            fail(format!(
                "{argv:?} did not print JSON ({error}): {}",
                output.stdout_text()
            ))
        })
    }

    pub(crate) fn wait(
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
                    // The chooser must own keyboard focus before typing; the
                    // portal's dialog is a separate process that maps later.
                    desk.wait(
                        "the file chooser to take focus",
                        Duration::from_secs(30),
                        || {
                            let focused =
                                desk.try_run(&["niri", "msg", "--json", "focused-window"])?;
                            Ok(focused.stdout_text().contains("Select toolkit-sample.txt"))
                        },
                    )?;
                    std::thread::sleep(Duration::from_millis(500));
                    let path = format!("{home}/toolkit-sample.txt");
                    match toolkit {
                        // GTK 3: "/" opens the location entry (no chords via wlrctl).
                        Toolkit::Gtk3 => {
                            desk.type_text(toolkit, "/")?;
                            std::thread::sleep(Duration::from_millis(700));
                            desk.type_text(toolkit, path.trim_start_matches('/'))?;
                        }
                        // GTK 4: typing starts a search, so open the location
                        // entry explicitly with Ctrl+L first.
                        Toolkit::Gtk4 => {
                            desk.run(&["wtype", "-M", "ctrl", "-k", "l", "-m", "ctrl"])?;
                            std::thread::sleep(Duration::from_millis(700));
                            desk.type_text(toolkit, &path)?;
                        }
                    }
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

/// Actual generated unit behavior on the static composed image, without lab tools.
fn composition_unit(context: &Context<'_>) -> Result<()> {
    let docker = context.docker;
    let id = &context.environment.id;
    let expected = context
        .composition_identity
        .as_deref()
        .ok_or_else(|| fail("composition identity is missing"))?;
    let receipt: Value =
        serde_json::from_slice(&docker.read_file(id, "/usr/share/sysroot/composition.json")?)
            .map_err(|error| fail(error.to_string()))?;
    ensure(receipt["identity"].as_str() == Some(expected), || {
        "installed composition identity differs".into()
    })?;
    ensure(
        docker.read_file(id, "/etc/sysroot-context-fixture.conf")? == b"context-config-v1\n",
        || "installed fixture configuration differs".into(),
    )?;
    let unit = "sysroot-context-fixture.service";
    docker.run(
        id,
        &Exec::new([
            "systemd-analyze",
            "verify",
            "/usr/lib/systemd/system/sysroot-context-fixture.service",
        ]),
    )?;
    docker.run(id, &Exec::new(["systemctl", "start", unit]))?;
    docker.run(id, &Exec::new(["systemctl", "is-active", unit]))?;
    let properties = docker.run(
        id,
        &Exec::new([
            "systemctl",
            "show",
            unit,
            "--property=Result,ExecMainStatus,ActiveState,SubState",
        ]),
    )?;
    for required in [
        "Result=success",
        "ExecMainStatus=0",
        "ActiveState=active",
        "SubState=exited",
    ] {
        ensure(
            properties
                .stdout_text()
                .lines()
                .any(|line| line == required),
            || format!("unit property {required} missing"),
        )?;
    }
    let expected_output = b"context-fixture:unit:runtime-v1:context-config-v1\n";
    ensure(
        docker.read_file(id, "/run/sysroot-context-fixture.marker")? == expected_output,
        || "unit output marker differs".into(),
    )?;
    let journal = docker.run(
        id,
        &Exec::new(["journalctl", "--no-pager", "-u", unit, "-o", "cat"]),
    )?;
    ensure(
        journal
            .stdout_text()
            .lines()
            .any(|line| line == "context-fixture:unit:runtime-v1:context-config-v1"),
        || "unit journal lacks actual program output".into(),
    )?;
    std::fs::create_dir_all(&context.artifacts)?;
    std::fs::write(
        context.artifacts.join("composition-unit-properties.txt"),
        properties.stdout,
    )?;
    std::fs::write(
        context.artifacts.join("composition-unit-journal.txt"),
        journal.stdout,
    )?;
    Ok(())
}
