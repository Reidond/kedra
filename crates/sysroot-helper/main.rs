use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args_os().len() != 1 {
        eprintln!(
            "sysroot-helper: accepts only the bounded stdin protocol; no paths, commands or verified flags"
        );
        return ExitCode::from(78);
    }
    #[cfg(target_os = "linux")]
    {
        use std::io::Read;
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let mut bytes = Vec::new();
            std::io::stdin()
                .take(sysroot_helper::protocol::MAX_REQUEST as u64 + 1)
                .read_to_end(&mut bytes)?;
            let envelope = sysroot_helper::protocol::decode(&bytes)?;
            sysroot_helper::management::run(envelope.request)
        })();
        if let Err(error) = result {
            eprintln!("sysroot-helper: {error}");
            return ExitCode::from(78);
        }
        ExitCode::SUCCESS
    }
    #[cfg(not(target_os = "linux"))]
    {
        eprintln!("sysroot-helper: installed Linux management is required");
        ExitCode::from(78)
    }
}
