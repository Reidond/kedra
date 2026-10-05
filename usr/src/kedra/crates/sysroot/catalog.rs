use clap::{Args, Subcommand};
use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use sysroot_catalog::{Catalog, Policy, ResolvedPackage, builtin};
use sysroot_engine::{
    Argument, Error, Result, Segment, Store, SystemContent, SystemDefinition, SystemFile,
};

#[derive(Args)]
pub struct Options {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Emit deterministic source pins and author recipe metadata for preflight.
    Pins,
    /// List the built-in collection or an independently authored Rust catalog.
    List {
        #[arg(long)]
        catalog: Option<PathBuf>,
    },
    /// Emit an authorized graph for sysroot build without opening a store.
    Plan(Selection),
    /// Show authorized metadata, entrypoint and resolved output identities.
    Resolve(Selection),
    /// Resolve authority, then build in an existing private store.
    Build {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        store: PathBuf,
        #[arg(long)]
        rebuild: bool,
    },
    /// Author installed configuration and receipts from already verified packages.
    Contribute(Contribution),
}

#[derive(Args)]
struct Contribution {
    #[arg(long)]
    store: PathBuf,
    #[arg(long)]
    builder: String,
    #[arg(long)]
    foundation: String,
    #[arg(long)]
    policy: PathBuf,
    #[arg(long)]
    source_revision: String,
    /// Deterministic preflight material, before package realization.
    #[arg(long)]
    input_material: PathBuf,
    #[arg(long)]
    expected_input_material_sha256: String,
    /// New directory; existing destinations are refused.
    #[arg(long)]
    output_dir: PathBuf,
}

#[derive(Serialize)]
struct ContributionReceipt {
    schema_version: u32,
    source_revision: String,
    foundation_image: String,
    input_material_sha256: String,
    definition_sha256: String,
    catalog_pins_sha256: String,
    author_sha256: String,
    outputs: BTreeMap<String, String>,
}

#[derive(Args)]
struct Selection {
    /// Serialized Catalog produced by independent Rust authoring; omit for Kedra.
    #[arg(long, conflicts_with_all = ["builder", "runtime"])]
    catalog: Option<PathBuf>,
    #[arg(long)]
    package: String,
    /// Independently selected exact package/image/source allowlists.
    #[arg(long)]
    policy: PathBuf,
    /// Exact admitted builder image for the built-in collection.
    #[arg(long, requires = "runtime")]
    builder: Option<String>,
    /// Exact admitted runtime image for the built-in collection.
    #[arg(long, requires = "builder")]
    runtime: Option<String>,
}

fn read<T: DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&read_bytes(
        path,
        sysroot_engine::MAX_JSON,
    )?)?)
}

fn read_bytes(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(Error::Invalid(
            "catalog input exceeds its byte limit".into(),
        ));
    }
    Ok(bytes)
}

fn emit(value: &impl Serialize) -> Result<ExitCode> {
    let mut stdout = std::io::stdout().lock();
    serde_json::to_writer(&mut stdout, value)?;
    writeln!(stdout)?;
    Ok(ExitCode::SUCCESS)
}

fn resolve(selection: Selection) -> Result<ResolvedPackage> {
    let catalog = if let Some(path) = selection.catalog {
        read::<Catalog>(&path)?
    } else {
        builtin(
            selection.builder.as_deref().ok_or_else(|| {
                Error::Invalid("built-in catalog requires --builder and --runtime".into())
            })?,
            selection.runtime.as_deref().ok_or_else(|| {
                Error::Invalid("built-in catalog requires --builder and --runtime".into())
            })?,
        )
    };
    let policy: Policy = read(&selection.policy)?;
    catalog
        .resolve(&selection.package, &policy)
        .map_err(|error| match error {
            sysroot_catalog::Error::Engine(error) => error,
            other => Error::Invalid(other.to_string()),
        })
}

