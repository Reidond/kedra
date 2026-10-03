//! Installed catalog applications and persistent service state in the sanctioned harness.
use kedra_container_tests::docker::Exec;
use kedra_container_tests::scenario::Context;
use kedra_container_tests::{Error, Result};
use serde_json::Value;
use sysroot_engine::LOGICAL_PREFIX;

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Invalid(message.into()))
    }
}

fn output<'a>(inventory: &'a Value, name: &str) -> Result<&'a str> {
    let value = inventory["outputs"][name]
        .as_str()
        .ok_or_else(|| Error::Invalid("catalog output is missing".into()))?;
    ensure(
        value.strip_prefix("out-").is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        }),
        "catalog output identity is invalid",
    )?;
    Ok(value)
}

pub fn installed_catalog(context: &Context<'_>) -> Result<()> {
    std::fs::create_dir_all(&context.artifacts)?;
    let docker = context.docker;
    let container = &context.environment.id;
    let user = context
        .user
        .as_ref()
        .ok_or_else(|| Error::Invalid("catalog case requires a disposable user".into()))?;
    let inventory: Value =
        serde_json::from_slice(&docker.read_file(container, "/usr/share/kedra/catalog.json")?)
            .map_err(|error| Error::Invalid(error.to_string()))?;
    ensure(
        inventory["schema_version"] == 1 && inventory["namespace"] == "kedra",
        "installed catalog inventory is unsupported",
    )?;
    let jq_version = inventory["versions"]["jq"]
        .as_str()
        .ok_or_else(|| Error::Invalid("selected jq version is missing".into()))?;
    let sqlite_version = inventory["versions"]["sqlite"]
        .as_str()
        .ok_or_else(|| Error::Invalid("selected SQLite version is missing".into()))?;
    let jq = format!("{LOGICAL_PREFIX}/{}/bin/jq", output(&inventory, "jq")?);
    let sqlite = format!(
        "{LOGICAL_PREFIX}/{}/bin/sqlite3",
        output(&inventory, "sqlite")?
    );
    let version = context
        .environment
        .run(docker, &user.exec([jq.as_str(), "--version"]))?;
    ensure(
        version.stdout_text() == format!("jq-{jq_version}\n"),
        "installed catalog jq version differs",
    )?;
    let shell_path = context.environment.run(
        docker,
        &user.exec(["/bin/bash", "-lc", "command -v jq; command -v sqlite3"]),
    )?;
    ensure(
        shell_path.stdout_text() == format!("{jq}\n{sqlite}\n"),
        "login shell does not select exact catalog paths",
    )?;

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
    let user_path = context.environment.run(
        docker,
        &user.exec([
            "systemd-run",
            "--user",
            "--quiet",
            "--wait",
            "--pipe",
            "--collect",
            "/bin/bash",
            "--noprofile",
            "--norc",
            "-c",
            "command -v jq; command -v sqlite3",
        ]),
    )?;
    ensure(
        user_path.stdout_text() == format!("{jq}\n{sqlite}\n"),
        "user service environment does not select exact catalog paths",
    )?;

    let rows = context.environment.run(docker, &user.exec([sqlite.as_str(), ":memory:",
        "CREATE TABLE work(name TEXT, count INTEGER); INSERT INTO work VALUES('alpha',2),('beta',1),('gamma',3); SELECT json_group_array(json_object('name',name,'count',count)) FROM (SELECT * FROM work ORDER BY name);"
    ]))?;
    let transformed = context.environment.run(
        docker,
        &user
            .exec([
                jq.as_str(),
                "-c",
                "map(select(.count >= 2 and (.name | test(\"^a|^g\")))) | map(.name)",
            ])
            .stdin(rows.stdout.clone()),
    )?;
    ensure(
        transformed.stdout == b"[\"alpha\",\"gamma\"]\n",
        "installed SQL to jq/regex workflow differs",
    )?;
    let rpm = docker.run(container, &Exec::new(["rpm", "-qf", "/usr/bin/jq"]))?;
    ensure(
        rpm.stdout_text().starts_with("jq-"),
        "foundation jq lost RPM ownership",
    )?;
    let original = docker.run(container, &Exec::new(["/usr/bin/jq", "-n", "40+2"]))?;
    ensure(
        original.stdout == b"42\n",
        "foundation jq no longer executes",
    )?;

    let unit = "kedra-catalog-history.service";
    let database = "/var/lib/kedra-catalog/history.sqlite";
    docker.run(
        container,
        &Exec::new([
            "systemd-analyze",
            "verify",
            "/usr/lib/systemd/system/kedra-catalog-history.service",
        ]),
    )?;
    docker.run(container, &Exec::new(["systemctl", "start", unit]))?;
    let first = docker.run(
        container,
        &Exec::new([
            sqlite.as_str(),
            database,
            "SELECT count(*) FROM catalog_runs;",
        ]),
    )?;
    let first: u64 = first
        .stdout_text()
        .trim()
        .parse()
        .map_err(|_| Error::Invalid("invalid first history count".into()))?;
    ensure(first > 0, "service did not persist its first observation")?;
    docker.run(container, &Exec::new([sqlite.as_str(), database, "CREATE TABLE owner_notes(note TEXT); INSERT INTO owner_notes VALUES('retained-user-row');"]))?;
    docker.run(container, &Exec::new(["systemctl", "stop", unit]))?;
    docker.run(container, &Exec::new(["systemctl", "start", unit]))?;
    let persisted = docker.run(container, &Exec::new([sqlite.as_str(), database,
        "SELECT count(*) FROM catalog_runs; SELECT note FROM owner_notes; SELECT min(sqlite_version) FROM catalog_runs; SELECT min(source_revision) FROM catalog_runs;"
    ]))?;
    let revision = inventory["source_revision"]
        .as_str()
        .ok_or_else(|| Error::Invalid("catalog source revision missing".into()))?;
    ensure(
        persisted.stdout_text()
            == format!(
                "{}\nretained-user-row\n{sqlite_version}\n{revision}\n",
                first + 1
            ),
        "service restart lost or changed persistent history/user data",
    )?;
    let state = docker.run(
        container,
        &Exec::new([
            "systemctl",
            "show",
            unit,
            "--property=Result,ExecMainStatus",
        ]),
    )?;
    ensure(
        state.stdout_text().contains("Result=success")
            && state.stdout_text().contains("ExecMainStatus=0"),
        "catalog service did not exit successfully",
    )?;
    std::fs::write(
        context.artifacts.join("catalog-user-path.txt"),
        &user_path.stdout,
    )?;
    std::fs::write(
        context.artifacts.join("catalog-sql-to-jq.json"),
        &transformed.stdout,
    )?;
    std::fs::write(
        context.artifacts.join("catalog-service-persistence.txt"),
        &persisted.stdout,
    )?;
    std::fs::write(
        context.artifacts.join("catalog-service-state.txt"),
        &state.stdout,
    )?;
    Ok(())
}
