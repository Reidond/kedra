use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const PIN: &str = "5c40d3ad785193231b7d0dbfb8e1eb447e5edd94";
const MEMBERS: &[(&str, &str)] = &[
    ("crates/sysroot", "main.rs"),
    ("crates/sysroot-core", "lib.rs"),
    ("crates/sysroot-helper", "main.rs"),
    ("xtask", "main.rs"),
];

fn problem(message: impl Into<String>) -> io::Error {
    io::Error::other(message.into())
}

fn reject_src(path: &Path) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if entry.file_name() == "src" {
            return Err(problem(format!(
                "first-party src path: {}",
                entry.path().display()
            )));
        }
        if kind.is_symlink() {
            return Err(problem(format!(
                "symlink in first-party Rust tree: {}",
                entry.path().display()
            )));
        }
        if kind.is_dir() {
            reject_src(&entry.path())?;
        }
    }
    Ok(())
}

fn layout(root: &Path) -> io::Result<()> {
    if root.join("src").exists() || root.join("src").is_symlink() {
        return Err(problem("root src/ is prohibited"));
    }
    for (member, target) in MEMBERS {
        let directory = root.join(member);
        reject_src(&directory)?;
        let manifest = fs::read_to_string(directory.join("Cargo.toml"))?;
        // This bootstrap check enforces our canonical manifest spelling, not arbitrary TOML.
        for required in [
            format!("path = \"{target}\""),
            "edition.workspace = true".to_owned(),
            "rust-version.workspace = true".to_owned(),
            "publish.workspace = true".to_owned(),
            "[lints]\nworkspace = true".to_owned(),
        ] {
            if !manifest.contains(&required) {
                return Err(problem(format!(
                    "{member}/Cargo.toml must contain {required:?}"
                )));
            }
        }
        if !directory.join(target).is_file() {
            return Err(problem(format!(
                "missing explicit entry point: {member}/{target}"
            )));
        }
    }
    let helper = fs::read_to_string(root.join("crates/sysroot-helper/Cargo.toml"))?;
    if helper.contains("sysroot.workspace") || helper.contains("xtask") {
        return Err(problem("helper must not depend on CLI or xtask"));
    }
    println!("layout: explicit first-party entry points and no src/ paths");
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> io::Result<String> {
    let output = Command::new("git").args(args).current_dir(root).output()?;
    if !output.status.success() {
        return Err(problem(format!(
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    String::from_utf8(output.stdout).map_err(|error| problem(error.to_string()))
}

fn skills(root: &Path) -> io::Result<()> {
    let lock = fs::read_to_string(root.join("skills.lock.toml"))?;
    if !lock.contains(&format!("revision = \"{PIN}\"")) {
        return Err(problem("skills.lock.toml and validator pin disagree"));
    }
    let staged = git(root, &["ls-files", "--stage", "vendor/rust-skills"])?;
    if !staged.starts_with(&format!("160000 {PIN} 0\t")) {
        return Err(problem(
            "recorded Rust-skills gitlink does not match the pin",
        ));
    }
    let upstream = root.join("vendor/rust-skills");
    if !upstream.join("skills/rust-router/SKILL.md").is_file() {
        return Err(problem(
            "initialize pinned skills: git submodule update --init --recursive",
        ));
    }
    if git(&upstream, &["rev-parse", "HEAD"])?.trim() != PIN {
        return Err(problem(
            "initialized Rust-skills checkout differs from the pin",
        ));
    }
    let mut names = BTreeSet::new();
    for base in ["skills", "vendor/rust-skills/skills"] {
        for entry in fs::read_dir(root.join(base))? {
            let entry = entry?;
            let skill = entry.path().join("SKILL.md");
            if !skill.is_file() {
                continue;
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| problem("invalid skill name"))?;
            if !names.insert(name.clone()) {
                return Err(problem(format!("duplicate skill name: {name}")));
            }
            let text = fs::read_to_string(&skill)?;
            if !text.starts_with("---\n")
                || !text.contains("\nname:")
                || !text.contains("\ndescription:")
            {
                return Err(problem(format!(
                    "missing skill frontmatter: {}",
                    skill.display()
                )));
            }
            for agent in [".agents", ".claude"] {
                let link = root.join(agent).join("skills").join(&name);
                let expected = PathBuf::from(format!("../../{base}/{name}"));
                if fs::read_link(&link)? != expected {
                    return Err(problem(format!("incorrect skill link: {}", link.display())));
                }
                if link.canonicalize()? != entry.path().canonicalize()? {
                    return Err(problem(format!("skill link target mismatch: {name}")));
                }
            }
        }
    }
    for agent in [".agents", ".claude"] {
        let entries = fs::read_dir(root.join(agent).join("skills"))?;
        for entry in entries {
            let name = entry?.file_name();
            if !names.contains(name.to_string_lossy().as_ref()) {
                return Err(problem(format!(
                    "unregistered skill link: {}",
                    name.to_string_lossy()
                )));
            }
        }
    }
    println!(
        "skills: {} canonical directories exposed to both agents",
        names.len()
    );
    Ok(())
}

fn cargo(root: &Path, args: &[&str]) -> io::Result<()> {
    println!("+ cargo {}", args.join(" "));
    let executable = env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(executable)
        .args(args)
        .current_dir(root)
        .status()?;
    if !status.success() {
        return Err(problem(format!("cargo {args:?} failed with {status}")));
    }
    Ok(())
}

fn run() -> io::Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| problem("workspace root missing"))?;
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [task] if task == "check-layout" => layout(root),
        [task] if task == "check-skills" => skills(root),
        [task] if task == "check" => {
            layout(root)?;
            skills(root)?;
            cargo(
                root,
                &["metadata", "--locked", "--no-deps", "--format-version", "1"],
            )?;
            cargo(root, &["fmt", "--all", "--", "--check"])?;
            cargo(
                root,
                &[
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--locked",
                    "--",
                    "-D",
                    "warnings",
                ],
            )?;
            cargo(root, &["test", "--workspace", "--locked"])?;
            cargo(root, &["build", "--workspace", "--release", "--locked"])
        }
        _ => Err(problem(
            "usage: cargo xtask <check|check-layout|check-skills>",
        )),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn nested_src_is_rejected() -> io::Result<()> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?;
        let directory = env::temp_dir().join(format!(
            "kedra-layout-{}-{}",
            std::process::id(),
            stamp.as_nanos()
        ));
        fs::create_dir_all(directory.join("module/src"))?;
        let result = reject_src(&directory);
        fs::remove_dir_all(&directory)?;
        assert!(result.is_err());
        Ok(())
    }
}
