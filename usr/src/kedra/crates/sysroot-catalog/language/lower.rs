use super::{Binding, Content, Intent, RecipeIntent, SourceIntent};
use crate::{Catalog, Error, Package, Policy, Recipe, Result, Source, SourceOrigin};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use sysroot_engine::{Argument, BuildGraph, BuildNode, Input, SourceFile, source_identity};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Images {
    pub foundation: String,
    #[serde(deserialize_with = "sysroot_engine::deserialize_unique_map")]
    pub builders: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Lowered {
    pub catalog: Catalog,
    pub resources: BTreeMap<String, BTreeMap<String, SourceFile>>,
    pub names: BTreeMap<String, String>,
}

fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}

fn content(value: &Content, external: &BTreeMap<String, Vec<u8>>) -> Result<SourceFile> {
    match value {
        Content::Inline { bytes, executable } => Ok(SourceFile {
            bytes: bytes.as_bytes().to_vec(),
            executable: *executable,
        }),
        Content::File { path, executable } => Ok(SourceFile {
            bytes: external
                .get(path)
                .cloned()
                .ok_or_else(|| invalid("missing admitted external resource"))?,
            executable: *executable,
        }),
    }
}

fn add_tree(
    resources: &mut BTreeMap<String, BTreeMap<String, SourceFile>>,
    files: BTreeMap<String, SourceFile>,
) -> Result<String> {
    let identity = source_identity(&files)?;
    resources.entry(identity.clone()).or_insert(files);
    if resources.values().map(BTreeMap::len).sum::<usize>() > super::MAX_RESOURCES
        || resources
            .values()
            .flat_map(BTreeMap::values)
            .map(|f| f.bytes.len())
            .sum::<usize>()
            > super::MAX_TOTAL
    {
        return Err(invalid("aggregate lowered resource limit exceeded"));
    }
    Ok(identity)
}

fn input_binding(binding: &Binding, dependencies: &BTreeMap<String, String>) -> Result<Argument> {
    match binding {
        Binding::Literal(value) => Ok(Argument::literal(value)),
        Binding::Path { package, path } => {
            let name = dependencies
                .iter()
                .find_map(|(alias, key)| (key == package).then_some(alias))
                .ok_or_else(|| invalid("path references a non-build dependency"))?;
            Ok(Argument::input(name, path))
        }
        Binding::SourceRevision => Err(invalid("source revision cannot enter build inputs")),
    }
}

