//! Public installed-home commands over an actual exported composition.
use std::path::PathBuf;

use kedra_container_tests::docker::Exec;
use kedra_container_tests::scenario::Context;
use kedra_container_tests::{Error, Result};
use serde_json::{Value, json};

const RECORD: &str = "/usr/share/sysroot/home-artifacts.json";
const BASELINE: &str = "/usr/share/sysroot/home/default/.config/niri/config.kdl";
const OLD: &str = "/usr/libexec/kedra-lab/sysroot-old";

fn require(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}

pub fn installed_record(context: &Context<'_>) -> Result<()> {
    let docker = context.docker;
    let id = &context.environment.id;
    let record = docker.read_file(id, RECORD)?;
    let baseline = docker.read_file(id, BASELINE)?;
    let original: Value =
        serde_json::from_slice(&record).map_err(|error| Error::Invalid(error.to_string()))?;
    let receipt = serde_json::from_value(original["niri"].clone())
        .map_err(|error| Error::Invalid(error.to_string()))?;
    sysroot_engine::verify_single_file_source(&receipt, "config.kdl", &baseline)
        .map_err(|error| Error::Invalid(error.to_string()))?;
    let user = context
        .user
        .as_ref()
        .ok_or_else(|| Error::Invalid("test user is missing".into()))?;
    let live = format!("{}/.config/niri/config.kdl", user.home);
    let live_before = docker.read_file(id, &live)?;
    let uid = user.uid.to_string();
    let execute = |binary: &str, state: &str, args: &[&str]| {
        let mut argv = vec![binary, "home", "--state", state, "file"];
        argv.extend_from_slice(args);
        let mut exec = user.exec(argv);
        exec.env.extend([
            ("HOME".into(), user.home.clone()),
            (
                "DOCKER_HOST".into(),
                "unix:///unavailable-build-store.sock".into(),
            ),
        ]);
        context.environment.exec(docker, &exec)
    };
    let mut sequence = 0;
    let mut adopt = |binary: &str, refusal: Option<&str>| -> Result<String> {
        sequence += 1;
        let state = format!("{}/artifact-case-{sequence}", user.home);
        let output = execute(binary, &state, &["init", "--reviewed-safe"])?;
        match refusal {
            Some(reason) => {
                require(
                    output.exit != 0 && output.stdout.is_empty(),
                    "invalid record was accepted",
                )?;
                require(
                    output.stderr_text().contains(reason),
                    &format!("wrong record refusal: {}", output.stderr_text()),
                )?;
            }
            None => {
                require(
                    output.exit == 0,
                    &format!("valid installed baseline refused: {}", output.stderr_text()),
                )?;
            }
        }
        require(
            docker.read_file(id, &live)? == live_before,
            "baseline loading changed live home",
        )?;
        Ok(state)
    };
    let write = |bytes: &[u8]| docker.write_file(id, RECORD, bytes, "root", "0644");
    let result: Result<()> = (|| {
        let current = adopt("sysroot", None)?;
        let state_before = execute("sysroot", &current, &["status"])?;
        require(
            state_before.exit == 0,
            "current record state cannot be read",
        )?;
        let mut padded = record.clone();
        padded.resize(4096, b' ');
        write(&padded)?;
        adopt("sysroot", None)?;
        padded.push(b' ');
        write(&padded)?;
        adopt(
            "sysroot",
            Some("unsafe ownership, type, links, size or mode"),
        )?;
        for (pointer, value, reason) in [
            ("/schema", json!(2), "unsupported home artifact"),
            ("/niri/schema", json!(2), "single-file source receipt"),
            (
                "/niri/object",
                json!("src-invalid"),
                "single-file source receipt",
            ),
            (
                "/niri/tree_sha256",
                json!("0".repeat(64)),
                "single-file source receipt",
            ),
            (
                "/niri/references",
                json!([original["niri"]["object"].clone()]),
                "single-file source receipt",
            ),
            (
                "/niri/runtime_image",
                json!("sha256:".to_owned() + &"0".repeat(64)),
                "single-file source receipt",
            ),
            ("/niri/derivation", json!({}), "missing field"),
        ] {
            let mut changed = original.clone();
            *changed
                .pointer_mut(pointer)
                .ok_or_else(|| Error::Invalid("fixture field missing".into()))? = value;
            write(
                &serde_json::to_vec(&changed).map_err(|error| Error::Invalid(error.to_string()))?,
            )?;
            adopt("sysroot", Some(reason))?;
        }
        let mut unknown = original.clone();
        unknown["destination"] = json!("/home/other-file");
        write(&serde_json::to_vec(&unknown).map_err(|error| Error::Invalid(error.to_string()))?)?;
        adopt("sysroot", Some("unknown field"))?;
        unknown = original.clone();
        unknown["niri"]["extra"] = json!(true);
        write(&serde_json::to_vec(&unknown).map_err(|error| Error::Invalid(error.to_string()))?)?;
        adopt("sysroot", Some("unknown field"))?;
        let duplicate = format!(
            "{{\"schema\":1,\"schema\":1,\"niri\":{}}}",
            original["niri"]
        );
        write(duplicate.as_bytes())?;
        adopt("sysroot", Some("duplicate field"))?;
        let duplicate = format!(
            "{{\"schema\":1,\"niri\":{},\"niri\":{}}}",
            original["niri"], original["niri"]
        );
        write(duplicate.as_bytes())?;
        adopt("sysroot", Some("duplicate field"))?;
        write(&record)?;
        for mode in ["0600", "0664", "0755"] {
            docker.run(id, &Exec::new(["chmod", mode, RECORD]))?;
            adopt("sysroot", Some("mode 0644"))?;
        }
        write(&record)?;
        docker.run(id, &Exec::new(["chown", &uid, RECORD]))?;
        adopt("sysroot", Some("unsafe ownership"))?;
        docker.run(id, &Exec::new(["chown", "root:root", RECORD]))?;
        let link = "/usr/share/sysroot/artifact-hardlink-fixture";
        docker.run(id, &Exec::new(["ln", RECORD, link]))?;
        adopt("sysroot", Some("unsafe ownership"))?;
        docker.run(id, &Exec::new(["rm", link]))?;
        for target in [BASELINE, "/nonexistent-home-artifact-fixture"] {
            docker.run(id, &Exec::new(["rm", RECORD]))?;
            docker.run(id, &Exec::new(["ln", "-s", target, RECORD]))?;
            // A dangling link is present: it must never become legacy absence.
            adopt("sysroot", Some("mode 0644"))?;
            docker.run(id, &Exec::new(["rm", RECORD]))?;
            write(&record)?;
        }
        docker.write_file(id, BASELINE, b"layout {}\n", "root", "0644")?;
        adopt("sysroot", Some("installed niri baseline hash differs"))?;
        docker.write_file(id, BASELINE, &baseline, "root", "0644")?;
        let manifest_path = "/usr/share/sysroot/source.json";
        let manifest = docker.read_file(id, manifest_path)?;
        let mut invalid_manifest: Value =
            serde_json::from_slice(&manifest).map_err(|error| Error::Invalid(error.to_string()))?;
        invalid_manifest["schema_version"] = json!(2);
        docker.write_file(
            id,
            manifest_path,
            &serde_json::to_vec(&invalid_manifest)
                .map_err(|error| Error::Invalid(error.to_string()))?,
            "root",
            "0644",
        )?;
        let source_result = adopt("sysroot", Some("unique ordinary-file provenance"));
        docker.write_file(id, manifest_path, &manifest, "root", "0644")?;
        source_result?;
        docker.run(id, &Exec::new(["rm", RECORD]))?;
        adopt("sysroot", None)?;
        write(&record)?;
        let after = execute("sysroot", &current, &["status"])?;
        require(
            after.exit == 0 && after.stdout == state_before.stdout,
            "record refusals changed existing review state",
        )?;

        // Require an independently retained pre-feature executable for cross-version coverage.
        let old_path = std::env::var_os("KEDRA_LAB_OLD_SYSROOT")
            .map(PathBuf::from)
            .ok_or_else(|| {
                Error::Invalid("KEDRA_LAB_OLD_SYSROOT must pin a retained old Linux CLI".into())
            })?;
        let old = std::fs::read(&old_path)?;
        docker.write_file(id, OLD, &old, "root", "0755")?;
        let old_state = adopt(OLD, None)?;
        for (binary, state) in [("sysroot", old_state.as_str()), (OLD, current.as_str())] {
            let status = execute(binary, state, &["status"])?;
            require(status.exit == 0, "cross-version state readback failed")?;
            let value: Value = serde_json::from_slice(&status.stdout)
                .map_err(|error| Error::Invalid(error.to_string()))?;
            require(
                value["pending_activation"].is_null(),
                "cross-version adoption created recovery state",
            )?;
        }
        docker.run(id, &Exec::new(["rm", RECORD]))?;
        adopt(OLD, None)?;
        adopt("sysroot", None)?;
        write(&record)?;
        Ok(())
    })();
    // Restore only the exact fixture paths on this disposable container, even on failure.
    let _ = docker.run(
        id,
        &Exec::new([
            "rm",
            "-f",
            RECORD,
            "/usr/share/sysroot/artifact-hardlink-fixture",
        ]),
    );
    let restore_record = write(&record);
    let restore_baseline = docker.write_file(id, BASELINE, &baseline, "root", "0644");
    result?;
    restore_record?;
    restore_baseline?;
    require(
        docker.read_file(id, &live)? == live_before,
        "record checks changed live data",
    )?;
    Ok(())
}