pub fn run(options: Options) -> Result<ExitCode> {
    match options.command {
        Command::Pins => emit(&pins()),
        Command::List { catalog } => {
            let catalog = match catalog {
                Some(path) => read::<Catalog>(&path)?,
                None => builtin("", ""),
            };
            let packages: Vec<_> = catalog
                .packages
                .iter()
                .map(|(name, package)| {
                    serde_json::json!({
                        "name": name, "version": package.version, "summary": package.summary,
                        "license": package.license, "sources": package.sources,
                        "program": package.recipe.program,
                    })
                })
                .collect();
            emit(&serde_json::json!({"namespace": catalog.namespace, "packages": packages}))
        }
        Command::Plan(selection) => emit(&resolve(selection)?.recipe.graph),
        Command::Resolve(selection) => emit(&resolve(selection)?),
        Command::Build {
            selection,
            store,
            rebuild,
        } => {
            let resolved = resolve(selection)?;
            let result = Store::open(&store)?.build(
                &resolved.recipe.graph,
                &resolved.recipe.root,
                rebuild,
            )?;
            emit(&serde_json::json!({
                "namespace": resolved.namespace, "package": resolved.package,
                "version": resolved.version, "root": resolved.recipe.root,
                "program": resolved.recipe.program, "result": result,
            }))
        }
        Command::Contribute(options) => contribute(options),
    }
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

const TEMPLATE_ROOT: &str = "usr/src/kedra/image/catalog/templates/";

struct ConfigTemplate {
    relative_path: &'static str,
    contents: &'static str,
}

impl ConfigTemplate {
    fn source_path(&self) -> String {
        format!("{TEMPLATE_ROOT}{}", self.relative_path)
    }

    fn destination(&self) -> String {
        format!("/{}", self.relative_path)
    }
}

macro_rules! config_template {
    ($path:literal) => {
        ConfigTemplate {
            relative_path: $path,
            contents: include_str!(concat!("../../image/catalog/templates/", $path)),
        }
    };
}

const CONFIG_TEMPLATES: &[ConfigTemplate] = &[
    config_template!("etc/profile.d/kedra-catalog.sh"),
    config_template!("usr/lib/environment.d/60-kedra-catalog.conf"),
    config_template!("usr/lib/systemd/system/kedra-catalog-history.service"),
];

#[derive(Clone, Copy, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ConfigBinding {
    Input {
        name: &'static str,
        path: &'static str,
    },
    SourceRevision,
}

const CONFIG_BINDINGS: &[(&str, ConfigBinding)] = &[
    (
        "${KEDRA_CATALOG_JQ_BIN}",
        ConfigBinding::Input {
            name: "jq",
            path: "bin",
        },
    ),
    (
        "${KEDRA_CATALOG_SQLITE_BIN}",
        ConfigBinding::Input {
            name: "sqlite",
            path: "bin",
        },
    ),
    (
        "${KEDRA_CATALOG_SQLITE}",
        ConfigBinding::Input {
            name: "sqlite",
            path: "bin/sqlite3",
        },
    ),
    (
        "${KEDRA_CATALOG_SOURCE_REVISION}",
        ConfigBinding::SourceRevision,
    ),
];

fn pins() -> serde_json::Value {
    let catalog = builtin("", "");
    let packages: BTreeMap<_, _> = catalog
        .packages
        .into_iter()
        .map(|(name, package)| {
            (
                name,
                serde_json::json!({"version": package.version, "license": package.license,
            "sources": package.sources, "program": package.recipe.program}),
            )
        })
        .collect();
    let mut recipes = BTreeMap::from([
        (
            "usr/src/kedra/crates/sysroot-catalog/lib.rs".to_owned(),
            hash(include_bytes!("../sysroot-catalog/lib.rs")),
        ),
        (
            "usr/src/kedra/crates/sysroot-catalog/recipes.rs".to_owned(),
            hash(include_bytes!("../sysroot-catalog/recipes.rs")),
        ),
        (
            "usr/src/kedra/crates/sysroot/catalog.rs".to_owned(),
            hash(include_bytes!("catalog.rs")),
        ),
    ]);
    let templates: BTreeMap<_, _> = CONFIG_TEMPLATES
        .iter()
        .map(|template| {
            let path = template.source_path();
            recipes.insert(path.clone(), hash(template.contents.as_bytes()));
            (path, template.destination())
        })
        .collect();
    let bindings: BTreeMap<_, _> = CONFIG_BINDINGS.iter().copied().collect();
    serde_json::json!({
        "schema_version": 1, "namespace": catalog.namespace, "packages": packages,
        "recipes": recipes, "templates": templates, "bindings": bindings,
    })
}

