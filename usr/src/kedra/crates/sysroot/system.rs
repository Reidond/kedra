//! Kedra committed-source policy over the independent composition engine.
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use sysroot_engine::{
    Error, PLATFORM, Result, Store, SystemContent, SystemDefinition, SystemFile,
    VerifiedComposition, read_system,
};

use crate::source;

const SOURCE_OWNER: &str = "kedra-source:";
const SOURCE_NAMESPACE: &str = "/usr/share/sysroot";

#[derive(Args)]
pub struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Verify exported context bytes against an independently retained identity.
    Verify {
        #[arg(long)]
        context: PathBuf,
        #[arg(long)]
        expected_identity: String,
        /// Owned private directory for bounded temporary verification snapshots.
        #[arg(long)]
        workdir: PathBuf,
    },
    /// Resolve committed configuration and observe exact retained Fedora packages.
    Plan(Inputs),
    /// Export a new deterministic context without installing or running RPM transactions.
    Compose {
        #[command(flatten)]
        inputs: Inputs,
        /// New context directory; existing destinations are refused.
        #[arg(long)]
        output_dir: PathBuf,
    },
}

#[derive(Args)]
struct Inputs {
    /// Checkout whose committed HEAD is resolved once; dirty files are excluded.
    #[arg(long)]
    repo: PathBuf,
    /// Explicit Kedra target; this first composition backend supports qemu-arm64.
    #[arg(long)]
    target: String,
    /// Existing ordinary-user engine store with retained foundation evidence.
    #[arg(long)]
    store: PathBuf,
    /// Exact retained Docker image ID (sha256:...), never a mutable tag.
    #[arg(long)]
    foundation: String,
    /// Optional bounded SystemDefinition JSON emitted by the Rust authoring API.
    #[arg(long)]
    definition: Option<PathBuf>,
}

fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}

fn definition(inputs: &Inputs) -> Result<SystemDefinition> {
    if inputs.target != "qemu-arm64" {
        return Err(invalid("system composition supports only qemu-arm64"));
    }
    let source =
        source::plan(&inputs.repo, &inputs.target).map_err(|error| invalid(error.to_string()))?;
    if source.target.architecture != "aarch64" || source.target.fedora_release != 44 {
        return Err(invalid("system composition requires native ARM Fedora 44"));
    }
    let mut definition = if let Some(path) = &inputs.definition {
        let authored = read_system(path)?;
        if authored.platform != PLATFORM || authored.foundation != inputs.foundation {
            return Err(invalid(
                "authored definition must match the explicit platform and foundation",
            ));
        }
        for key in authored.provenance.keys() {
            if key.starts_with("kedra.") {
                return Err(invalid(
                    "kedra.* provenance is reserved for committed source",
                ));
            }
        }
        for file in &authored.files {
            if file.path == SOURCE_NAMESPACE
                || file.path.starts_with(&format!("{SOURCE_NAMESPACE}/"))
                || file.provenance.starts_with(SOURCE_OWNER)
            {
                return Err(invalid(
                    "source manifests, home baselines and source ownership are reserved",
                ));
            }
        }
        authored
    } else {
        SystemDefinition {
            schema: 1,
            platform: PLATFORM.into(),
            foundation: inputs.foundation.clone(),
            provenance: BTreeMap::new(),
            outputs: BTreeMap::new(),
            required_packages: Vec::new(),
            removed_packages: Vec::new(),
            files: Vec::new(),
        }
    };
    definition.provenance.extend([
        (
            "kedra.source_revision".into(),
            source.source_revision.clone(),
        ),
        ("kedra.target".into(), source.target.id.clone()),
        ("kedra.fedora_release".into(), "44".into()),
        ("kedra.input_scope".into(), source.input_scope.into()),
    ]);
    definition
        .required_packages
        .extend(source.packages.iter().cloned());
    definition
        .removed_packages
        .extend(source.remove_packages.iter().cloned());
    definition.required_packages.sort();
    definition.required_packages.dedup();
    definition.removed_packages.sort();
    definition.removed_packages.dedup();
    let ownership: BTreeMap<_, _> = source
        .files
        .iter()
        .map(|file| (file.destination.as_str(), file.source_path.as_str()))
        .collect();
    source::materialize(&inputs.repo, &source, |path, bytes, mode| {
        let owner = ownership.get(path).copied().unwrap_or("generated-manifest");
        // Assembly's fixed wrapper mode is an assertion on the retained foundation.
        // Its raw Git mode remains unchanged in source.json.
        let mode = if path == "usr/libexec/kedra-session" {
            definition.provenance.insert(
                "kedra.session_mode".into(),
                "0755 (image/assemble.sh)".into(),
            );
            0o755
        } else {
            mode
        };
        definition.files.push(SystemFile {
            path: format!("/{path}"),
            mode,
            provenance: format!("{SOURCE_OWNER}{owner}"),
            priority: 0,
            replaces: None,
            content: SystemContent::Bytes(bytes.to_vec()),
        });
        Ok(())
    })
    .map_err(|error| invalid(error.to_string()))?;
    Ok(definition)
}

fn json(value: &impl serde::Serialize) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(())
}

pub fn run(options: Options) -> Result<()> {
    match options.command {
        Command::Verify {
            context,
            expected_identity,
            workdir,
        } => {
            let verified = VerifiedComposition::open(&context, &expected_identity, &workdir)?;
            json(&serde_json::json!({
                "schema": 1,
                "identity": verified.composition().identity,
                "foundation": verified.composition().plan.foundation.receipt.image,
                "objects": verified.composition().plan.objects.len(),
                "scope": "static context bytes"
            }))
        }
        Command::Plan(inputs) => {
            let definition = definition(&inputs)?;
            json(&Store::open(&inputs.store)?.plan_system(&definition)?)
        }
        Command::Compose { inputs, output_dir } => {
            let definition = definition(&inputs)?;
            json(&Store::open(&inputs.store)?.compose_system(&definition, &output_dir)?)
        }
    }
}
