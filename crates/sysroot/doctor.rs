//! Read-only observations of the installed system; never source-repository tests.
pub fn run(json: bool) -> Result<bool, Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        linux::run(json)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = json;
        Err("doctor requires an installed Linux Kedra desktop".into())
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use serde::{Deserialize, Serialize};
    use std::io::Read;
    use std::path::Path;
    use std::process::{Command, Stdio};
    type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[derive(Deserialize)]
    struct Source {
        schema_version: u32,
        source_revision: String,
        target: Target,
    }
    #[derive(Deserialize)]
    struct Target {
        id: String,
    }
    #[derive(Serialize)]
    struct Check {
        name: &'static str,
        passed: bool,
        required_for_session: bool,
        detail: String,
        next_action: Option<&'static str>,
    }
    fn query(binary: &str, arguments: &[&str]) -> Result<(bool, String)> {
        let mut child = Command::new("/usr/bin/timeout")
            .args(["--kill-after=2s", "10s", binary])
            .args(arguments)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()?;
        let result = (|| -> Result<(bool, String)> {
            let mut bytes = Vec::new();
            child
                .stdout
                .take()
                .ok_or("diagnostic output is unavailable")?
                .take(8193)
                .read_to_end(&mut bytes)?;
            if bytes.len() > 8192 {
                return Err("diagnostic output exceeded its bound".into());
            }
            let success = child.wait()?.success();
            let value = String::from_utf8(bytes).map_err(|_| "diagnostic output is malformed")?;
            Ok((success, value.trim().to_owned()))
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }
    fn check(
        name: &'static str,
        binary: &str,
        arguments: &[&str],
        expected: Option<&str>,
        success: &'static str,
        failure: &'static str,
        next: &'static str,
    ) -> Check {
        let passed = query(binary, arguments)
            .is_ok_and(|(ok, value)| ok && expected.is_none_or(|wanted| value == wanted));
        Check {
            name,
            passed,
            required_for_session: true,
            detail: if passed {
                success.into()
            } else {
                failure.into()
            },
            next_action: if passed { None } else { Some(next) },
        }
    }
    pub(super) fn run(json: bool) -> Result<bool> {
        if rustix::process::geteuid().as_raw() == 0 {
            return Err("run doctor as the ordinary logged-in desktop user, without sudo".into());
        }
        let source: Source = serde_json::from_slice(&sysroot_helper::trusted_file::read(
            Path::new("/usr/share/sysroot/source.json"),
            0,
            1_048_576,
        )?)
        .map_err(|_| "installed Kedra source information is malformed")?;
        if source.schema_version != 1 {
            return Err("installed source information has an unsupported version".into());
        }
        let mut checks = vec![
            check(
                "selinux",
                "/usr/sbin/getenforce",
                &[],
                Some("Enforcing"),
                "Enforcing",
                "SELinux is not enforcing or could not be read",
                "Inspect getenforce and the boot configuration.",
            ),
            check(
                "system_services",
                "/usr/bin/systemctl",
                &["show", "--property=NFailedUnits", "--value"],
                Some("0"),
                "No failed system services",
                "System services failed or could not be inspected",
                "Run systemctl --failed.",
            ),
            check(
                "user_services",
                "/usr/bin/systemctl",
                &["--user", "show", "--property=NFailedUnits", "--value"],
                Some("0"),
                "No failed user services",
                "User services failed or the user session is unavailable",
                "Run systemctl --user --failed in the desktop session.",
            ),
            check(
                "niri_configuration",
                "/usr/bin/niri",
                &["validate"],
                None,
                "Configuration is valid",
                "Niri configuration did not validate",
                "Run niri validate and review its diagnostics.",
            ),
            check(
                "noctalia_configuration",
                "/usr/bin/noctalia",
                &["config", "validate"],
                None,
                "Configuration is valid",
                "Noctalia configuration did not validate",
                "Run noctalia config validate and review its diagnostics.",
            ),
            check(
                "login_keyring",
                "/usr/bin/busctl",
                &[
                    "--user",
                    "get-property",
                    "org.freedesktop.secrets",
                    "/org/freedesktop/secrets/aliases/default",
                    "org.freedesktop.Secret.Collection",
                    "Locked",
                ],
                Some("b false"),
                "Login keyring is unlocked",
                "Login keyring is locked or unavailable",
                "Log in with the account password and inspect the login keyring.",
            ),
        ];
        for (name, unit) in [
            ("compositor", "niri.service"),
            ("desktop_controls", "kedra-noctalia.service"),
            ("audio", "pipewire.service"),
            ("audio_policy", "wireplumber.service"),
            ("desktop_portal", "xdg-desktop-portal.service"),
        ] {
            checks.push(check(
                name,
                "/usr/bin/systemctl",
                &["--user", "is-active", unit],
                Some("active"),
                "Service is active",
                "Service is inactive or unavailable",
                "Inspect systemctl --user --failed and log back into the desktop.",
            ));
        }
        let release_trust = sysroot_helper::trusted_file::read(
            Path::new("/usr/lib/sysroot/trust/release-policy.json"),
            0,
            16_384,
        )
        .is_ok()
            && sysroot_helper::trusted_file::read(
                Path::new("/usr/lib/sysroot/trust/release.pub"),
                0,
                4096,
            )
            .is_ok();
        let research = Path::new("/usr/share/sysroot/research-only").try_exists()?;
        checks.push(Check {
            name: "release_setup",
            passed: release_trust && !research,
            required_for_session: false,
            detail: if research {
                "Research signing authority; this is not an owner release".into()
            } else if release_trust {
                "Public release trust is present; enrollment has not been queried".into()
            } else {
                "Release trust is not configured in this development image".into()
            },
            next_action: Some(if release_trust && !research {
                "Run sysroot update status to inspect the signed deployment."
            } else {
                "Use a promoted owner installer when it becomes available."
            }),
        });
        let passed = checks
            .iter()
            .filter(|c| c.required_for_session)
            .all(|c| c.passed);
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"schema_version":1,"project":"Kedra",
                "target":source.target.id,"source_revision":source.source_revision,
                "desktop_session_checks_passed":passed,"changes_performed":false,"checks":checks})
                )?
            );
        } else {
            println!(
                "Kedra {} — {}",
                source.target.id,
                if passed {
                    "desktop session checks passed"
                } else {
                    "desktop session needs attention"
                }
            );
            for item in checks {
                println!(
                    "{} {}: {}",
                    if item.passed { "OK" } else { "ATTENTION" },
                    item.name,
                    item.detail
                );
                if let Some(next) = item.next_action {
                    println!("  {next}");
                }
            }
            println!("Source: {}", source.source_revision);
        }
        Ok(passed)
    }
}