fn literal(value: impl Into<String>) -> Segment {
    Segment::Literal {
        value: value.into(),
    }
}

// Only catalog-owned tokens are bound here. Native runtime expressions such as
// ${PATH:-...} remain literal configuration; no environment lookup occurs.
fn bind_template(template: &ConfigTemplate, source_revision: &str) -> Result<Argument> {
    if template.contents.contains("$KEDRA_CATALOG_") {
        return Err(Error::Invalid(
            "catalog configuration tokens require braces".into(),
        ));
    }
    let mut remaining = template.contents;
    let mut segments = Vec::new();
    while let Some(start) = remaining.find("${KEDRA_CATALOG_") {
        if start != 0 {
            segments.push(literal(&remaining[..start]));
        }
        let token_start = &remaining[start..];
        let end = token_start
            .find('}')
            .ok_or_else(|| Error::Invalid("unterminated catalog configuration token".into()))?;
        let token = &token_start[..=end];
        let binding = CONFIG_BINDINGS
            .iter()
            .find_map(|(name, binding)| (*name == token).then_some(*binding))
            .ok_or_else(|| {
                Error::Invalid(format!("unknown catalog configuration token {token}"))
            })?;
        segments.push(match binding {
            ConfigBinding::Input { name, path } => Segment::Input {
                name: name.into(),
                path: path.into(),
            },
            ConfigBinding::SourceRevision => literal(source_revision),
        });
        remaining = &token_start[end + 1..];
    }
    if !remaining.is_empty() {
        segments.push(literal(remaining));
    }
    Ok(Argument(segments))
}

fn system_file(template: &ConfigTemplate, source_revision: &str) -> Result<SystemFile> {
    let path = template.destination();
    sysroot_engine::validate_system_path(&path)?;
    Ok(SystemFile {
        path,
        mode: 0o644,
        provenance: "catalog:kedra:1".into(),
        priority: 10,
        replaces: None,
        content: SystemContent::Template(bind_template(template, source_revision)?),
    })
}

fn definition(
    foundation: &str,
    source_revision: &str,
    namespace: &str,
    outputs: BTreeMap<String, String>,
    versions: BTreeMap<String, String>,
) -> Result<SystemDefinition> {
    let inventory = json_bytes(&serde_json::json!({
        "schema_version": 1, "namespace": namespace, "source_revision": source_revision,
        "foundation": foundation, "outputs": outputs, "versions": versions,
    }))?;
    let mut files = CONFIG_TEMPLATES
        .iter()
        .map(|template| system_file(template, source_revision))
        .collect::<Result<Vec<_>>>()?;
    files.push(SystemFile {
        path: "/usr/share/kedra/catalog.json".into(),
        mode: 0o644,
        provenance: "catalog:kedra:1".into(),
        priority: 10,
        replaces: None,
        content: SystemContent::Bytes(inventory),
    });
    Ok(SystemDefinition {
        schema: 1,
        platform: sysroot_engine::PLATFORM.into(),
        foundation: foundation.into(),
        provenance: BTreeMap::from([
            ("catalog.namespace".into(), namespace.into()),
            ("catalog.source_revision".into(), source_revision.into()),
        ]),
        outputs,
        required_packages: vec![],
        removed_packages: vec![],
        files,
    })
}

