use std::process::ExitCode;

fn main() -> ExitCode {
    eprintln!("sysroot-helper: unavailable; R01/R08/R10 must define and test privileged operations.");
    ExitCode::from(78)
}
