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

impl Options {
    pub(crate) fn planning(&self) -> bool {
        matches!(
            self.command,
            Command::Check(_)
                | Command::Fmt { .. }
                | Command::Pins(_)
                | Command::Plan { .. }
                | Command::Resolve(_)
        )
    }
}

#[derive(Subcommand)]
enum Command {
    /// Check admitted .kedra definitions and emit pure selected intent.
    Check(crate::catalog_language::Inputs),
    /// Format only the selected files, preserving decoded literal bytes.
    Fmt {
        #[arg(long)]
        check: bool,
        #[arg(required = true)]
        files: Vec<PathBuf>,
    },
    /// Emit deterministic source pins and author recipe metadata for preflight.
    Pins(LanguageSelection),
    /// List the built-in collection or an independently authored Rust catalog.
    List {
        #[arg(long)]
        catalog: Option<PathBuf>,
    },
    /// Emit an authorized graph for sysroot build without opening a store.
    Plan {
        #[command(flatten)]
        selection: Selection,
        /// Publish a complete graph/resource packet to a new file atomically.
        #[arg(long)]
        output: Option<PathBuf>,
    },
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
    #[command(flatten)]
    language: LanguageSelection,
    #[arg(long)]
    store: PathBuf,
    #[arg(long, conflicts_with = "resolution")]
    builder: Option<String>,
    #[arg(long, requires = "entry")]
    resolution: Option<PathBuf>,
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
    #[arg(long, conflicts_with_all = ["builder", "runtime", "entry"])]
    catalog: Option<PathBuf>,
    #[command(flatten)]
    language: LanguageSelection,
    /// Exact independently approved image identities per symbolic builder role.
    #[arg(long, requires = "entry", conflicts_with_all = ["builder", "runtime"])]
    resolution: Option<PathBuf>,
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

#[derive(Args, Default)]
struct LanguageSelection {
    #[arg(long, requires_all = ["input_root", "target", "lock", "target_policy"])]
    entry: Option<String>,
    #[arg(long, requires = "entry")]
    input_root: Option<PathBuf>,
    #[arg(long, requires = "entry")]
    target: Option<String>,
    #[arg(long, requires = "entry")]
    lock: Option<String>,
    #[arg(long, requires = "entry")]
    target_policy: Option<PathBuf>,
}

impl LanguageSelection {
    fn inputs(&self) -> Result<Option<crate::catalog_language::Inputs>> {
        let Some(entry) = &self.entry else {
            return Ok(None);
        };
        Ok(Some(crate::catalog_language::Inputs {
            entry: entry.clone(),
            input_root: self
                .input_root
                .clone()
                .ok_or_else(|| Error::Invalid("--input-root required".into()))?,
            target: self
                .target
                .clone()
                .ok_or_else(|| Error::Invalid("--target required".into()))?,
            lock: self
                .lock
                .clone()
                .ok_or_else(|| Error::Invalid("--lock required".into()))?,
            target_policy: self
                .target_policy
                .clone()
                .ok_or_else(|| Error::Invalid("--target-policy required".into()))?,
        }))
    }
}

struct Authorized {
    package: ResolvedPackage,
    resources: BTreeMap<String, BTreeMap<String, sysroot_engine::SourceFile>>,
    roles: Option<(
        sysroot_catalog::language::Intent,
        sysroot_catalog::language::Images,
    )>,
    frontend: Option<serde_json::Value>,
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

fn resolve(selection: Selection) -> Result<Authorized> {
    let mut policy: Policy = read(&selection.policy)?;
    let mut resources = BTreeMap::new();
    let mut roles = None;
    let mut frontend = None;
    let catalog = if let Some(path) = selection.catalog {
        read::<Catalog>(&path)?
    } else if let Some(inputs) = selection.language.inputs()? {
        let loaded = crate::catalog_language::load(&inputs)?;
        if loaded.intent.namespace == "kedra"
            && inputs.target == "desktop"
            && !loaded.intent.selected.is_empty()
        {
            return Err(Error::Invalid(
                "native x86 source package realization is not supported".into(),
            ));
        }
        let images = if let Some(path) = &selection.resolution {
            read::<sysroot_catalog::language::Images>(path)?
        } else {
            sysroot_catalog::language::Images {
                foundation: selection.runtime.clone().ok_or_else(|| {
                    Error::Invalid(
                        "language lowering requires exact --runtime or --resolution".into(),
                    )
                })?,
                builders: loaded
                    .intent
                    .builders
                    .keys()
                    .map(|role| {
                        Ok((
                            role.clone(),
                            selection.builder.clone().ok_or_else(|| {
                                Error::Invalid(
                                    "language lowering requires exact --builder or --resolution"
                                        .into(),
                                )
                            })?,
                        ))
                    })
                    .collect::<Result<_>>()?,
            }
        };
        let lowered =
            sysroot_catalog::language::lower(&loaded.intent, &loaded.resources, &images, &policy)
                .map_err(|e| Error::Invalid(e.to_string()))?;
        resources = lowered.resources;
        frontend = Some(
            serde_json::json!({"frontend_version":1,"inputs":loaded.inventory,"intent_sha256":hash(&serde_json::to_vec(&serde_json::to_value(&loaded.intent)?)?),"images":images,"policy_sha256":hash(&serde_json::to_vec(&serde_json::to_value(&policy)?)?),"binary_sha256":hash(&read_bytes(&std::env::current_exe()?,64*1024*1024)?)}),
        );
        policy.source_objects.extend(resources.keys().cloned());
        roles = Some((loaded.intent, images));
        lowered.catalog
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
    let package = catalog
        .resolve(&selection.package, &policy)
        .map_err(|error| match error {
            sysroot_catalog::Error::Engine(error) => error,
            other => Error::Invalid(other.to_string()),
        })?;
    Ok(Authorized {
        package,
        resources,
        roles,
        frontend,
    })
}

pub fn run(options: Options) -> Result<ExitCode> {
    match options.command {
        Command::Check(inputs) => {
            let loaded = crate::catalog_language::load(&inputs)?;
            emit(&serde_json::json!({"intent": loaded.intent, "inputs": loaded.inventory}))
        }
        Command::Fmt { check, files } => {
            crate::catalog_language::fmt(files, check)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Pins(language) => match language.inputs()? {
            Some(inputs) => emit(&language_pins(&crate::catalog_language::load(&inputs)?)?),
            None => emit(&pins()),
        },
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
        Command::Plan { selection, output } => {
            let authorized = resolve(selection)?;
            match output {
                Some(path) => emit(&publish_plan(&authorized, &path)?),
                None => emit(&authorized.package.recipe.graph),
            }
        }
        Command::Resolve(selection) => emit(&resolve(selection)?.package),
        Command::Build {
            selection,
            store,
            rebuild,
        } => {
            let authorized = resolve(selection)?;
            let resolved = authorized.package;
            let store = Store::open(&store)?;
            let material = authorized
                .roles
                .as_ref()
                .map(|(intent, images)| observe_roles(&store, intent, images))
                .transpose()?;
            for (expected, files) in &authorized.resources {
                if store.import_resources(files)?.object != *expected {
                    return Err(Error::Invalid("admitted resource identity differs".into()));
                }
            }
            let result = store.build(&resolved.recipe.graph, &resolved.recipe.root, rebuild)?;
            emit(&serde_json::json!({
                "namespace": resolved.namespace, "package": resolved.package,
                "version": resolved.version, "root": resolved.recipe.root,
                "program": resolved.recipe.program, "result": result,
                "resolution_material": material,
            }))
        }
        Command::Contribute(options) => contribute(options),
    }
}

fn observe_roles(
    store: &Store,
    intent: &sysroot_catalog::language::Intent,
    images: &sysroot_catalog::language::Images,
) -> Result<serde_json::Value> {
    let foundation = store.observe_foundation(&images.foundation)?;
    let names = |inventory: &str| {
        inventory
            .lines()
            .filter_map(|line| line.split_once('\t').map(|(name, _)| name.to_owned()))
            .collect::<std::collections::BTreeSet<_>>()
    };
    let installed = names(&foundation.rpm_inventory);
    if !intent.packages.is_subset(&installed) || !intent.remove.is_disjoint(&installed) {
        return Err(Error::Invalid(
            "runtime Fedora requests differ from admitted image material".into(),
        ));
    }
    let mut builders = BTreeMap::new();
    for (role, required) in &intent.builders {
        let image = &images.builders[role];
        let observed = store.observe_foundation(image)?;
        let installed = names(&observed.rpm_inventory);
        if !required.is_subset(&installed) || !intent.packages.is_subset(&installed) {
            return Err(Error::Invalid(
                "compiler Fedora requests differ from admitted image material".into(),
            ));
        }
        builders.insert(role.clone(), serde_json::json!({"image":image,"rpm_sha256":observed.rpm_sha256,"rpm_inventory":observed.rpm_inventory}));
    }
    Ok(serde_json::json!({"foundation":foundation,"builders":builders}))
}

fn publish_plan(authorized: &Authorized, output: &Path) -> Result<serde_json::Value> {
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = output
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::Invalid("invalid packet output name".into()))?;
    if !sysroot_catalog::language::relative(name) || name.contains('/') {
        return Err(Error::Invalid("invalid packet output name".into()));
    }
    let directory = crate::catalog_language::root(parent)?;
    let snapshot =
        sysroot_engine::ManagedSnapshot::create(parent, sysroot_engine::SnapshotPurpose::Catalog)?;
    let packet = snapshot.path().join("plan.tar");
    let mut archive = tar::Builder::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&packet)?,
    );
    let graph = json_bytes(&authorized.package.recipe.graph)?;
    let resource_ids: Vec<_> = authorized.resources.keys().cloned().collect();
    let receipt = json_bytes(
        &serde_json::json!({"schema_version":1,"namespace":authorized.package.namespace,"package":authorized.package.package,"root":authorized.package.recipe.root,"graph_sha256":hash(&graph),"resource_objects":resource_ids,"frontend":authorized.frontend}),
    )?;
    let append =
        |archive: &mut tar::Builder<File>, name: &str, mode: u32, bytes: &[u8]| -> Result<()> {
            let mut header = tar::Header::new_gnu();
            header.set_mode(mode);
            header.set_uid(0);
            header.set_gid(0);
            header.set_mtime(0);
            header.set_size(bytes.len() as u64);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            archive.append_data(&mut header, name, bytes)?;
            Ok(())
        };
    append(&mut archive, "graph.json", 0o600, &graph)?;
    for (id, files) in &authorized.resources {
        if sysroot_engine::source_identity(files)? != *id {
            return Err(Error::Invalid("packet resource identity differs".into()));
        }
        for (path, file) in files {
            append(
                &mut archive,
                &format!("objects/{id}/{path}"),
                if file.executable { 0o755 } else { 0o644 },
                &file.bytes,
            )?;
        }
    }
    append(&mut archive, "frontend.json", 0o600, &receipt)?;
    archive.into_inner()?.sync_all()?;
    let bytes = read_bytes(&packet, 64 * 1024 * 1024)?;
    let source = crate::catalog_language::root(snapshot.path())?;
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    rustix::fs::renameat_with(
        &source,
        "plan.tar",
        &directory,
        name,
        rustix::fs::RenameFlags::NOREPLACE,
    )?;
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    return Err(Error::Invalid(
        "atomic packet publication requires Linux or macOS".into(),
    ));
    directory.sync_all()?;
    snapshot.finish()?;
    Ok(serde_json::json!({"path":output,"sha256":hash(&bytes),"bytes":bytes.len()}))
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
            "usr/src/kedra/crates/sysroot-catalog/legacy.json".to_owned(),
            hash(include_bytes!("../sysroot-catalog/legacy.json")),
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
        "legacy_data":"usr/src/kedra/crates/sysroot-catalog/legacy.json",
    })
}