fn contribute(options: Contribution) -> Result<ExitCode> {
    if options.source_revision.len() != 40
        || !options
            .source_revision
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Invalid(
            "source revision must be an exact lowercase Git commit".into(),
        ));
    }
    let catalog = builtin(&options.builder, &options.foundation);
    let policy: Policy = read(&options.policy)?;
    let selected = ["jq", "sqlite"]
        .into_iter()
        .map(|package| {
            catalog
                .resolve(package, &policy)
                .map_err(|error| Error::Invalid(error.to_string()))
        })
        .collect::<Result<Vec<_>>>()?;
    let material_bytes = read_bytes(&options.input_material, sysroot_engine::MAX_JSON)?;
    let material_hash = hash(&material_bytes);
    if material_hash != options.expected_input_material_sha256 {
        return Err(Error::Invalid("input material hash differs".into()));
    }
    let material: serde_json::Value = serde_json::from_slice(&material_bytes)?;
    if serde_json::to_vec(&material)? != material_bytes {
        return Err(Error::Invalid(
            "input material must be canonical without duplicate members".into(),
        ));
    }
    let pins_bytes = json_bytes(&pins())?;
    let pins_hash = hash(&pins_bytes);
    let author_hash = hash(&read_bytes(&std::env::current_exe()?, 64 * 1024 * 1024)?);
    if material["schema_version"] != 1
        || material["source"]["target"]["id"] != "qemu-arm64"
        || material["source"]["target"]["architecture"] != "aarch64"
        || material["artifacts"]["catalog-pins"] != pins_hash
        || material["artifacts"]["catalog-author"] != author_hash
        || material["artifacts"]["sysroot"] != author_hash
    {
        return Err(Error::Invalid(
            "catalog pins/author/target differ from preflight".into(),
        ));
    }
    let store = Store::open(&options.store)?;
    let mut outputs = BTreeMap::new();
    let mut versions = BTreeMap::new();
    let mut objects = BTreeMap::new();
    for package in selected {
        for (node, spec) in &package.plan.specs {
            let object = &package.plan.outputs[node];
            let actual = store.verify(object)?;
            if actual.runtime_image.as_deref() != Some(options.foundation.as_str())
                || serde_json::to_value(&actual.derivation)? != serde_json::to_value(Some(spec))?
            {
                return Err(Error::Invalid(
                    "realized catalog object differs from selected recipe/foundation".into(),
                ));
            }
            objects.insert(object.clone(), actual);
        }
        let root = &package.plan.outputs[&package.recipe.root];
        store.closure(root)?;
        versions.insert(package.package.clone(), package.version);
        outputs.insert(package.package, root.clone());
    }
    let definition = definition(
        &options.foundation,
        &options.source_revision,
        &catalog.namespace,
        outputs.clone(),
        versions,
    )?;
    let definition_bytes = json_bytes(&definition)?;
    let receipt = ContributionReceipt {
        schema_version: 1,
        source_revision: options.source_revision,
        foundation_image: options.foundation,
        input_material_sha256: material_hash,
        definition_sha256: hash(&definition_bytes),
        catalog_pins_sha256: pins_hash,
        author_sha256: author_hash,
        outputs,
    };
    let receipt_bytes = json_bytes(&receipt)?;
    let results_bytes = json_bytes(&serde_json::json!({"schema_version":1,"objects":objects}))?;
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&options.output_dir)?;
    for (name, bytes) in [
        ("system.json", &definition_bytes),
        ("contribution.json", &receipt_bytes),
        ("catalog-pins.json", &pins_bytes),
        ("catalog-results.json", &results_bytes),
    ] {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(options.output_dir.join(name))?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    File::open(&options.output_dir)?.sync_all()?;
    emit(&serde_json::json!({
        "definition": options.output_dir.join("system.json"),
        "definition_sha256": receipt.definition_sha256,
        "contribution_receipt": options.output_dir.join("contribution.json"),
        "contribution_receipt_sha256": hash(&receipt_bytes),
        "outputs": receipt.outputs,
    }))
}
