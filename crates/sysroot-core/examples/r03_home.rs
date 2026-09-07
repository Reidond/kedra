//! Synthetic R03 experiment only. No caller-supplied home or repository paths.
mod r03;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "R03 synthetic writable-home experiment\nUsage: cargo run -p sysroot-core --example r03_home\nCreates fresh fixtures under target/r03; accepts no home paths. Retains evidence."
        );
        return ExitCode::SUCCESS;
    }
    if !args.is_empty() {
        eprintln!("r03_home: accepts no paths or operational commands; use --help");
        return ExitCode::from(2);
    }
    match r03::demo() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("r03_home: {error}");
            ExitCode::FAILURE
        }
    }
}