fn language_pins(loaded: &crate::catalog_language::Loaded) -> Result<serde_json::Value> {
    use sysroot_catalog::language::SourceIntent;
    let intent = &loaded.intent;
    let mut packages = BTreeMap::new();
    for key in &intent.selected {
        let recipe = &intent.recipes[key];
        let mut sources = BTreeMap::new();
        language_sources(
            key,
            intent,
            &mut sources,
            &mut std::collections::BTreeSet::new(),
        )?;
        let program = recipe
            .exports
            .values()
            .find(|e| e.kind == "command")
            .ok_or_else(|| Error::Invalid("selected package lacks command".into()))?
            .path
            .clone();
        if packages.insert(recipe.name.clone(), serde_json::json!({"version":recipe.version, "summary":recipe.summary, "license":recipe.license, "program":program, "sources":sources.into_values().collect::<Vec<_>>(), "exports":recipe.exports})).is_some() {
            return Err(Error::Invalid("selected package name conflict".into()));
        }
    }
    let archives: BTreeMap<_, _> = intent
        .recipes
        .values()
        .filter_map(|recipe| match &recipe.source {
            SourceIntent::Archive(pin) => Some((pin.object.clone(), pin.clone())),
            _ => None,
        })
        .collect();
    let templates: BTreeMap<_, _> = intent.recipes.iter().flat_map(|(key, recipe)| recipe.configs.iter().map(move |(path, template)| (path.clone(), serde_json::json!({"package":key,"body_sha256":hash(template.body.as_bytes()),"bindings":template.bindings})))).collect();
    Ok(serde_json::json!({
        "schema_version":2, "format":"kedra", "frontend_version":1, "namespace":intent.namespace, "target":intent.target,
        "intent_sha256": hash(&serde_json::to_vec(&serde_json::to_value(intent)?)?),
        "inputs": loaded.inventory, "packages":packages, "archives":archives,
        "builders":intent.builders, "templates":templates,
    }))
}

