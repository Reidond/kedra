use std::env;
use std::process::ExitCode;

const HELP: &str = "sysroot - Kedra management (bootstrap only)\n\nUsage:\n  sysroot --help\n  sysroot --version\n  sysroot status [--json]\n\nOS deployment, writable-home management, setup, and agent launchers are\nnot implemented. Read docs/HANDOFF.md and RESEARCH.md before adding them.";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["--help"] | ["-h"] => println!("{HELP}"),
        ["--version"] | ["-V"] => println!("sysroot {}", env!("CARGO_PKG_VERSION")),
        ["status"] => println!("Kedra: bootstrap only; no OS management operations are available."),
        ["status", "--json"] => println!("{}", sysroot_core::STATUS_JSON),
        [command, ..] => {
            if let Some(gates) = sysroot_core::research_gate(command) {
                eprintln!("sysroot: {command} is not implemented; complete research {gates}.");
                return ExitCode::from(78);
            }
            eprintln!("sysroot: unsupported command or arguments; run sysroot --help");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
