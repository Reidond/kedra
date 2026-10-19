//! `kedra-lab`: run the Kedra desktop from any stage in a local container.
//!
//! Development tool, not a check runner: `up` starts a retained environment,
//! `shot` saves a screenshot, `sync` pushes working-tree account defaults into
//! the running session, `down` removes a retained environment and `clean`
//! removes this checkout's harness objects and large artifact trees.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use kedra_container_tests::docker::{self, Docker, Exec};
use kedra_container_tests::environment::{Environment, Kind};
use kedra_container_tests::image::{self, Overlay, Request, Source};
use kedra_container_tests::session::{self, Display};
use kedra_container_tests::shared_cache::{self, Cache, Kind as CacheKind};
use kedra_container_tests::shared_store;
use kedra_container_tests::{Error, Result, artifact_root, builder};

#[derive(Parser)]
#[command(
    name = "kedra-lab",
    about = "Run and inspect the Kedra desktop in a local container"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args, Clone)]
struct ImageArgs {
    /// Image target (desktop, qemu-arm64); defaults to the engine's native architecture.
    #[arg(long)]
    target: Option<String>,
    /// stable, run-<id>-<attempt>, sha256:<hex>, builds:<tag>, ref:<reference>, build, build:<revision>, composition:<directory>.
    #[arg(long, default_value = "stable")]
    image: String,
    /// Layer the working tree (payload and sysroot binaries) over the image: worktree or none.
    #[arg(long)]
    overlay: Option<String>,
    /// Directory with prebuilt Linux sysroot and sysroot-helper binaries.
    #[arg(long)]
    binaries: Option<PathBuf>,
    /// Independently selected identity for composition:<directory>.
    #[arg(long)]
    composition_identity: Option<String>,
}

#[derive(Subcommand)]
enum Command {
    /// Native QEMU readiness and development operations.
    Vm {
        #[command(subcommand)]
        command: kedra_container_tests::vm::VmCommand,
    },
    /// Prepare the native source archiver once; hot config sync never builds Rust.
    PrepareSync,
    /// Resolve and build the lab image, then print it.
    Image(ImageArgs),
    /// Build only the verified static composition; optionally run one typed output.
    Replay {
        #[command(flatten)]
        image: ImageArgs,
        #[arg(long, requires = "program")]
        output: Option<String>,
        #[arg(long, requires = "output")]
        program: Option<String>,
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            requires = "output"
        )]
        argv: Vec<String>,
    },
    /// Start a retained desktop environment and print how to use it.
    Up {
        #[command(flatten)]
        image: ImageArgs,
        /// Environment name.
        #[arg(long, default_value = "default")]
        name: String,
        /// Parent output and niri scale, e.g. 2560x1600@2.
        #[arg(long, default_value = "2560x1600@2")]
        display: String,
    },
    /// Save a screenshot of the running session.
    Shot {
        /// File name stem; the PNG goes to target/kedra-lab/shots/.
        #[arg(default_value = "desktop")]
        label: String,
        #[arg(long)]
        name: Option<String>,
        /// Exact output path.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Copy the working tree's account defaults (etc/skel payload) into the running account.
    Sync {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        target: Option<String>,
        /// Capture the resulting desktop with this label after readiness succeeds.
        #[arg(long)]
        shot: Option<String>,
    },
    /// Run a command in the session (as the test user unless --root).
    Exec {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        root: bool,
        #[arg(required = true, trailing_var_arg = true, allow_hyphen_values = true)]
        argv: Vec<String>,
    },
    /// Save journal and unit state of a running environment.
    Logs {
        #[arg(long)]
        name: Option<String>,
    },
    /// List this checkout's harness containers.
    Ls,
    /// Remove a retained lab environment.
    Down {
        #[arg(long)]
        name: Option<String>,
    },
    /// Remove everything this checkout's harness created: every container,
    /// image and volume labeled `kedra.checkout=<id>`, its scoped image tags,
    /// the large artifact trees (cache, derive, composition, native) under
    /// target/ and the native VM's stopped instances, and drop this checkout's
    /// roots on shared VM disks, RPM locks and engine stores. Run reports, screenshots and
    /// the root on the shared QEMU runtime stay. `kedra-lab gc` then deletes
    /// what no checkout roots.
    Clean {
        /// Only remove the containers of one execution.
        #[arg(long, conflicts_with = "vm_runtime")]
        execution: Option<String>,
        /// Also drop the root on the shared QEMU runtime (rebuilding it takes long).
        #[arg(long)]
        vm_runtime: bool,
    },
    /// Delete shared cache entries and shared engine stores that no checkout
    /// roots, after dropping the roots of checkouts that no longer exist, and
    /// report the space freed. Corrupt or unknown entries are reported, not deleted.
    Gc {
        /// Report what would be deleted without deleting it.
        #[arg(long)]
        dry_run: bool,
        /// Delete one entry, KIND/KEY, even a corrupt one, and every root on it.
        #[arg(long, conflicts_with = "dry_run")]
        remove: Option<String>,
    },
}

