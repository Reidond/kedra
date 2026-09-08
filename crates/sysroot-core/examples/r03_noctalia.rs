//! Path-free Noctalia projection/review demonstration; not a home activator.
use std::io::Read;
use std::process::ExitCode;
use sysroot_core::noctalia::{self, Baseline, Key, Settings, State, Theme};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--project" {
        let version = args[1]
            .to_str()
            .ok_or(noctalia::Error::UnsupportedVersion)?;
        let mut input = String::new();
        std::io::stdin()
            .take((noctalia::MAX_EXPORT + 1) as u64)
            .read_to_string(&mut input)?;
        let settings = noctalia::project(version, &input)?;
        println!("{}", serde_json::to_string(&settings)?);
        return Ok(());
    }
    if !args.is_empty() {
        return Err("accepts no home paths; use --project VERSION with a full effective TOML export on stdin".into());
    }
    let base = Settings {
        theme_mode: Theme::Dark,
        button_borders: true,
        input_borders: true,
    };
    let baseline = Baseline {
        target: "desktop".into(),
        source_path: "home/.config/noctalia/config.toml".into(),
        source_revision: "a".repeat(40),
        settings: base,
    };
    let live = Settings {
        theme_mode: Theme::Light,
        button_borders: false,
        input_borders: false,
    };
    let instance = "0".repeat(32);
    let mut state = State::new(instance.clone(), baseline.clone(), live)?;
    state.stage(Key::ThemeMode)?;
    state.ignore_exact(Key::ButtonBorders)?;
    let selected = state.selection()?;
    state.capture(Settings {
        theme_mode: Theme::Auto,
        ..live
    })?;
    assert_eq!(state.selection()?, selected);
    let published = Settings {
        theme_mode: Theme::Light,
        ..base
    };
    state.record_source_commit(&"b".repeat(40), published)?;
    let serialized = state.to_bytes()?;
    let mut state = State::from_bytes(&serialized, &instance)?;
    let transition = state.prepare(Baseline {
        source_revision: "b".repeat(40),
        settings: published,
        ..baseline
    })?;
    let desired = transition.desired_live();
    assert_eq!(desired.theme_mode, Theme::Auto);
    state.accept(transition, desired)?; // Synthetic receipt only; no filesystem mutation.
    println!("{}", serde_json::to_string_pretty(&state.rows()?)?);
    println!(
        "PASS: pinned selection, local-only exclusion, serialized-state roundtrip and next-baseline transition; no home paths were read or written"
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("r03_noctalia: {error}");
            ExitCode::FAILURE
        }
    }
}
