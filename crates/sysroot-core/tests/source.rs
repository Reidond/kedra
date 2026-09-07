use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use sysroot_core::source;

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-source-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let fixture = Self(path);
        fixture.git(&["init", "--template=", "-b", "main"]);
        fixture.write("hosts/desktop/host.toml", "id='desktop'\narchitecture='x86_64'\nimage='ghcr.io/reidond/kedra-desktop'\nfedora_release=44\ncandidate_target=true\nhardware_status='synthetic'\n");
        fixture.write("packages/common.list", "git\nniri\n# comment\n");
        fixture.write("packages/remove.list", "# none\n");
        fixture.write("hosts/desktop/packages.list", "git\nfoot\n");
        fixture
    }
    fn git(&self, args: &[&str]) -> String {
        let mut command = Command::new("git");
        for (name, _) in std::env::vars_os() {
            if name.to_string_lossy().starts_with("GIT_") {
                command.env_remove(name);
            }
        }
        let output = command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .args(["-C"])
            .arg(&self.0)
            .args([
                "-c",
                "user.name=Kedra fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.autocrlf=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
    fn write(&self, path: &str, text: &str) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    fn commit(&self) {
        self.git(&["add", "."]);
        self.git(&["commit", "-m", "fixture"]);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn routes_overrides_and_home_provenance_without_touching_dirty_work() {
    let f = Fixture::new();
    f.write("etc/example.conf", "shared\n");
    f.write("hosts/desktop/etc/example.conf", "host\n");
    f.write("home/.config/example.conf", "abc");
    f.write("home/.gitkeep", "");
    f.write("plugins/private.example", "not a payload\n");
    f.commit();
    f.write("home/.config/example.conf", "unpublished changed bytes");
    f.git(&["add", "home/.config/example.conf"]);
    f.write("home/.config/example.conf", "later unstaged bytes");
    f.write("home/untracked", "must not be captured");
    let before = f.git(&["status", "--porcelain=v1"]);
    let plan = source::plan(&f.0, "desktop").unwrap();
    assert_eq!(plan.packages, ["foot", "git", "niri"]);
    assert_eq!(plan.source_revision, f.git(&["rev-parse", "HEAD"]).trim());
    assert_eq!(plan.files.len(), 2);
    assert_eq!(plan.files[0].source_path, "hosts/desktop/etc/example.conf");
    assert_eq!(plan.files[0].replaces.as_deref(), Some("etc/example.conf"));
    assert_eq!(
        plan.files[1].destination,
        "usr/share/sysroot/home/default/.config/example.conf"
    );
    assert!(plan.files[1].home_baseline);
    assert_eq!(
        plan.files[1].sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(f.git(&["status", "--porcelain=v1"]), before);
    assert_eq!(
        fs::read_to_string(f.0.join("home/.config/example.conf")).unwrap(),
        "later unstaged bytes"
    );
}

#[test]
fn rejects_target_injection_before_git_is_called() {
    for target in [
        "../desktop",
        "--help",
        "Desktop",
        "a/b",
        "",
        "desktop;whoami",
    ] {
        assert!(
            source::plan(Path::new("missing-repo"), target)
                .unwrap_err()
                .to_string()
                .contains("target identifier")
        );
    }
}

#[test]
fn disabled_and_mismatched_targets_fail_closed() {
    for (from, to) in [
        ("candidate_target=true", "candidate_target=false"),
        ("fedora_release=44", "fedora_release=45"),
        ("id='desktop'", "id='xps'"),
        ("architecture='x86_64'", "architecture='aarch64'"),
    ] {
        let f = Fixture::new();
        let original = fs::read_to_string(f.0.join("hosts/desktop/host.toml")).unwrap();
        f.write("hosts/desktop/host.toml", &original.replace(from, to));
        f.commit();
        assert!(source::plan(&f.0, "desktop").is_err());
    }
}

#[test]
fn rejects_package_options_and_conflicting_intent() {
    for bad in [
        "--allowerasing\n",
        "git;echo bad\n",
        "https://example.invalid/package.rpm\n",
    ] {
        let f = Fixture::new();
        f.write("packages/common.list", bad);
        f.commit();
        assert!(
            source::plan(&f.0, "desktop")
                .unwrap_err()
                .to_string()
                .contains("RPM name")
        );
    }
    let f = Fixture::new();
    f.write("packages/remove.list", "git\n");
    f.commit();
    assert!(
        source::plan(&f.0, "desktop")
            .unwrap_err()
            .to_string()
            .contains("both installed and removed")
    );
}

#[test]
fn refuses_symlink_blobs_without_reading_their_targets() {
    let f = Fixture::new();
    f.write("home/link", "../../outside");
    f.commit();
    let blob = f.git(&["rev-parse", "HEAD:home/link"]);
    f.git(&[
        "update-index",
        "--cacheinfo",
        &format!("120000,{},home/link", blob.trim()),
    ]);
    f.git(&["commit", "-m", "symlink fixture"]);
    assert!(
        source::plan(&f.0, "desktop")
            .unwrap_err()
            .to_string()
            .contains("unsupported file mode/path")
    );
}

#[test]
fn rejects_layer_file_directory_collisions_and_reserved_namespaces() {
    let f = Fixture::new();
    f.write("etc/conf", "file");
    f.write("hosts/desktop/etc/conf/child", "nested");
    f.commit();
    assert!(
        source::plan(&f.0, "desktop")
            .unwrap_err()
            .to_string()
            .contains("file/directory collision")
    );
    for path in [
        "usr/etc/forbidden",
        "usr/share/sysroot/home/default/forbidden",
    ] {
        let f = Fixture::new();
        f.write(path, "must fail");
        f.commit();
        assert!(source::plan(&f.0, "desktop").is_err());
    }
}

#[test]
fn archive_is_deterministic_and_preserves_modes_without_worktree_capture() {
    use std::io::Read;
    let f = Fixture::new();
    f.write("usr/bin/demo", "#!/bin/sh\nexit 0\n");
    f.write("home/.config/demo", "reviewed baseline");
    f.commit();
    f.git(&["update-index", "--chmod=+x", "usr/bin/demo"]);
    f.git(&["commit", "-m", "executable"]);
    f.write("usr/bin/demo", "unreviewed application write");
    let first = f.0.join("first.tar");
    let second = f.0.join("second.tar");
    let plan = source::archive(&f.0, "desktop", &first).unwrap();
    source::archive(&f.0, "desktop", &second).unwrap();
    assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
    let mut archive = tar::Archive::new(fs::File::open(first).unwrap());
    let mut seen = 0;
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        assert!(entry.header().entry_type().is_file());
        assert_eq!(entry.header().uid().unwrap(), 0);
        assert_eq!(entry.header().mtime().unwrap(), 0);
        let path = entry.path().unwrap().to_str().unwrap().to_owned();
        let mut bytes = String::new();
        entry.read_to_string(&mut bytes).unwrap();
        match path.as_str() {
            "usr/bin/demo" => {
                assert_eq!(entry.header().mode().unwrap(), 0o755);
                assert_eq!(bytes, "#!/bin/sh\nexit 0\n");
            }
            "usr/share/sysroot/home/default/.config/demo" => assert_eq!(bytes, "reviewed baseline"),
            "usr/share/sysroot/source.json" => {
                let manifest: serde_json::Value = serde_json::from_str(&bytes).unwrap();
                assert_eq!(manifest["source_revision"], plan.source_revision);
                assert_eq!(manifest["files"].as_array().unwrap().len(), 2);
            }
            _ => panic!("unplanned archive entry: {path}"),
        }
        seen += 1;
    }
    assert_eq!(seen, 3);
}

#[test]
fn archive_never_overwrites_existing_output_or_creates_output_for_bad_target() {
    let f = Fixture::new();
    f.commit();
    let output = f.0.join("existing");
    fs::write(&output, "keep this").unwrap();
    assert!(source::archive(&f.0, "desktop", &output).is_err());
    assert_eq!(fs::read_to_string(output).unwrap(), "keep this");
    let absent = f.0.join("absent");
    assert!(source::archive(&f.0, "../desktop", &absent).is_err());
    assert!(!absent.exists());
}

#[test]
fn credential_paths_and_key_material_are_rejected_before_archiving() {
    for (path, content) in [
        ("home/.ssh/id_ed25519", "synthetic credential fixture"),
        ("home/.codex/auth.json", "synthetic credential fixture"),
        (
            "home/.local/state/noctalia/settings.toml",
            "runtime state is not a baseline",
        ),
        ("etc/shadow", "synthetic fixture"),
        ("etc/ssh/ssh_host_ed25519_key", "synthetic fixture"),
        (
            "home/.config/misnamed",
            "-----BEGIN PRIVATE KEY-----\nsynthetic test only",
        ),
    ] {
        let f = Fixture::new();
        f.write(path, content);
        f.commit();
        let output = f.0.join("payload.tar");
        assert!(source::archive(&f.0, "desktop", &output).is_err());
        assert!(!output.exists());
    }
}
