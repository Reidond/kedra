//! Generated external consumer: only catalog/engine APIs, no Kedra CLI dispatch.
mod definitions;

use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;
use sysroot_catalog::{Catalog, Policy};
use sysroot_engine::{RunResult, Store};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const PROFILE: &str = "development";

fn emit(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn output(value: RunResult) -> Result<i32> {
    std::io::stdout().write_all(&value.stdout)?;
    std::io::stderr().write_all(&value.stderr)?;
    if value.stdout_truncated || value.stderr_truncated {
        return Err("runtime output was truncated".into());
    }
    Ok(value.code)
}

fn dispatch(args: &[String]) -> Result<i32> {
    let arguments: Vec<&str> = args.iter().map(String::as_str).collect();
    match arguments.as_slice() {
        ["declare", source, builder, runtime, version] => {
            let (catalog, policy) = definitions::definitions(source, builder, runtime, version);
            emit(&serde_json::json!({"catalog": catalog, "policy": policy}))?;
        }
        ["admit", store, source, builder, runtime] => {
            let store = Store::create(Path::new(store))?;
            store.import_image(builder)?;
            store.import_image(runtime)?;
            emit(&store.import_source(Path::new(source))?)?;
        }
        ["admit-source", store, source] => {
            emit(&Store::open(Path::new(store))?.import_source(Path::new(source))?)?;
        }
        ["resolve", catalog, policy, package] => {
            let catalog: Catalog = serde_json::from_slice(&fs::read(catalog)?)?;
            let policy: Policy = serde_json::from_slice(&fs::read(policy)?)?;
            emit(&catalog.resolve(package, &policy)?)?;
        }
        ["build", store, catalog, policy, package] => {
            let catalog: Catalog = serde_json::from_slice(&fs::read(catalog)?)?;
            let policy: Policy = serde_json::from_slice(&fs::read(policy)?)?;
            // Pure authorization happens before even opening or creating state.
            let resolved = catalog.resolve(package, &policy)?;
            let store = Store::create(Path::new(store))?;
            emit(&store.build(&resolved.recipe.graph, &resolved.recipe.root, false)?)?;
        }
        ["run", store, object] => {
            return output(Store::open(Path::new(store))?.run_output(object, PROGRAM, &[], 60)?);
        }
        ["switch", store, object] => {
            emit(&Store::open(Path::new(store))?.profile_switch(PROFILE, object, PROGRAM, &[])?)?;
        }
        ["profile", store] => {
            emit(&Store::open(Path::new(store))?.profile_list(PROFILE)?)?;
        }
        ["profile-run", store] => {
            return output(Store::open(Path::new(store))?.profile_run(PROFILE, &[], 60)?);
        }
        ["rollback", store] => {
            emit(&Store::open(Path::new(store))?.profile_rollback(PROFILE)?)?;
        }
        ["export", store, object, bundle] => {
            emit(&Store::open(Path::new(store))?.export(object, Path::new(bundle))?)?;
        }
        ["import", store, bundle, sha256] => {
            emit(&Store::create(Path::new(store))?.import(Path::new(bundle), sha256)?)?;
        }
        ["closure", store, object] => {
            emit(&Store::open(Path::new(store))?.closure(object)?)?;
        }
        ["unpin", store, object] => Store::open(Path::new(store))?.unpin(object)?,
        ["unpin-image", store, image] => Store::open(Path::new(store))?.unpin_image(image)?,
        ["gc", store] => emit(&Store::open(Path::new(store))?.gc(true)?)?,
        ["verify", store, object] => emit(&Store::open(Path::new(store))?.verify(object)?)?,
        _ => return Err("unknown operation or wrong argument count".into()),
    }
    Ok(0)
}

fn main() -> ExitCode {
    match dispatch(&std::env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => ExitCode::from(u8::try_from(code).unwrap_or(1)),
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