fn request(docker: &Docker, args: &ImageArgs) -> Result<Request> {
    let source: Source = args.image.parse()?;
    let overlay = match &args.overlay {
        Some(value) => value.parse()?,
        None if matches!(source, Source::Build(_) | Source::Composition(_)) => Overlay::None,
        None => Overlay::Worktree,
    };
    Ok(Request {
        target: image::target(docker, args.target.as_deref())?,
        source,
        overlay,
        binaries: args.binaries.clone(),
        composition_identity: args
            .composition_identity
            .clone()
            .or_else(|| std::env::var("KEDRA_LAB_COMPOSITION_IDENTITY").ok()),
    })
}

fn up(docker: &Docker, args: &ImageArgs, name: &str, display: &str) -> Result<()> {
    let display: Display = display.parse()?;
    let request = request(docker, args)?;
    if !matches!(request.source, Source::Composition(_)) {
        builder::prepare_archiver(&artifact_root().join("cache"))?;
    }
    let lab = image::prepare(docker, &request)?;
    let environment = Environment::start(docker, &lab, &Kind::Lab { name: name.into() })?;
    let started = (|| {
        let user = session::create_user(docker, &environment)?;
        session::start(docker, &environment, &user, display)
    })();
    if let Err(error) = started {
        let evidence = artifact_root().join("lab").join(&environment.name);
        let _ = environment.collect(docker, &evidence);
        let cleanup = environment.terminate(docker);
        eprintln!("kedra-lab: diagnostics in {}", evidence.display());
        if let Err(cleanup) = cleanup {
            eprintln!("kedra-lab: cleanup also failed: {cleanup}");
        }
        return Err(error);
    }
    let elapsed = environment.started.elapsed();
    let container = environment.retain();
    println!(
        "Lab environment {container} is running ({}, {}).",
        lab.target, lab.source
    );
    println!("Image: {} (base {})", lab.reference(), lab.base);
    if let Some(overlay) = &lab.overlay
        && !overlay.unapplied_build_inputs.is_empty()
    {
        println!(
            "Not applied by the overlay: {}",
            overlay.unapplied_build_inputs.join(", ")
        );
    }
    println!(
        "Session ready in {:.1}s at {}x{} scale {}.",
        elapsed.as_secs_f64(),
        display.width,
        display.height,
        display.scale
    );
    println!("  kedra-lab shot            screenshot to target/kedra-lab/shots/");
    println!("  kedra-lab sync            apply working-tree niri/Noctalia defaults live");
    println!("  kedra-lab exec -- CMD     run CMD in the session");
    println!("  kedra-lab down            remove it");
    Ok(())
}

fn shot(docker: &Docker, label: &str, name: Option<&str>, output: Option<PathBuf>) -> Result<()> {
    let environment = Environment::attach(docker, name)?;
    let session = session::existing(docker, &environment)?;
    capture_session(docker, &environment, &session, label, output)
}

