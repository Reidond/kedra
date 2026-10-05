//! Generated external consumer: only catalog/engine APIs, no Kedra CLI dispatch.
mod definitions;

use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sysroot_catalog::{Catalog, Policy};
use sysroot_engine::{CachePolicy, RunResult, Store, verify_cache_receipt};

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

fn bounded(path: &str, limit: u64) -> Result<Vec<u8>> {
    let file: File = rustix::fs::openat(
        rustix::fs::CWD,
        path,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK
            | rustix::fs::OFlags::CLOEXEC,
        rustix::fs::Mode::empty(),
    )?
    .into();
    if !file.metadata()?.is_file() || file.metadata()?.len() > limit {
        return Err("external consumer input is not a bounded ordinary file".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err("external consumer input exceeds its size limit".into());
    }
    Ok(bytes)
}

fn dispatch(args: &[String]) -> Result<i32> {
    let arguments: Vec<&str> = args.iter().map(String::as_str).collect();
    match arguments.as_slice() {
        ["init", store] => {
            let store = Store::create(Path::new(store))?;
            emit(&serde_json::json!({"store": store.path()}))?;
        }
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
        [
            "substitute",
            store,
            catalog,
            policy,
            package,
            cache_policy,
            receipt,
            signature,
            public_key,
            bundle,
        ] => {
            let catalog: Catalog =
                serde_json::from_slice(&bounded(catalog, sysroot_engine::MAX_JSON)?)?;
            let policy: Policy =
                serde_json::from_slice(&bounded(policy, sysroot_engine::MAX_JSON)?)?;
            let resolved = catalog.resolve(package, &policy)?;
            let selected_scope = format!("{}/{}", resolved.namespace, resolved.package);
            let cache_policy: CachePolicy =
                serde_json::from_slice(&bounded(cache_policy, sysroot_engine::MAX_JSON)?)?;
            if cache_policy.scope != selected_scope {
                return Err("cache scope differs from selected catalog namespace/package".into());
            }
            let expected = resolved
                .plan
                .specs
                .get(&resolved.recipe.root)
                .ok_or("resolved cache recipe is absent")?;
            let public_key = String::from_utf8(bounded(public_key, 4096)?)?;
            let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            let verified = verify_cache_receipt(
                &bounded(receipt, sysroot_engine::MAX_JSON)?,
                &bounded(signature, 1024)?,
                &public_key,
                &cache_policy,
                expected,
                now,
            )?;
            // Recipe identity cannot authorize a namespace or package. Both
            // catalog and producer policy are settled before touching the store.
            let imported =
                Store::open(Path::new(store))?.substitute(Path::new(bundle), &verified)?;
            emit(&serde_json::json!({
                "namespace": resolved.namespace, "package": resolved.package,
                "scope": selected_scope, "signer_fingerprint": verified.signer_fingerprint(),
                "roots": imported.roots,
            }))?;
        }
        ["closure", store, object] => {
            emit(&Store::open(Path::new(store))?.closure(object)?)?;
        }
        ["unpin", store, object] => Store::open(Path::new(store))?.unpin(object)?,
        ["unpin-image", store, image] => Store::open(Path::new(store))?.unpin_image(image)?,
        ["gc", store] => emit(&Store::open(Path::new(store))?.gc(true)?)?,
        ["gc-plan", store] => emit(&Store::open(Path::new(store))?.gc(false)?)?,
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