fn language_sources(
    key: &str,
    intent: &sysroot_catalog::language::Intent,
    sources: &mut BTreeMap<String, sysroot_catalog::Source>,
    visited: &mut std::collections::BTreeSet<String>,
) -> Result<()> {
    if !visited.insert(key.into()) {
        return Ok(());
    }
    let recipe = intent
        .recipes
        .get(key)
        .ok_or_else(|| Error::Invalid("missing recipe dependency".into()))?;
    if let sysroot_catalog::language::SourceIntent::Archive(pin) = &recipe.source {
        sources.insert(
            pin.object.clone(),
            sysroot_catalog::Source {
                object: pin.object.clone(),
                origin: sysroot_catalog::SourceOrigin::Archive {
                    url: pin.url.clone(),
                    sha256: pin.sha256.clone(),
                },
            },
        );
    }
    for dependency in recipe
        .build_deps
        .values()
        .chain(recipe.runtime_deps.values())
    {
        language_sources(dependency, intent, sources, visited)?;
    }
    Ok(())
}

fn language_argument(
    template: &sysroot_catalog::language::TemplateIntent,
    source_revision: &str,
    aliases: &BTreeMap<String, String>,
) -> Result<Argument> {
    use sysroot_catalog::language::Binding;
    let mut remaining = template.body.as_str();
    let mut segments = Vec::new();
    while let Some(start) = remaining.find("{{") {
        if start != 0 {
            segments.push(literal(&remaining[..start]));
        }
        let token = &remaining[start + 2..];
        let (name, rest) = token
            .split_once("}}")
            .ok_or_else(|| Error::Invalid("unterminated contribution binding".into()))?;
        let binding = template
            .bindings
            .get(name)
            .ok_or_else(|| Error::Invalid("missing contribution binding".into()))?;
        segments.push(match binding {
            Binding::Literal(value) => literal(value),
            Binding::SourceRevision => literal(source_revision),
            Binding::Path { package, path } => Segment::Input {
                name: aliases.get(package).cloned().ok_or_else(|| {
                    Error::Invalid("contribution references unselected output".into())
                })?,
                path: path.clone(),
            },
        });
        remaining = rest;
    }
    if !remaining.is_empty() {
        segments.push(literal(remaining));
    }
    Ok(Argument(segments))
}

