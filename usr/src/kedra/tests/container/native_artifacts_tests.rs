//! Native artifacts exercised inside the sanctioned System-profile environment.

use std::time::Duration;

use kedra_container_tests::docker::Exec;
use kedra_container_tests::scenario::Context;
use kedra_container_tests::session::TestUser;
use kedra_container_tests::{Error, Result};
use sha2::{Digest, Sha256};
use sysroot_engine::{NativeArtifactKind, NativeReceipt};

const SCHEMA: &str = "org.kedra.NativeFixture";
const HOME_FILE: &str = ".native-fixture";
const SEEDED: &[u8] = b"derived-home-v1\n";

fn fail(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn ensure(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(fail(message))
    }
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn unit_behavior(context: &Context<'_>) -> Result<()> {
    let docker = context.docker;
    let container = &context.environment.id;
    let unit = "sysroot-native-fixture.service";
    let enabled = docker.run(container, &Exec::new(["systemctl", "is-enabled", unit]))?;
    ensure(
        enabled.stdout_text().trim() == "enabled",
        "native unit was not enabled",
    )?;
    let target = docker.run(container, &Exec::new(["systemctl", "get-default"]))?;
    ensure(
        target.stdout_text().trim() == "multi-user.target",
        "native default target differs",
    )?;
    docker.run(
        container,
        &Exec::new([
            "systemd-analyze",
            "verify",
            "/usr/lib/systemd/system/sysroot-native-fixture.service",
        ]),
    )?;
    docker.run(container, &Exec::new(["systemctl", "start", unit]))?;
    let status = docker.run(
        container,
        &Exec::new([
            "systemctl",
            "show",
            unit,
            "--property=Result,ExecMainStatus,ActiveState,SubState",
        ]),
    )?;
    for expected in [
        "Result=success",
        "ExecMainStatus=0",
        "ActiveState=active",
        "SubState=exited",
    ] {
        ensure(
            status.stdout_text().lines().any(|line| line == expected),
            format!("native unit lacks {expected}"),
        )?;
    }
    let journal = docker.run(
        container,
        &Exec::new(["journalctl", "--no-pager", "-u", unit, "-o", "cat"]),
    )?;
    ensure(
        journal
            .stdout_text()
            .lines()
            .any(|line| line == "native-fixture-ready"),
        "native unit journal lacks actual program output",
    )?;
    let masked = "sysroot-native-masked.service";
    let state = docker.exec(container, &Exec::new(["systemctl", "is-enabled", masked]))?;
    ensure(
        state.stdout_text().trim() == "masked",
        "selected unit was not masked",
    )?;
    let start = docker.exec(container, &Exec::new(["systemctl", "start", masked]))?;
    ensure(
        start.exit != 0 && start.stderr_text().contains("masked"),
        "masked native unit did not refuse startup",
    )?;
    std::fs::write(
        context.artifacts.join("native-unit-status.txt"),
        &status.stdout,
    )?;
    std::fs::write(
        context.artifacts.join("native-unit-journal.txt"),
        &journal.stdout,
    )?;
    Ok(())
}

fn gsettings(context: &Context<'_>, user: &TestUser, args: &[&str]) -> Result<Vec<u8>> {
    let mut argv = vec!["gsettings"];
    argv.extend_from_slice(args);
    let mut command = user.exec(argv);
    command
        .env
        .push(("GSETTINGS_BACKEND".into(), "dconf".into()));
    Ok(context.environment.run(context.docker, &command)?.stdout)
}

fn defaults_and_accounts(context: &Context<'_>) -> Result<()> {
    let user = context
        .user
        .as_ref()
        .ok_or_else(|| fail("native artifacts case needs TestUser fixture"))?;
    let docker = context.docker;
    let container = &context.environment.id;
    docker.run(
        container,
        &Exec::new([
            "install",
            "-d",
            "-m",
            "700",
            "-o",
            &user.name,
            "-g",
            &user.name,
            &user.runtime_dir(),
        ]),
    )?;
    docker.run(
        container,
        &Exec::new(["systemctl", "start", &format!("user@{}.service", user.uid)]),
    )?;
    context.environment.run(
        docker,
        &user.exec(["systemctl", "--user", "start", "dbus.socket"]),
    )?;
    ensure(
        gsettings(context, user, &["get", SCHEMA, "greeting"])? == b"'derived-default'\n",
        "fresh account did not observe the compiled GLib override",
    )?;
    gsettings(
        context,
        user,
        &["set", SCHEMA, "greeting", "'explicit-preference'"],
    )?;
    // Distinct native clients share the real systemd user bus and persistent dconf.
    ensure(
        gsettings(context, user, &["get", SCHEMA, "greeting"])? == b"'explicit-preference'\n",
        "explicit dconf preference did not override the compiled default",
    )?;
    let home_file = format!("{}/{HOME_FILE}", user.home);
    ensure(
        docker.read_file(container, &home_file)? == SEEDED,
        "fresh account did not receive declared skeleton",
    )?;
    docker.write_file(
        container,
        &home_file,
        b"existing-user-edit\n",
        &user.name,
        "600",
    )?;
    docker.run(
        container,
        &Exec::new(["useradd", "-m", "kedra-native-second"]),
    )?;
    let entry = docker.run(
        container,
        &Exec::new(["getent", "passwd", "kedra-native-second"]),
    )?;
    let entry = entry.stdout_text();
    let fields: Vec<_> = entry.trim().split(':').collect();
    let second_home = fields
        .get(5)
        .ok_or_else(|| fail("second disposable account lacks a home"))?;
    ensure(
        docker.read_file(container, &format!("{second_home}/{HOME_FILE}"))? == SEEDED,
        "subsequent fresh account did not receive the native skeleton",
    )?;
    ensure(
        docker.read_file(container, &home_file)? == b"existing-user-edit\n",
        "fresh-account seeding changed an existing user's modified home",
    )?;
    ensure(
        docker.read_file(container, &format!("/etc/skel/{HOME_FILE}"))? == SEEDED,
        "user customization changed the image skeleton",
    )?;
    ensure(
        gsettings(context, user, &["get", SCHEMA, "greeting"])? == b"'explicit-preference'\n",
        "fresh-account creation overwrote the existing preference",
    )?;
    std::fs::write(context.artifacts.join("native-defaults-and-home.txt"),
        b"fresh GLib default=derived-default\npersisted dconf override=explicit-preference\nfresh homes=derived-home-v1\nexisting modified home=existing-user-edit\n")?;
    Ok(())
}

fn artifact_material(context: &Context<'_>, receipt: &NativeReceipt) -> Result<()> {
    let docker = context.docker;
    let container = &context.environment.id;
    for artifact in &receipt.artifacts {
        match &artifact.entry {
            NativeArtifactKind::Regular {
                sha256,
                bytes,
                mode,
            } => {
                let digest = docker.run(
                    container,
                    &Exec::new(["sha256sum", "--", &artifact.path])
                        .timeout(Duration::from_secs(180)),
                )?;
                ensure(
                    digest.stdout_text().split_whitespace().next() == Some(sha256.as_str()),
                    format!("native artifact bytes differ: {}", artifact.path),
                )?;
                let metadata = docker.run(
                    container,
                    &Exec::new(["stat", "-c", "%s %a", "--", &artifact.path]),
                )?;
                ensure(
                    metadata.stdout_text().trim() == format!("{bytes} {mode:o}"),
                    format!("native artifact size/mode differs: {}", artifact.path),
                )?;
            }
            NativeArtifactKind::Symlink { target } => {
                let actual =
                    docker.run(container, &Exec::new(["readlink", "--", &artifact.path]))?;
                ensure(
                    actual.stdout_text().trim() == target,
                    format!("native link differs: {}", artifact.path),
                )?;
            }
            NativeArtifactKind::RemovedSymlink { .. } => {
                docker.run(container, &Exec::new(["test", "!", "-e", &artifact.path]))?;
                docker.run(container, &Exec::new(["test", "!", "-L", &artifact.path]))?;
            }
        }
    }
    ensure(
        !receipt.kernels.is_empty(),
        "native fixture generated no kernel initramfs",
    )?;
    for kernel in &receipt.kernels {
        let root = format!("/usr/lib/modules/{}", kernel.version);
        let digest = docker.run(
            container,
            &Exec::new(["sha256sum", "--", &format!("{root}/vmlinuz")]),
        )?;
        ensure(
            digest.stdout_text().split_whitespace().next() == Some(kernel.kernel_sha256.as_str()),
            "image-local kernel digest differs from the generated receipt",
        )?;
        let mut command = Exec::new(["lsinitrd", &format!("{root}/initramfs.img")])
            .timeout(Duration::from_secs(180));
        command.env.push(("LC_ALL".into(), "C".into()));
        let listing = docker.run(container, &command)?;
        ensure(
            hash(&listing.stdout) == kernel.listing_sha256,
            "actual initramfs listing differs",
        )?;
        let listing_text = listing.stdout_text();
        for module in ["qemu", "crypt", "dm", "rootfs-block"] {
            ensure(
                listing_text.lines().any(|line| line.trim() == module),
                format!("generic initramfs lacks required dracut module {module}"),
            )?;
        }
        for driver in ["virtio_dma_buf", "virtio_gpu", "virtio_input"] {
            let path = kernel
                .required_modules
                .get(driver)
                .ok_or_else(|| fail(format!("kernel receipt omits {driver}")))?;
            ensure(
                listing_text.contains(path.trim_start_matches('/')),
                format!("actual initramfs lacks required module {driver}"),
            )?;
            let installed = docker.run(
                container,
                &Exec::new(["modinfo", "-k", &kernel.version, "-n", driver]),
            )?;
            let canonical = docker.run(
                container,
                &Exec::new(["readlink", "-f", installed.stdout_text().trim()]),
            )?;
            ensure(
                canonical.stdout_text().trim() == path,
                "required driver resolves outside recorded kernel material",
            )?;
        }
        std::fs::write(
            context
                .artifacts
                .join(format!("native-initramfs-{}.txt", kernel.version)),
            &listing.stdout,
        )?;
    }
    Ok(())
}

pub fn native_artifacts(context: &Context<'_>) -> Result<()> {
    std::fs::create_dir_all(&context.artifacts)?;
    let bytes = context.docker.read_file(
        &context.environment.id,
        "/usr/share/sysroot/native-receipt.json",
    )?;
    let receipt: NativeReceipt =
        serde_json::from_slice(&bytes).map_err(|error| fail(error.to_string()))?;
    let expected = context.derivation_identity.as_deref().ok_or_else(|| {
        fail("native artifact case requires independently selected derivation identity")
    })?;
    ensure(
        receipt.identity == expected,
        "installed native derivation identity differs",
    )?;
    ensure(
        context.composition_identity.as_deref() == Some(receipt.parent_identity.as_str()),
        "installed native parent identity differs",
    )?;
    artifact_material(context, &receipt)?;
    unit_behavior(context)?;
    defaults_and_accounts(context)?;
    std::fs::write(context.artifacts.join("native-receipt.json"), bytes)?;
    Ok(())
}
