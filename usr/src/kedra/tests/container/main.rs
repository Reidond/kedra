//! Container scenarios and native tests through `cargo test`.
//!
//! ```text
//! cargo test -p kedra-container-tests --test container               # everything for the native target
//! cargo test -p kedra-container-tests --test container -- desktop    # filter before provisioning
//! cargo test -p kedra-container-tests --test container -- --list     # validate and list, no containers
//! ```
//!
//! Settings (environment): KEDRA_LAB_TARGET, KEDRA_LAB_IMAGE, KEDRA_LAB_OVERLAY,
//! KEDRA_LAB_BINARIES, KEDRA_LAB_ARTIFACTS and KEDRA_LAB_KEEP (never, failed,
//! always; local debugging only, refused when CI is set). See README.md.

mod native;
mod native_artifacts_tests;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use kedra_container_tests::docker::Docker;
use kedra_container_tests::environment::{Environment, Kind};
use kedra_container_tests::image::{self, LabImage, Request};
use kedra_container_tests::report::{self, Report, TestResult};
use kedra_container_tests::scenario::{self, Context, Fixture, Profile, Runner, Scenario, Setup};
use kedra_container_tests::session::{self, Display};
use kedra_container_tests::{Error, artifact_root, clip, execution_id, harness_dir};
use libtest_mimic::{Arguments, Failed, Trial};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Keep {
    Never,
    Failed,
    Always,
}

struct Shared {
    docker: Docker,
    image: LabImage,
    execution: String,
    run_dir: PathBuf,
    keep: Keep,
    results: Mutex<Vec<TestResult>>,
    setups: std::collections::BTreeMap<String, Setup>,
    selected_count: usize,
    composition_identity: Option<String>,
    derivation_identity: Option<String>,
}

enum Body {
    Scenario(Scenario),
    Native(&'static native::NativeTest),
}

impl Body {
    fn fixtures(&self) -> std::collections::BTreeSet<Fixture> {
        match self {
            Body::Scenario(scenario) => scenario.effective_fixtures(),
            Body::Native(test) => {
                let mut fixtures: std::collections::BTreeSet<Fixture> =
                    test.fixtures.iter().copied().collect();
                if test.profile == Profile::Desktop {
                    fixtures.extend([Fixture::TestUser, Fixture::DesktopSession]);
                }
                fixtures
            }
        }
    }

    fn kind(&self) -> &'static str {
        match self {
            Body::Scenario(_) => "scenario",
            Body::Native(_) => "native",
        }
    }
}