/// Lower independently accepted symbolic intent after exact image resolution.
/// No filesystem or build operation occurs; callers admit the returned bytes.
pub fn lower(
    intent: &Intent,
    external: &BTreeMap<String, Vec<u8>>,
    images: &Images,
    policy: &Policy,
) -> Result<Lowered> {
    if intent.schema_version != 1
        || intent.recipes.len() > 256
        || images.builders.len() != intent.builders.len()
        || images.builders.keys().ne(intent.builders.keys())
        || !policy.runtime_images.contains(&images.foundation)
        || intent.namespace != policy.namespace
    {
        return Err(invalid(
            "incomplete image resolution or denied frontend namespace",
        ));
    }
    let mut resources = BTreeMap::new();
    let mut nodes = BTreeMap::new();
    let mut origins = BTreeMap::new();
    let names: BTreeMap<_, _> = intent
        .recipes
        .keys()
        .enumerate()
        .map(|(i, key)| (key.clone(), format!("package_{i}")))
        .collect();
    for (key, recipe) in &intent.recipes {
        let builder = images
            .builders
            .get(&recipe.builder)
            .ok_or_else(|| invalid("unresolved builder role"))?;
        if !policy.builder_images.contains(builder) || recipe.runtime != intent.foundation {
            return Err(invalid("denied/incompatible image role"));
        }
        let mut files = BTreeMap::from([("build.sh".into(), content(&recipe.build, external)?)]);
        let raw_source = match &recipe.source {
            SourceIntent::Archive(pin) => {
                if !policy.source_objects.contains(&pin.object) {
                    return Err(Error::Denied("archive source identity".into()));
                }
                origins.insert(
                    pin.object.clone(),
                    SourceOrigin::Archive {
                        url: pin.url.clone(),
                        sha256: pin.sha256.clone(),
                    },
                );
                pin.object.clone()
            }
            SourceIntent::Files(values) => {
                let files = values
                    .iter()
                    .map(|(path, value)| Ok((path.clone(), content(value, external)?)))
                    .collect::<Result<_>>()?;
                let id = add_tree(&mut resources, files)?;
                origins.insert(
                    id.clone(),
                    SourceOrigin::Local {
                        description: "independently admitted inline source".into(),
                    },
                );
                id
            }
        };
        let prepared = !recipe.patches.is_empty()
            || !recipe.files.is_empty()
            || !recipe.replace_files.is_empty();
        if prepared {
            preparation(recipe, external, &mut files)?;
        }
        let resource = add_tree(&mut resources, files)?;
        origins.insert(
            resource.clone(),
            SourceOrigin::Local {
                description: "independently admitted recipe resources".into(),
            },
        );
        let name = &names[key];
        let mut inputs = BTreeMap::from([
            ("source".into(), Input::Object(raw_source.clone())),
            ("resources".into(), Input::Object(resource.clone())),
        ]);
        if prepared {
            let prepare_name = format!("{name}_source");
            nodes.insert(
                prepare_name.clone(),
                BuildNode {
                    builder_image: builder.clone(),
                    runtime_image: images.foundation.clone(),
                    inputs: inputs.clone(),
                    argv: vec![
                        Argument::literal("/usr/bin/python3"),
                        Argument::input("resources", "prepare.py"),
                        Argument::input("source", ""),
                        Argument::output(""),
                        Argument::input("resources", ""),
                    ],
                    env: BTreeMap::new(),
                    runtime_inputs: Vec::new(),
                    timeout_seconds: recipe.timeout,
                },
            );
            inputs.insert("source".into(), Input::Node(prepare_name));
        }
        let dependencies: BTreeMap<_, _> = recipe
            .build_deps
            .iter()
            .chain(&recipe.runtime_deps)
            .map(|(alias, key)| (alias.clone(), key.clone()))
            .collect();
        for (alias, dependency) in &dependencies {
            if ["source", "resources"].contains(&alias.as_str()) {
                return Err(invalid("dependency uses reserved input alias"));
            }
            inputs.insert(
                alias.clone(),
                Input::Node(
                    names
                        .get(dependency)
                        .cloned()
                        .ok_or_else(|| invalid("missing dependency instance"))?,
                ),
            );
        }
        let mut env = recipe
            .env
            .iter()
            .map(|(name, value)| Ok((name.clone(), input_binding(value, &dependencies)?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        env.insert("src".into(), Argument::input("source", ""));
        env.insert("out".into(), Argument::output(""));
        nodes.insert(
            name.clone(),
            BuildNode {
                builder_image: builder.clone(),
                runtime_image: images.foundation.clone(),
                inputs,
                argv: vec![
                    Argument::literal("/bin/sh"),
                    Argument::literal("-eu"),
                    Argument::input("resources", "build.sh"),
                ],
                env,
                runtime_inputs: recipe.runtime_deps.keys().cloned().collect(),
                timeout_seconds: recipe.timeout,
            },
        );
    }
    if nodes.len() > 256 {
        return Err(invalid("complete lowered graph exceeds256 nodes"));
    }
    let mut packages = BTreeMap::new();
    let mut catalog_bytes = 0usize;
    for key in &intent.selected {
        let recipe = intent
            .recipes
            .get(key)
            .ok_or_else(|| invalid("missing selected recipe"))?;
        let root = &names[key];
        let mut graph = BTreeMap::new();
        closure(root, &nodes, &mut graph)?;
        let sources: BTreeSet<_> = graph
            .values()
            .flat_map(|n| n.inputs.values())
            .filter_map(|i| match i {
                Input::Object(id) => Some(id.clone()),
                Input::Node(_) => None,
            })
            .collect();
        let program = recipe
            .exports
            .values()
            .find(|e| e.kind == "command")
            .ok_or_else(|| invalid("selected package has no command export"))?
            .path
            .clone();
        let package = Package {
            version: recipe.version.clone(),
            summary: recipe.summary.clone(),
            license: recipe.license.clone(),
            sources: sources
                .into_iter()
                .map(|object| Source {
                    origin: origins[&object].clone(),
                    object,
                })
                .collect(),
            recipe: Recipe {
                graph: BuildGraph {
                    schema: 1,
                    nodes: graph,
                },
                root: root.clone(),
                program,
            },
        };
        catalog_bytes += serde_json::to_vec(&package)?.len();
        if catalog_bytes > super::MAX_INPUT {
            return Err(invalid("expanded catalog exceeds8MiB"));
        }
        if packages.insert(recipe.name.clone(), package).is_some() {
            return Err(invalid("selected package name conflicts across modules"));
        }
    }
    let catalog = Catalog {
        namespace: intent.namespace.clone(),
        packages,
    };
    // Resource IDs come from accepted bytes, not producer assertions. This
    // extension is invocation-local and never alters the standing caller policy.
    let mut scoped = policy.clone();
    scoped.source_objects.extend(resources.keys().cloned());
    for name in catalog.packages.keys() {
        catalog.resolve(name, &scoped)?;
    }
    Ok(Lowered {
        catalog,
        resources,
        names,
    })
}

fn closure(
    name: &str,
    nodes: &BTreeMap<String, BuildNode>,
    graph: &mut BTreeMap<String, BuildNode>,
) -> Result<()> {
    if graph.contains_key(name) {
        return Ok(());
    }
    if graph.len() >= 256 {
        return Err(invalid("lowered graph exceeds256 nodes"));
    }
    let node = nodes
        .get(name)
        .ok_or_else(|| invalid("missing lowered node"))?;
    graph.insert(name.into(), node.clone());
    for input in node.inputs.values() {
        if let Input::Node(name) = input {
            closure(name, nodes, graph)?;
        }
    }
    Ok(())
}

fn preparation(
    recipe: &RecipeIntent,
    external: &BTreeMap<String, Vec<u8>>,
    files: &mut BTreeMap<String, SourceFile>,
) -> Result<()> {
    let mut overlays = BTreeMap::new();
    for (kind, values) in [("add", &recipe.files), ("replace", &recipe.replace_files)] {
        let mut entries = BTreeMap::new();
        for (name, value) in values {
            let resource = format!("{kind}/{name}");
            let content = content(value, external)?;
            let mode = if content.executable { 0o755 } else { 0o644 };
            files.insert(resource.clone(), content);
            entries.insert(
                name.clone(),
                serde_json::json!({"file": resource, "mode": mode}),
            );
        }
        overlays.insert(kind, entries);
    }
    let mut patches = Vec::new();
    for (i, patch) in recipe.patches.iter().enumerate() {
        let name = format!("patches/{i}.patch");
        files.insert(name.clone(), content(&patch.content, external)?);
        patches.push(serde_json::json!({"file": name, "strip": patch.strip}));
    }
    let plan = serde_json::json!({"patches":patches, "add":overlays["add"], "replace":overlays["replace"]});
    files.insert(
        "prepare.json".into(),
        SourceFile {
            executable: false,
            bytes: serde_json::to_vec(&plan)?,
        },
    );
    files.insert(
        "prepare.py".into(),
        SourceFile {
            executable: false,
            bytes: include_bytes!("prepare.py").to_vec(),
        },
    );
    Ok(())
}