fn capture_session(
    docker: &Docker,
    environment: &Environment,
    session: &session::Session,
    label: &str,
    output: Option<PathBuf>,
) -> Result<()> {
    let destination = match output {
        Some(path) => path,
        None => kedra_container_tests::capture::destination(label)?,
    };
    let started = std::time::Instant::now();
    let path = session::screenshot(docker, environment, session, &destination)?;
    let receipt = kedra_container_tests::capture::container_receipt(
        docker,
        environment,
        session,
        &path,
        started.elapsed(),
    )?;
    println!("{}", path.display());
    eprintln!("capture receipt: {}", receipt.display());
    Ok(())
}

/// Validate and apply supported desktop files in the retained disposable account.
fn sync(
    docker: &Docker,
    name: Option<&str>,
    target: Option<&str>,
    capture: Option<&str>,
) -> Result<()> {
    let environment = Environment::attach(docker, name)?;
    let session = session::existing(docker, &environment)?;
    let installed: serde_json::Value = serde_json::from_slice(
        &docker.read_file(&environment.id, "/usr/share/sysroot/source.json")?,
    )
    .map_err(|e| Error::Invalid(format!("installed source manifest: {e}")))?;
    let actual = installed["target"]["id"]
        .as_str()
        .ok_or_else(|| Error::Invalid("installed image has no target".into()))?;
    if target.is_some_and(|target| target != actual) {
        return Err(Error::Invalid(
            "requested sync target differs from the running image".into(),
        ));
    }
    let request = kedra_container_tests::lab_sync::request(actual)?;
    let script = format!(
        "{}/kedra-sync-{}.py",
        session.user.runtime_dir(),
        kedra_container_tests::execution_id()
    );
    docker.write_file(
        &environment.id,
        &script,
        &std::fs::read(kedra_container_tests::harness_dir().join("lab/probes/sync-home.py"))?,
        &session.user.name,
        "0700",
    )?;
    let outcome = environment.run(
        docker,
        &session
            .exec(["python3", &script])
            .stdin(request)
            .timeout(Duration::from_secs(30)),
    );
    let _ = environment.exec(docker, &session.exec(["rm", "-f", &script]));
    let result = outcome?;
    println!("{}", result.stdout_text().trim());
    if let Some(label) = capture {
        capture_session(docker, &environment, &session, label, None)?;
    }
    Ok(())
}

fn exec(docker: &Docker, name: Option<&str>, root: bool, argv: Vec<String>) -> Result<ExitCode> {
    let environment = Environment::attach(docker, name)?;
    let command = if root {
        Exec::new(argv)
    } else {
        session::existing(docker, &environment)?.exec(argv)
    };
    let output = environment.exec(docker, &command.timeout(Duration::from_secs(600)))?;
    use std::io::Write;
    std::io::stdout().write_all(&output.stdout)?;
    std::io::stderr().write_all(&output.stderr)?;
    Ok(ExitCode::from(u8::try_from(output.exit).unwrap_or(1)))
}