/// libtest's filter semantics, applied before any container exists.
fn selected(args: &Arguments, name: &str, ignored: bool) -> bool {
    let matches = |pattern: &String| {
        if args.exact {
            name == pattern
        } else {
            name.contains(pattern.as_str())
        }
    };
    if args.filter.as_ref().is_some_and(|filter| !matches(filter)) {
        return false;
    }
    if args.skip.iter().any(matches) {
        return false;
    }
    if args.ignored {
        return ignored;
    }
    args.include_ignored || !ignored
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn execute(shared: &Shared, name: &str, body: &Body) -> Result<(), Failed> {
    if kedra_container_tests::cancel::requested() {
        return Err(Failed::from("execution interrupted; case was not started"));
    }
    let started = Instant::now();
    let docker = &shared.docker;
    let artifacts = shared.run_dir.join(sanitize(name));
    let mut result = TestResult {
        name: name.to_owned(),
        kind: body.kind(),
        outcome: "passed",
        duration_ms: 0,
        failure: None,
        cleanup_failure: None,
        retained_container: None,
        artifacts: artifacts.display().to_string(),
        steps: Vec::new(),
    };
    let environment = Environment::start(
        docker,
        &shared.image,
        &Kind::Test {
            execution: shared.execution.clone(),
            test: name.to_owned(),
        },
    );
    let failure: Option<String> = match environment {
        Err(error) => Some(format!("environment startup: {error}")),
        Ok(environment) => {
            let fixtures = body.fixtures();
            let mut context = Context {
                docker,
                environment: &environment,
                user: None,
                session: None,
                artifacts: artifacts.clone(),
                target: shared.image.target.clone(),
                composition_identity: shared.composition_identity.clone(),
                derivation_identity: shared.derivation_identity.clone(),
            };
            let prepared: kedra_container_tests::Result<()> = (|| {
                if fixtures.contains(&Fixture::TestUser) {
                    context.user = Some(session::create_user(docker, &environment)?);
                }
                if fixtures.contains(&Fixture::DesktopSession) {
                    let user = context
                        .user
                        .as_ref()
                        .ok_or_else(|| Error::Invalid("session without account".into()))?;
                    context.session = Some(session::start(
                        docker,
                        &environment,
                        user,
                        Display::default(),
                    )?);
                }
                Ok(())
            })();
            let failure = match prepared {
                Err(error) => Some(format!("fixtures: {error}")),
                Ok(()) => match body {
                    Body::Scenario(scenario) => {
                        let mut runner = Runner {
                            context: &context,
                            setups: &shared.setups,
                            records: Vec::new(),
                            deadline: Instant::now() + scenario.deadline(),
                        };
                        let outcome = runner.run(scenario);
                        result.steps = runner.records;
                        outcome.err().map(|failure| failure.to_string())
                    }
                    Body::Native(test) => (test.run)(&context).err().map(|error| error.to_string()),
                },
            };
            let _ = std::fs::create_dir_all(&artifacts);
            let _ = std::fs::write(
                artifacts.join("steps.json"),
                serde_json::to_vec_pretty(&result.steps).unwrap_or_default(),
            );
            if failure.is_some() || shared.keep == Keep::Always {
                let _cleanup = kedra_container_tests::cancel::Cleanup::enter();
                if let Some(session) = &context.session {
                    let _ = session::screenshot(
                        docker,
                        &environment,
                        session,
                        &artifacts.join("screenshots/at-end.png"),
                    );
                }
                if let Err(error) = environment.collect(docker, &artifacts) {
                    eprintln!("{name}: diagnostics incomplete: {error}");
                }
            }
            let keep = match shared.keep {
                Keep::Always => true,
                Keep::Failed => failure.is_some(),
                Keep::Never => false,
            };
            if keep {
                let container = environment.retain();
                eprintln!(
                    "{name}: retained {container}; inspect with `docker exec -it {container} bash`, \
                     remove with `kedra-lab clean --execution {}`",
                    shared.execution
                );
                result.retained_container = Some(container);
            } else if let Err(error) = environment.terminate(docker) {
                result.cleanup_failure = Some(error.to_string());
            }
            failure
        }
    };
    result.duration_ms = started.elapsed().as_millis();
    let outcome = match (&failure, &result.cleanup_failure) {
        (Some(failure), cleanup) => {
            let mut message = failure.clone();
            if let Some(cleanup) = cleanup {
                message.push_str(&format!("\n(cleanup also failed: {cleanup})"));
            }
            message.push_str(&format!("\nartifacts: {}", artifacts.display()));
            Err(Failed::from(message))
        }
        // A cleanup failure fails an otherwise successful test.
        (None, Some(cleanup)) => Err(Failed::from(format!("cleanup failed: {cleanup}"))),
        (None, None) => Ok(()),
    };
    result.outcome = if outcome.is_ok() { "passed" } else { "failed" };
    result.failure = failure.or_else(|| {
        result
            .cleanup_failure
            .clone()
            .map(|cleanup| format!("cleanup failed: {cleanup}"))
    });
    if let Ok(mut results) = shared.results.lock() {
        results.push(result);
        if let Err(error) = report::write(
            &shared.run_dir,
            &Report {
                schema_version: 1,
                interrupted: kedra_container_tests::cancel::requested(),
                selected_count: shared.selected_count,
                execution: &shared.execution,
                image: Some(&shared.image),
                results: &results,
            },
        ) {
            eprintln!("{name}: could not preserve partial results: {error}");
            if outcome.is_ok() {
                if let Some(result) = results.last_mut() {
                    result.outcome = "failed";
                    result.failure = Some(format!("could not preserve test results: {error}"));
                }
                return Err(Failed::from(format!(
                    "could not preserve test results: {error}"
                )));
            }
        }
    }
    outcome
}

fn main() -> ExitCode {
    if let Err(error) = kedra_container_tests::cancel::install() {
        eprintln!("container tests: {error}");
        return ExitCode::FAILURE;
    }
    let mut args = Arguments::from_args();
    // Keep the quiet default; explicit two-worker runs retain a fresh container per case.
    let workers = *args.test_threads.get_or_insert(1);
    if !(1..=2).contains(&workers) {
        eprintln!("container tests: use --test-threads 1 or 2; nothing was provisioned");
        return ExitCode::from(2);
    }
    let targets: Vec<String> = match image::targets() {
        Ok(targets) => targets.into_iter().map(|target| target.id).collect(),
        Err(error) => {
            eprintln!("container tests: {error}");
            return ExitCode::from(2);
        }
    };
    let suite = match scenario::load(&harness_dir(), &targets) {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!("container tests: invalid scenarios, nothing was provisioned:\n{error}");
            return ExitCode::from(2);
        }
    };
    let keep = match std::env::var("KEDRA_LAB_KEEP").unwrap_or_default().as_str() {
        "" | "never" => Keep::Never,
        "failed" => Keep::Failed,
        "always" => Keep::Always,
        other => {
            eprintln!("container tests: KEDRA_LAB_KEEP={other:?} must be never, failed or always");
            return ExitCode::from(2);
        }
    };
    if keep != Keep::Never && std::env::var_os("CI").is_some() {
        eprintln!("container tests: KEDRA_LAB_KEEP is for local debugging and is refused in CI");
        return ExitCode::from(2);
    }

    let mut bodies: Vec<(String, Body)> = suite
        .scenarios
        .into_iter()
        .map(|scenario| {
            (
                format!("scenario::{}", scenario.name),
                Body::Scenario(scenario),
            )
        })
        .collect();
    let composition_source = std::env::var("KEDRA_LAB_IMAGE")
        .is_ok_and(|value| value.trim().starts_with("composition:"));
    let native_plan = std::env::var_os("KEDRA_LAB_NATIVE_PLAN").map(PathBuf::from);
    let derivation_identity = std::env::var("KEDRA_LAB_DERIVATION_IDENTITY").ok();
    let derivation_source = native_plan.is_some() && derivation_identity.is_some();
    if native_plan.is_some() != derivation_identity.is_some()
        || derivation_source && !composition_source
    {
        eprintln!(
            "container tests: native plans require composition source, KEDRA_LAB_NATIVE_PLAN and KEDRA_LAB_DERIVATION_IDENTITY; nothing was provisioned"
        );
        return ExitCode::from(2);
    }
    bodies.extend(
        native::TESTS
            .iter()
            .filter(|test| test.source.applies(composition_source, derivation_source))
            .map(|test| (format!("native::{}", test.name), Body::Native(test))),
    );
    if composition_source {
        bodies.retain(|(_, body)| matches!(body, Body::Native(test) if test.source.applies(true, derivation_source)));
    }
    match std::env::var("KEDRA_LAB_ORDER").as_deref() {
        Ok("reverse") => bodies.reverse(),
        Err(_) | Ok("") | Ok("forward") => {}
        Ok(other) => {
            eprintln!(
                "container tests: KEDRA_LAB_ORDER={other:?} must be forward or reverse; nothing was provisioned"
            );
            return ExitCode::from(2);
        }
    }

    if args.list {
        let trials = bodies
            .into_iter()
            .map(|(name, _)| Trial::test(name, || Ok(())))
            .collect();
        return libtest_mimic::run(&args, trials).exit_code();
    }

    let chosen: Vec<&(String, Body)> = bodies
        .iter()
        .filter(|(name, _)| selected(&args, name, false))
        .collect();
    if chosen.is_empty() {
        return libtest_mimic::run(&args, Vec::new()).exit_code();
    }
    let docker = match Docker::connect() {
        Ok(docker) => docker,
        Err(error) => {
            eprintln!("container tests: no container engine: {error}");
            return ExitCode::from(2);
        }
    };
    let request = match Request::from_env(&docker) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("container tests: {error}");
            return ExitCode::from(2);
        }
    };
    let target = request.target.id.clone();
    let applicable: Vec<String> = chosen
        .iter()
        .filter(|(name, body)| {
            let applies = match body {
                Body::Scenario(scenario) => scenario.applies_to(&target),
                Body::Native(test) => {
                    test.targets.is_empty() || test.targets.contains(&target.as_str())
                }
            };
            if !applies {
                eprintln!("container tests: {name} does not apply to target {target}");
            }
            applies
        })
        .map(|(name, _)| name.clone())
        .collect();
    if applicable.is_empty() {
        eprintln!(
            "container tests: no selected cases apply to {target}; no image or container was prepared (no coverage)"
        );
        return libtest_mimic::run(&args, Vec::new()).exit_code();
    }
    let execution = execution_id();
    let run_dir = artifact_root().join("runs").join(&execution);
    let preparation = Instant::now();
    let prepared_image = match (&native_plan, &derivation_identity) {
        (Some(plan), Some(identity)) => {
            image::prepare_native_system(&docker, &request, plan, identity)
        }
        _ => image::prepare_system(&docker, &request),
    };
    let image = match prepared_image {
        Ok(image) => image,
        Err(error) => {
            eprintln!("container tests: could not prepare the image under test: {error}");
            let _ = report::write(
                &run_dir,
                &Report {
                    schema_version: 1,
                    interrupted: kedra_container_tests::cancel::requested(),
                    selected_count: applicable.len(),
                    execution: &execution,
                    image: None,
                    results: &[],
                },
            );
            return ExitCode::FAILURE;
        }
    };
    eprintln!(
        "container tests: execution {execution}, {} ({}), image ready in {:.1}s; artifacts in {}",
        image.reference(),
        image.source,
        preparation.elapsed().as_secs_f64(),
        run_dir.display()
    );
    let shared = Arc::new(Shared {
        docker,
        image,
        execution: execution.clone(),
        run_dir: run_dir.clone(),
        keep,
        results: Mutex::new(Vec::new()),
        setups: suite.setups,
        selected_count: applicable.len(),
        composition_identity: request.composition_identity.clone(),
        derivation_identity,
    });
    let trials: Vec<Trial> = bodies
        .into_iter()
        .filter(|(name, _)| applicable.contains(name))
        .map(|(name, body)| {
            let shared = Arc::clone(&shared);
            let trial_name = name.clone();
            Trial::test(trial_name, move || execute(&shared, &name, &body))
        })
        .collect();
    let conclusion = libtest_mimic::run(&args, trials);
    let results = shared
        .results
        .lock()
        .map(|results| results.clone())
        .unwrap_or_default();
    if let Err(error) = report::write(
        &run_dir,
        &Report {
            schema_version: 1,
            interrupted: kedra_container_tests::cancel::requested(),
            selected_count: shared.selected_count,
            execution: &execution,
            image: Some(&shared.image),
            results: &results,
        },
    ) {
        eprintln!("container tests: could not write the report: {error}");
        return ExitCode::FAILURE;
    }
    eprintln!(
        "container tests: report {}",
        run_dir.join("report.json").display()
    );
    for result in results.iter().filter(|result| result.outcome == "failed") {
        eprintln!(
            "  FAILED {}: {}",
            result.name,
            clip(result.failure.as_deref().unwrap_or_default(), 600)
        );
    }
    if kedra_container_tests::cancel::requested() {
        ExitCode::from(130)
    } else {
        conclusion.exit_code()
    }
}
