use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "kedra-agents-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        let f = Self(path);
        fs::create_dir_all(f.repo().join("hosts/desktop")).unwrap();
        fs::write(
            f.repo().join("hosts/desktop/host.toml"),
            include_str!("../../../hosts/desktop/host.toml"),
        )
        .unwrap();
        for args in [
            vec!["init", "--quiet"],
            vec!["config", "user.name", "Fixture"],
            vec!["config", "user.email", "fixture@example.invalid"],
            vec!["config", "commit.gpgsign", "false"],
            vec!["config", "core.hooksPath", "/dev/null"],
            vec![
                "remote",
                "add",
                "origin",
                "https://github.com/Reidond/kedra.git",
            ],
            vec!["add", "."],
            vec!["commit", "--quiet", "-m", "fixture"],
        ] {
            source::git(&f.repo(), &args).unwrap();
        }
        f
    }
    fn repo(&self) -> PathBuf {
        self.0.join("repo")
    }
    fn binary(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, "#!/usr/bin/python3\nimport os,sys,json\nif sys.argv[1:] == ['--version']:\n print('fixture-cli 9.2.1')\nelse:\n print(json.dumps({'arguments':sys.argv[1:],'cwd':os.getcwd(),'home':os.environ.get('HOME'),'profile':os.environ.get('CODEX_HOME'),'vault_present':'BW_SESSION' in os.environ}))\n sys.exit(23)\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
    fn options(&self) -> Options {
        Options {
            repo: Some(self.repo()),
            runtime: Runtime::User,
            executable: Some(self.binary("codex")),
            config_scope: ConfigScope::Management,
            host: "desktop".into(),
            print_plan: false,
            arguments: vec![],
        }
    }
    fn select(&self, options: &Options) -> Result<Selection> {
        select(
            "codex",
            options,
            &self.0.join("home"),
            &self.0.join("state"),
            &self.0.join("bundle"),
            std::ffi::OsStr::new(""),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn selection_preserves_dirty_offline_checkout_and_does_not_create_profile() {
    let f = Fixture::new();
    let options = f.options();
    fs::write(f.repo().join("staged"), "selected").unwrap();
    source::git(&f.repo(), &["add", "staged"]).unwrap();
    fs::write(f.repo().join("staged"), "later uncommitted").unwrap();
    fs::write(f.repo().join("untracked"), "keep").unwrap();
    let before = source::git(&f.repo(), &["status", "--porcelain=v1"]).unwrap();
    let selection = f.select(&options).unwrap();
    assert_eq!(selection.version, "9.2.1");
    assert!(!selection.profile.as_ref().unwrap().exists());
    assert_eq!(
        source::git(&f.repo(), &["status", "--porcelain=v1"]).unwrap(),
        before
    );
    assert_eq!(
        source::git(&f.repo(), &["show", ":staged"]).unwrap(),
        b"selected"
    );
    assert!(!selection.deployment_authorized);
}

#[test]
fn missing_personal_runtime_and_wrong_origin_never_fall_back() {
    let f = Fixture::new();
    let mut options = f.options();
    options.executable = Some(f.0.join("missing"));
    assert!(f.select(&options).is_err());
    options.executable = None;
    assert!(f.select(&options).is_err());
    options.executable = Some(f.binary("codex"));
    source::git(
        &f.repo(),
        &[
            "remote",
            "set-url",
            "origin",
            "https://example.invalid/other",
        ],
    )
    .unwrap();
    assert!(f.select(&options).is_err());
}

#[test]
fn runtime_scope_and_version_profiles_are_distinct() {
    let f = Fixture::new();
    let mut options = f.options();
    let personal_runtime = f.select(&options).unwrap();
    let bundle = f.0.join("bundle/codex/bin");
    fs::create_dir_all(&bundle).unwrap();
    fs::copy(options.executable.as_ref().unwrap(), bundle.join("codex")).unwrap();
    options.runtime = Runtime::Bundled;
    options.executable = None;
    let bundled = f.select(&options).unwrap();
    assert_ne!(personal_runtime.profile, bundled.profile);
    options.config_scope = ConfigScope::Personal;
    assert!(f.select(&options).unwrap().profile.is_none());
    options.executable = Some(f.binary("other"));
    assert!(f.select(&options).is_err());
}

#[test]
fn profile_creation_is_private_and_existing_preferences_are_preserved() {
    let f = Fixture::new();
    let selected = f.select(&f.options()).unwrap();
    prepare_profile(&selected).unwrap();
    let profile = selected.profile.as_ref().unwrap();
    assert_eq!(fs::metadata(profile).unwrap().mode() & 0o777, 0o700);
    let config = profile.join("config.toml");
    assert!(fs::read_to_string(&config).unwrap().contains("keyring"));
    fs::write(&config, "# existing preferences\n").unwrap();
    prepare_profile(&selected).unwrap();
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        "# existing preferences\n"
    );
    fs::set_permissions(&config, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(prepare_profile(&selected).is_err());
}

#[test]
fn profile_links_and_checkout_concurrency_are_refused() {
    let f = Fixture::new();
    let selected = f.select(&f.options()).unwrap();
    let profile = selected.profile.as_ref().unwrap();
    fs::create_dir_all(profile.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&f.0, profile).unwrap();
    assert!(prepare_profile(&selected).is_err());
    let first = checkout_lock(&selected.lock_path).unwrap();
    assert!(checkout_lock(&selected.lock_path).is_err());
    drop(first);
    assert!(checkout_lock(&selected.lock_path).is_ok());
}

#[test]
fn child_arguments_and_home_are_preserved_without_a_shell() {
    let f = Fixture::new();
    let selected = f.select(&f.options()).unwrap();
    let arguments = vec![
        OsString::from("hello world"),
        OsString::from("$(touch not-executed); newline\nend"),
        OsString::from("--host"),
    ];
    let mut child = command(&selected, &arguments);
    assert!(
        child
            .get_envs()
            .any(|(key, value)| key == "BW_SESSION" && value.is_none())
    );
    assert!(!child.get_envs().any(|(key, _)| key == "HOME"));
    let output = child.env("HOME", &f.0).output().unwrap();
    assert_eq!(output.status.code(), Some(23));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["arguments"][0], "hello world");
    assert_eq!(json["arguments"][1], "$(touch not-executed); newline\nend");
    assert_eq!(json["home"], f.0.to_str().unwrap());
    assert_eq!(json["cwd"], selected.checkout.to_str().unwrap());
    assert_eq!(json["vault_present"], false);
    assert!(!f.repo().join("not-executed").exists());
}

#[test]
fn nested_explicit_personal_scope_restores_original_controls() {
    let f = Fixture::new();
    let mut selected = f.select(&f.options()).unwrap();
    selected.profile = None;
    selected.config_scope = ConfigScope::Personal;
    let child = command_with_environment(&selected, &[], |key| match key {
        "SYSROOT_ORIGINAL_CODEX_HOME" => Some("/custom/personal-codex".into()),
        "SYSROOT_ORIGINAL_DISABLE_UPDATES" => Some("".into()),
        _ => None,
    });
    let env: std::collections::BTreeMap<_, _> = child.get_envs().collect();
    assert_eq!(
        env[std::ffi::OsStr::new("CODEX_HOME")],
        Some(std::ffi::OsStr::new("/custom/personal-codex"))
    );
    assert_eq!(env[std::ffi::OsStr::new("DISABLE_UPDATES")], None);
    assert_eq!(
        env[std::ffi::OsStr::new("SYSROOT_ORIGINAL_CODEX_HOME")],
        None
    );
}