fn language_definition(
    loaded: &crate::catalog_language::Loaded,
    foundation: &str,
    source_revision: &str,
    outputs: BTreeMap<String, String>,
    versions: BTreeMap<String, String>,
) -> Result<SystemDefinition> {
    let mut aliases = BTreeMap::new();
    for (key, recipe) in &loaded.intent.recipes {
        if outputs.contains_key(&recipe.name) {
            aliases.insert(key.clone(), recipe.name.clone());
        }
    }
    let mut files = Vec::new();
    let mut paths = std::collections::BTreeSet::new();
    for recipe in loaded.intent.recipes.values() {
        if !outputs.contains_key(&recipe.name) {
            continue;
        }
        for (path, template) in &recipe.configs {
            sysroot_engine::validate_system_path(path)?;
            if !paths.insert(path.clone()) {
                return Err(Error::Invalid(
                    "conflicting selected configuration paths".into(),
                ));
            }
            files.push(SystemFile {
                path: path.clone(),
                mode: 0o644,
                provenance: "catalog:kedra:2".into(),
                priority: 10,
                replaces: None,
                content: SystemContent::Template(language_argument(
                    template,
                    source_revision,
                    &aliases,
                )?),
            });
        }
    }
    let mut command_dirs = std::collections::BTreeSet::new();
    let mut commands = BTreeMap::new();
    let mut launchers = false;
    for key in &loaded.intent.selected {
        let recipe = &loaded.intent.recipes[key];
        for (alias, export) in recipe.exports.iter().filter(|(_, e)| e.kind == "command") {
            let directory = export.path.rsplit_once('/').map_or("", |(dir, _)| dir);
            command_dirs.insert((recipe.name.clone(), directory.to_owned()));
            let basename = export.path.rsplit('/').next().unwrap_or_default();
            let launcher = if basename != alias {
                let path = format!("/usr/share/kedra/catalog-bin/{alias}");
                if !paths.insert(path.clone()) {
                    return Err(Error::Invalid(
                        "command launcher conflicts with authored config".into(),
                    ));
                }
                let escaped = export
                    .path
                    .replace('"', "\\\"")
                    .replace('$', "\\$")
                    .replace('`', "\\`");
                files.push(SystemFile {
                    path: path.clone(),
                    mode: 0o755,
                    provenance: "catalog:kedra:2".into(),
                    priority: 10,
                    replaces: None,
                    content: SystemContent::Template(Argument(vec![
                        literal("#!/bin/sh\nexec \""),
                        Segment::Input {
                            name: recipe.name.clone(),
                            path: String::new(),
                        },
                        literal(format!("/{escaped}\" \"$@\"\n")),
                    ])),
                });
                launchers = true;
                Some(path)
            } else {
                None
            };
            if commands.insert(alias.clone(),serde_json::json!({"package":recipe.name,"path":export.path,"launcher":launcher})).is_some() { return Err(Error::Invalid("duplicate command export".into())); }
        }
    }
    let mut path_segments = Vec::new();
    if launchers {
        path_segments.push(literal("/usr/share/kedra/catalog-bin:"));
    }
    for (index, (name, path)) in command_dirs.into_iter().enumerate() {
        if index != 0 {
            path_segments.push(literal(":"));
        }
        path_segments.push(Segment::Input { name, path });
    }
    path_segments.push(literal(":${PATH:-/usr/local/bin:/usr/bin:/bin}"));
    for (path, prefix, suffix) in [
        ("/etc/profile.d/kedra-catalog.sh", "export PATH=\"", "\"\n"),
        (
            "/usr/lib/environment.d/60-kedra-catalog.conf",
            "PATH=",
            "\n",
        ),
    ] {
        if !paths.insert(path.into()) {
            return Err(Error::Invalid(
                "package config collides with generated export environment".into(),
            ));
        }
        let mut segments = vec![literal(prefix)];
        segments.extend(path_segments.clone());
        segments.push(literal(suffix));
        files.push(SystemFile {
            path: path.into(),
            mode: 0o644,
            provenance: "catalog:kedra:2".into(),
            priority: 10,
            replaces: None,
            content: SystemContent::Template(Argument(segments)),
        });
    }
    let inventory = json_bytes(
        &serde_json::json!({"schema_version":1,"namespace":loaded.intent.namespace,"format":"kedra","commands":commands,"source_revision":source_revision,"foundation":foundation,"outputs":outputs,"versions":versions}),
    )?;
    files.push(SystemFile {
        path: "/usr/share/kedra/catalog.json".into(),
        mode: 0o644,
        provenance: "catalog:kedra:2".into(),
        priority: 10,
        replaces: None,
        content: SystemContent::Bytes(inventory),
    });
    Ok(SystemDefinition {
        schema: 1,
        platform: sysroot_engine::PLATFORM.into(),
        foundation: foundation.into(),
        provenance: BTreeMap::from([
            ("catalog.namespace".into(), loaded.intent.namespace.clone()),
            ("catalog.source_revision".into(), source_revision.into()),
        ]),
        outputs,
        required_packages: vec![],
        removed_packages: vec![],
        files,
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
    let loaded = options
        .language
        .inputs()?
        .map(|inputs| crate::catalog_language::load(&inputs))
        .transpose()?;
    let mut policy: Policy = read(&options.policy)?;
    let mut contribution_images = None;
    let (catalog, names) = if let Some(loaded) = &loaded {
        let images = if let Some(path) = &options.resolution {
            read::<sysroot_catalog::language::Images>(path)?
        } else {
            sysroot_catalog::language::Images {
                foundation: options.foundation.clone(),
                builders: loaded
                    .intent
                    .builders
                    .keys()
                    .map(|role| {
                        Ok((
                            role.clone(),
                            options.builder.clone().ok_or_else(|| {
                                Error::Invalid("--builder or --resolution required".into())
                            })?,
                        ))
                    })
                    .collect::<Result<_>>()?,
            }
        };
        if images.foundation != options.foundation {
            return Err(Error::Invalid(
                "resolved foundation differs from contribution".into(),
            ));
        }
        let lowered =
            sysroot_catalog::language::lower(&loaded.intent, &loaded.resources, &images, &policy)
                .map_err(|e| Error::Invalid(e.to_string()))?;
        policy
            .source_objects
            .extend(lowered.resources.keys().cloned());
        contribution_images = Some(images);
        (lowered.catalog, lowered.names)
    } else {
        (
            builtin(
                options.builder.as_deref().ok_or_else(|| {
                    Error::Invalid("legacy contribution requires --builder".into())
                })?,
                &options.foundation,
            ),
            BTreeMap::new(),
        )
    };
    let selected = catalog
        .packages
        .keys()
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
    if loaded.is_none() && !material["source"]["package_frontend"].is_null() {
        return Err(Error::Invalid(
            "new-format source requires explicit language contribution".into(),
        ));
    }
    if serde_json::to_vec(&material)? != material_bytes {
        return Err(Error::Invalid(
            "input material must be canonical without duplicate members".into(),
        ));
    }
    let pins_value = match &loaded {
        Some(loaded) => language_pins(loaded)?,
        None => pins(),
    };
    let pins_bytes = json_bytes(&pins_value)?;
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
    if loaded.is_some()
        && material["source"]["package_frontend"]["intent_sha256"] != pins_value["intent_sha256"]
    {
        return Err(Error::Invalid(
            "selected frontend differs from committed source intent".into(),
        ));
    }
    let store = Store::open(&options.store)?;
    if let (Some(loaded), Some(images)) = (&loaded, &contribution_images) {
        let observed = observe_roles(&store, &loaded.intent, images)?;
        let rows = |value: &serde_json::Value| -> Result<Vec<Vec<String>>> {
            let mut rows = value
                .as_str()
                .ok_or_else(|| Error::Invalid("missing observed RPM inventory".into()))?
                .lines()
                .filter(|line| !line.starts_with("gpg-pubkey\t"))
                .map(|line| line.split('\t').map(str::to_owned).collect::<Vec<_>>())
                .collect::<Vec<_>>();
            if rows.is_empty() || rows.iter().any(|row| row.len() != 7) {
                return Err(Error::Invalid("invalid observed RPM inventory".into()));
            }
            rows.sort();
            Ok(rows)
        };
        if serde_json::to_value(rows(&observed["foundation"]["rpm_inventory"])?)?
            != material["packages"]
        {
            return Err(Error::Invalid(
                "contribution foundation RPM material differs from preflight".into(),
            ));
        }
        let builders = observed["builders"]
            .as_object()
            .ok_or_else(|| Error::Invalid("missing observed compiler roles".into()))?
            .iter()
            .map(|(role, value)| Ok((role.clone(), rows(&value["rpm_inventory"])?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        if material["artifacts"]["catalog-builder-rpms"] != hash(&serde_json::to_vec(&builders)?) {
            return Err(Error::Invalid(
                "contribution compiler RPM material differs from preflight".into(),
            ));
        }
    }
    let mut outputs = BTreeMap::new();
    let mut versions = BTreeMap::new();
    let mut objects = BTreeMap::new();
    let mut realized_nodes = BTreeMap::new();
    for package in selected {
        for (node, spec) in &package.plan.specs {
            let object = &package.plan.outputs[node];
            if realized_nodes
                .insert(node.clone(), object.clone())
                .is_some_and(|prior| prior != *object)
            {
                return Err(Error::Invalid(
                    "conflicting realized graph node identity".into(),
                ));
            }
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
    if let Some(loaded) = &loaded {
        let mut runtime = loaded.intent.selected.clone();
        let mut pending: Vec<_> = runtime.iter().cloned().collect();
        while let Some(key) = pending.pop() {
            for dependency in loaded.intent.recipes[&key].runtime_deps.values() {
                if runtime.insert(dependency.clone()) {
                    pending.push(dependency.clone());
                }
            }
        }
        for key in runtime {
            let recipe = &loaded.intent.recipes[&key];
            let object = realized_nodes
                .get(&names[&key])
                .ok_or_else(|| Error::Invalid("verified runtime output missing".into()))?
                .clone();
            if let Some(previous) = outputs.insert(recipe.name.clone(), object.clone())
                && previous != object
            {
                return Err(Error::Invalid(
                    "runtime package output alias conflicts".into(),
                ));
            }
            versions.insert(recipe.name.clone(), recipe.version.clone());
        }
    }
    let definition = match &loaded {
        Some(loaded) => language_definition(
            loaded,
            &options.foundation,
            &options.source_revision,
            outputs.clone(),
            versions,
        )?,
        None => definition(
            &options.foundation,
            &options.source_revision,
            &catalog.namespace,
            outputs.clone(),
            versions,
        )?,
    };
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
