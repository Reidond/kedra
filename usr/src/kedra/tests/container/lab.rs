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
use kedra_container_tests::environment::{Environment, Extras, Kind};
use kedra_container_tests::image::{self, Overlay, Request, Source};
use kedra_container_tests::session::{self, Display, Host};
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
    /// Image target (desktop, utm); defaults to the engine's native architecture.
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
        /// Show the session live on this Mac through cocoa-way (waypipe) instead of headless.
        #[arg(long)]
        live: bool,
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
    /// Build cocoa-way and waypipe from their pinned sources into target/kedra-lab/tools (macOS).
    LiveTools,
    /// Forward loopback TCP to a waypipe client socket (started by `up --live`).
    #[command(hide = true)]
    Bridge {
        #[arg(long)]
        socket: PathBuf,
        #[arg(long)]
        port_file: PathBuf,
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

fn up(docker: &Docker, args: &ImageArgs, name: &str, display: &str, live: bool) -> Result<()> {
    let display: Display = display.parse()?;
    let lab = image::prepare(docker, &request(docker, args)?)?;
    let extras = Extras::default();
    let host = if live {
        Host::Waypipe {
            bridge_port: kedra_container_tests::live::prepare(name)?,
        }
    } else {
        Host::Headless
    };
    let environment = Environment::start(docker, &lab, &Kind::Lab { name: name.into() }, &extras)?;
    let started = (|| {
        let user = session::create_user(docker, &environment)?;
        session::start(docker, &environment, &user, display, host)
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
    let destination = output.unwrap_or_else(|| {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();
        artifact_root()
            .join("shots")
            .join(format!("{label}-{stamp}.png"))
    });
    let path = session::screenshot(docker, &environment, &session, &destination)?;
    println!("{}", path.display());
    Ok(())
}

/// Push the working tree's home-baseline files into the running account.
/// niri reloads its config on change; Noctalia watches its own files.
fn sync(docker: &Docker, name: Option<&str>, target: Option<&str>) -> Result<()> {
    let environment = Environment::attach(docker, name)?;
    let session = session::existing(docker, &environment)?;
    let target = image::target(docker, target)?;
    let cache = artifact_root().join("cache");
    let outputs = builder::prepare(
        docker,
        &target.id,
        &target.architecture,
        &builder::Stage::Worktree,
        None,
        &cache,
    )?;
    let mut archive = tar::Archive::new(std::fs::File::open(&outputs.payload)?);
    let mut count = 0;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().into_owned();
        let Some(relative) = path.strip_prefix("usr/share/sysroot/home/default/") else {
            continue;
        };
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut entry, &mut data)?;
        let destination = format!("{}/{relative}", session.user.home);
        let mode = format!("{:o}", entry.header().mode()? & 0o777);
        docker.write_file(
            &environment.id,
            &destination,
            &data,
            &session.user.name,
            &mode,
        )?;
        println!("synced ~/{relative}");
        count += 1;
    }
    let validation = environment.exec(docker, &session.exec(["niri", "validate"]))?;
    if validation.exit != 0 {
        eprintln!("{}", validation.stderr_text());
        return Err(Error::Invalid(
            "niri rejected the synced configuration".into(),
        ));
    }
    println!("{count} files synced; niri reloads its configuration automatically.");
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
    let cli = Cli::parse();
    let outcome = Docker::connect().and_then(|docker| match cli.command {
        Command::Image(args) => {
            let lab = image::prepare(&docker, &request(&docker, &args)?)?;
            println!("{}", serde_json::to_string_pretty(&lab).unwrap_or_default());
            Ok(ExitCode::SUCCESS)
        }
        Command::Up {
            image,
            name,
            display,
            live,
        } => up(&docker, &image, &name, &display, live).map(|_| ExitCode::SUCCESS),
        Command::Shot {
            label,
            name,
            output,
        } => shot(&docker, &label, name.as_deref(), output).map(|_| ExitCode::SUCCESS),
        Command::Sync { name, target } => {
            sync(&docker, name.as_deref(), target.as_deref()).map(|_| ExitCode::SUCCESS)
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
        Command::LiveTools => kedra_container_tests::live::build_tools().map(|_| ExitCode::SUCCESS),
        Command::Bridge { socket, port_file } => {
            kedra_container_tests::live::bridge(&socket, &port_file).map(|_| ExitCode::SUCCESS)
        }
        Command::Down { name } => {
            let environment = Environment::attach(&docker, name.as_deref())?;
            let label = environment.name.clone();
            environment.terminate(&docker)?;
            let short = label.strip_prefix("kedra-lab-").unwrap_or(&label);
            kedra_container_tests::live::stop(short);
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
