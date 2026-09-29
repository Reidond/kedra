//! `kedra-lab`: run the Kedra desktop from any stage in a local container.
//!
//! Development tool, not a check runner: `up` starts a retained environment,
//! `shot` saves a screenshot, `sync` pushes working-tree account defaults into
//! the running session, and `down`/`clean` remove only harness-owned containers.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use clap::{Parser, Subcommand};
use kedra_container_tests::docker::{self, Docker, Exec};
use kedra_container_tests::environment::{Environment, Kind};
use kedra_container_tests::image::{self, Overlay, Request, Source};
use kedra_container_tests::session::{self, Display};
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
    /// stable, run-<id>-<attempt>, sha256:<hex>, builds:<tag>, ref:<reference>, build or build:<revision>.
    #[arg(long, default_value = "stable")]
    image: String,
    /// Layer the working tree (payload and sysroot binaries) over the image: worktree or none.
    #[arg(long)]
    overlay: Option<String>,
    /// Directory with prebuilt Linux sysroot and sysroot-helper binaries.
    #[arg(long)]
    binaries: Option<PathBuf>,
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
    /// List harness-owned containers.
    Ls,
    /// Remove a retained lab environment.
    Down {
        #[arg(long)]
        name: Option<String>,
    },
    /// Remove harness-owned containers: one execution's, or all stopped/retained ones.
    Clean {
        #[arg(long)]
        execution: Option<String>,
        #[arg(long)]
        all: bool,
        /// Also remove harness-built images no container uses, except those named with --keep.
        #[arg(long)]
        images: bool,
        #[arg(long)]
        keep: Vec<String>,
    },
}

fn request(docker: &Docker, args: &ImageArgs) -> Result<Request> {
    let source: Source = args.image.parse()?;
    let overlay = match &args.overlay {
        Some(value) => value.parse()?,
        None if matches!(source, Source::Build(_)) => Overlay::None,
        None => Overlay::Worktree,
    };
    Ok(Request {
        target: image::target(docker, args.target.as_deref())?,
        source,
        overlay,
        binaries: args.binaries.clone(),
    })
}

fn up(docker: &Docker, args: &ImageArgs, name: &str, display: &str) -> Result<()> {
    let display: Display = display.parse()?;
    builder::prepare_archiver(&artifact_root().join("cache"))?;
    let lab = image::prepare(docker, &request(docker, args)?)?;
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
        Command::PrepareSync => unreachable!("dispatched without Docker"),
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
            let environment = Environment::attach(&docker, name.as_deref())?;
            let label = environment.name.clone();
            environment.terminate(&docker)?;
            println!("removed {label}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Clean {
            execution,
            all,
            images,
            keep,
        } => {
            let selected = match (&execution, all) {
                (Some(execution), false) => {
                    docker.owned(&[(docker::EXECUTION_LABEL, execution)])?
                }
                (None, true) => docker.owned(&[])?,
                (None, false) if images => Vec::new(),
                _ => {
                    return Err(Error::Invalid(
                        "pass --execution ID, --all or --images".into(),
                    ));
                }
            };
            if images {
                for tag in docker.prune_images(&keep)? {
                    println!("removed image {tag}");
                }
            }
            for item in selected {
                docker.remove(&item.id)?;
                println!("removed {}", item.name);
            }
            Ok(ExitCode::SUCCESS)
        }
    });
    match outcome {
        Ok(code) => code,
        Err(error) => {
            eprintln!("kedra-lab: {error}");
            ExitCode::FAILURE
        }
    }
}