fn down(docker: &Docker, selector: Option<&str>) -> Result<String> {
    if selector.is_some_and(|value| value.trim().is_empty()) {
        return Err(Error::Invalid(
            "retained lab environment name must not be empty".into(),
        ));
    }
    let canonical = selector.map(|text| {
        let cleaned: String = text
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '-' {
                    character.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();
        format!(
            "kedra-lab-{}",
            cleaned
                .trim_matches('-')
                .chars()
                .take(40)
                .collect::<String>()
        )
    });
    let mut matching: Vec<_> = docker
        .owned(&[(docker::KIND_LABEL, "lab")])?
        .into_iter()
        .filter(|item| match selector {
            None => true,
            Some(selector) => {
                item.name == selector
                    || canonical.as_deref() == Some(item.name.as_str())
                    || item.id.starts_with(selector)
            }
        })
        .collect();
    matching.sort_by(|left, right| left.name.cmp(&right.name));
    match matching.as_slice() {
        [one] => {
            docker.remove(&one.id)?;
            Ok(one.name.clone())
        }
        [] => Err(Error::Invalid(
            "no retained lab environment; start one with `kedra-lab up`".into(),
        )),
        many => Err(Error::Invalid(format!(
            "{} retained lab environments exist; name one of: {}",
            many.len(),
            many.iter()
                .map(|item| item.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Artifact trees `clean` removes: build caches, engine stores and contexts.
const LARGE_ARTIFACTS: [&str; 4] = ["cache", "derive", "composition", "native"];

/// Image IDs a `sysroot derive` state records in its bindings and journals.
fn bound_images(state: &std::path::Path) -> Result<std::collections::BTreeSet<String>> {
    let mut images = std::collections::BTreeSet::new();
    let entries = match std::fs::read_dir(state) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(images),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if !(name.ends_with("-binding.json") || name.ends_with("-transaction.json")) {
            continue;
        }
        let record: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)
            .map_err(|error| Error::Invalid(format!("{}: {error}", path.display())))?;
        if let Some(image) = record["image"].as_str().filter(|image| !image.is_empty()) {
            images.insert(image.to_owned());
        }
        if let Some(tag) = record["tag"].as_str() {
            images.insert(tag.to_owned());
        }
    }
    Ok(images)
}

/// Whether `path` lies inside this checkout's `target/`; overrides may point elsewhere.
fn inside_target(path: &std::path::Path) -> bool {
    let target = kedra_container_tests::repository_root().join("target");
    match (path.canonicalize(), target.canonicalize()) {
        (Ok(path), Ok(target)) => path.starts_with(target),
        _ => false,
    }
}

/// The native VM home: stopped instances through `vm.py remove`, then trees an
/// earlier per-checkout layout kept there (prepared disks, runtime build tree
/// and, when asked, the runtime).
fn clean_vm(vm_runtime: bool) -> Result<()> {
    let home = kedra_container_tests::vm::qemu_home();
    if !home.exists() {
        return Ok(());
    }
    if !inside_target(&home) {
        eprintln!(
            "kedra-lab: {} is outside this checkout's target/; its VM state was left alone",
            home.display()
        );
        return Ok(());
    }
    for name in kedra_container_tests::vm::remove_instances()? {
        println!("removed native VM instance {name}");
    }
    let mut trees = vec!["images", "build"];
    if vm_runtime {
        trees.push("runtime");
    }
    for name in trees {
        let path = home.join(name);
        if path.symlink_metadata().is_ok() {
            kedra_container_tests::remove_tree(&path)?;
            println!("removed {}", path.display());
        }
    }
    Ok(())
}

/// Drop this checkout's roots in the shared cache: VM disks, RPM locks and
/// engine stores always, the QEMU runtime only when asked. Entries and stores stay for other
/// checkouts until `kedra-lab gc`.
fn unroot_shared(vm_runtime: bool) -> Result<()> {
    let cache = Cache::locate()?;
    let mut kinds = vec![CacheKind::VmDisk, CacheKind::RpmLock];
    if vm_runtime {
        kinds.push(CacheKind::QemuRuntime);
    }
    for name in cache.unroot(&kinds)? {
        println!("dropped shared cache root {name}");
    }
    for store in [shared_store::RPM, shared_store::ENGINE] {
        if cache.unroot_store(store)? {
            println!("dropped shared engine store root {store}");
        }
    }
    Ok(())
}

/// The shared engine store's Docker volume: deleted when no checkout roots it
/// and no build holds it.
fn gc_engine_volume(
    cache: &Cache,
    dry_run: bool,
    report: &mut shared_cache::GcReport,
) -> Result<()> {
    let name = shared_store::engine_volume(cache);
    let docker = match Docker::connect() {
        Ok(docker) => docker,
        Err(error) => {
            report
                .lines
                .push(format!("engine store volume {name} not checked: {error}"));
            return Ok(());
        }
    };
    let Some(labels) = docker.volume_labels(&name)? else {
        return Ok(());
    };
    let bytes = docker.volume_usage(&name)?;
    let shown = bytes.map_or_else(|| "size unknown".to_owned(), shared_cache::size);
    if labels.get(docker::CACHE_LABEL) != Some(&cache.id()) {
        report.lines.push(format!(
            "volume {name} lacks {}={}; left in place",
            docker::CACHE_LABEL,
            cache.id()
        ));
        return Ok(());
    }
    if cache.store_rooted(shared_store::ENGINE)? {
        report
            .lines
            .push(format!("keep engine store volume {name} ({shown}; rooted)"));
        return Ok(());
    }
    let Some(_lock) = cache.try_store_lock(shared_store::ENGINE)? else {
        report
            .lines
            .push(format!("keep engine store volume {name} (in use)"));
        return Ok(());
    };
    if cache.store_rooted(shared_store::ENGINE)? {
        report.lines.push(format!(
            "keep engine store volume {name} (rooted meanwhile)"
        ));
        return Ok(());
    }
    if !dry_run {
        docker.remove_volume(&name)?;
    }
    report.freed += bytes.unwrap_or_default();
    report.lines.push(format!(
        "{} engine store volume {name} ({shown})",
        if dry_run { "would delete" } else { "deleted" }
    ));
    Ok(())
}

fn gc(dry_run: bool, remove: Option<&str>) -> Result<()> {
    let cache = Cache::locate()?;
    if let Some(name) = remove {
        let (kind, key) = name
            .split_once('/')
            .and_then(|(kind, key)| Some((CacheKind::parse(kind)?, key)))
            .ok_or_else(|| {
                Error::Invalid(format!(
                    "--remove takes KIND/KEY, KIND one of {}",
                    CacheKind::ALL.map(CacheKind::name).join(", ")
                ))
            })?;
        let freed = cache.remove(kind, key)?;
        println!("removed {name} ({})", shared_cache::size(freed));
        return Ok(());
    }
    let mut report = cache.gc(dry_run)?;
    // What the cache GC already deleted is reported even if the volume step fails.
    let volume = gc_engine_volume(&cache, dry_run, &mut report);
    for line in &report.lines {
        println!("{line}");
    }
    println!(
        "{} {} in {}",
        if dry_run { "would free" } else { "freed" },
        shared_cache::size(report.freed),
        cache.root().display()
    );
    volume.map_err(|error| {
        Error::Invalid(format!(
            "the engine store volume {} was not checked: {error}",
            shared_store::engine_volume(&cache)
        ))
    })
}

fn clean(docker: &Docker, vm_runtime: bool) -> Result<()> {
    let checkout = kedra_container_tests::checkout_id();
    let cleaned = docker.clean_checkout()?;
    for (kind, names) in [
        ("container", &cleaned.containers),
        ("image", &cleaned.images),
        ("volume", &cleaned.volumes),
    ] {
        for name in names {
            println!("removed {kind} {name}");
        }
    }
    // Instances use the shared disks and runtime: remove them before the roots.
    clean_vm(vm_runtime)?;
    unroot_shared(vm_runtime)?;
    // Only trees inside this checkout's target/; KEDRA_LAB_ARTIFACTS may point elsewhere.
    let root = artifact_root();
    if inside_target(&root) {
        // The engine refuses a final tag whose binding is gone: remove the images
        // the derive state binds before the state itself.
        for tag in docker.remove_engine_images(&bound_images(&root.join("derive"))?)? {
            println!("removed engine image name {tag}");
        }
        for name in LARGE_ARTIFACTS {
            let path = root.join(name);
            if path.symlink_metadata().is_ok() {
                kedra_container_tests::remove_tree(&path)?;
                println!("removed {}", path.display());
            }
        }
    } else if root.exists() {
        eprintln!(
            "kedra-lab: {} is outside this checkout's target/; its trees were left alone",
            root.display()
        );
    }
    println!("cleaned checkout {checkout}");
    Ok(())
}

fn main() -> ExitCode {
    if let Err(error) = kedra_container_tests::cancel::install() {
        eprintln!("kedra-lab: {error}");
        return ExitCode::FAILURE;
    }
    let cli = Cli::parse();
    if let Command::Vm { command } = cli.command {
        return match kedra_container_tests::vm::run(command) {
            Ok(code) => code,
            Err(Error::Interrupted) => ExitCode::from(130),
            Err(error) => {
                eprintln!("kedra-lab: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if let Command::Gc { dry_run, remove } = &cli.command {
        return match gc(*dry_run, remove.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(Error::Interrupted) => ExitCode::from(130),
            Err(error) => {
                eprintln!("kedra-lab: {error}");
                ExitCode::FAILURE
            }
        };
    }
    if matches!(cli.command, Command::PrepareSync) {
        return match builder::prepare_archiver(&artifact_root().join("cache")) {
            Ok(path) => {
                println!("{}", path.display());
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("kedra-lab: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let outcome = Docker::connect().and_then(|docker| match cli.command {
        Command::Vm { .. } => unreachable!("dispatched without Docker"),
        Command::PrepareSync | Command::Gc { .. } => unreachable!("dispatched without Docker"),
        Command::Replay {
            image,
            output,
            program,
            argv,
        } => {
            let replay =
                kedra_container_tests::composition::prepare(&docker, &request(&docker, &image)?)?;
            if let (Some(output), Some(program)) = (output, program) {
                let result = replay.run(&docker, &output, &program, &argv)?;
                use std::io::Write;
                std::io::stdout().write_all(&result.stdout)?;
                std::io::stderr().write_all(&result.stderr)?;
                Ok(if result.exit == 0 {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                })
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&replay)
                        .map_err(|error| Error::Invalid(error.to_string()))?
                );
                Ok(ExitCode::SUCCESS)
            }
        }
        Command::Image(args) => {
            let lab = image::prepare(&docker, &request(&docker, &args)?)?;
            println!("{}", serde_json::to_string_pretty(&lab).unwrap_or_default());
            Ok(ExitCode::SUCCESS)
        }
        Command::Up {
            image,
            name,
            display,
        } => up(&docker, &image, &name, &display).map(|_| ExitCode::SUCCESS),
        Command::Shot {
            label,
            name,
            output,
        } => shot(&docker, &label, name.as_deref(), output).map(|_| ExitCode::SUCCESS),
        Command::Sync { name, target, shot } => {
            sync(&docker, name.as_deref(), target.as_deref(), shot.as_deref())
                .map(|_| ExitCode::SUCCESS)
        }
        Command::Exec { name, root, argv } => exec(&docker, name.as_deref(), root, argv),
        Command::Logs { name } => {
            let environment = Environment::attach(&docker, name.as_deref())?;
            let directory = artifact_root().join("lab").join(&environment.name);
            environment.collect(&docker, &directory)?;
            println!("{}", directory.display());
            Ok(ExitCode::SUCCESS)
        }
        Command::Ls => {
            for item in docker.owned(&[])? {
                println!(
                    "{:<44} {:<9} {:<8} {}",
                    item.name,
                    item.state,
                    item.labels
                        .get(docker::KIND_LABEL)
                        .map(String::as_str)
                        .unwrap_or("-"),
                    item.labels
                        .get(docker::IMAGE_LABEL)
                        .map(String::as_str)
                        .unwrap_or("-"),
                );
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Down { name } => {
            let label = down(&docker, name.as_deref())?;
            println!("removed {label}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Clean {
            execution: Some(execution),
            ..
        } => {
            for item in docker.owned(&[(docker::EXECUTION_LABEL, &execution)])? {
                docker.remove(&item.id)?;
                println!("removed {}", item.name);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Clean {
            execution: None,
            vm_runtime,
        } => clean(&docker, vm_runtime).map(|_| ExitCode::SUCCESS),
    });
    match outcome {
        Ok(code) => code,
        Err(error) => {
            eprintln!("kedra-lab: {error}");
            ExitCode::FAILURE
        }
    }
}
