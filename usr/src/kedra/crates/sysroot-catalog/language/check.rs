use super::parser::Kind;
use super::{
    Block, Declaration, Diagnostic, Export, MAX_DEPTH, MAX_MODULES, MAX_TOTAL, Module, Result,
    Span, Value, fail, import_path, parse, relative,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcePin {
    pub url: String,
    pub sha256: String,
    pub object: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lockfile {
    pub schema_version: u32,
    #[serde(deserialize_with = "sysroot_engine::deserialize_unique_map")]
    pub sources: BTreeMap<String, SourcePin>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetPolicy {
    pub schema_version: u32,
    pub namespace: String,
    pub targets: BTreeSet<String>,
    pub repositories: BTreeSet<String>,
    pub required_packages: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Intent {
    pub schema_version: u32,
    pub namespace: String,
    pub target: String,
    pub foundation: String,
    pub packages: BTreeSet<String>,
    pub remove: BTreeSet<String>,
    pub builders: BTreeMap<String, BTreeSet<String>>,
    pub recipes: BTreeMap<String, RecipeIntent>,
    pub selected: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecipeIntent {
    pub module: String,
    pub name: String,
    pub library: bool,
    pub version: String,
    pub summary: String,
    pub license: String,
    pub builder: String,
    pub runtime: String,
    pub source: SourceIntent,
    pub build: Content,
    pub env: BTreeMap<String, Binding>,
    pub build_deps: BTreeMap<String, String>,
    pub runtime_deps: BTreeMap<String, String>,
    pub files: BTreeMap<String, Content>,
    pub replace_files: BTreeMap<String, Content>,
    pub patches: Vec<PatchIntent>,
    pub exports: BTreeMap<String, ExportIntent>,
    pub configs: BTreeMap<String, TemplateIntent>,
    pub timeout: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SourceIntent {
    Archive(SourcePin),
    Files(BTreeMap<String, Content>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Content {
    Inline { bytes: String, executable: bool },
    File { path: String, executable: bool },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchIntent {
    pub strip: u64,
    pub content: Content,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportIntent {
    pub kind: String,
    pub path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateIntent {
    pub body: String,
    pub bindings: BTreeMap<String, Binding>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Binding {
    Literal(String),
    Path { package: String, path: String },
    SourceRevision,
}

#[derive(Clone, Debug, PartialEq)]
enum Type {
    String,
    Integer,
    Foundation,
    Builder,
    Set,
    Package,
    Fedora,
    Content,
    Script,
    Path,
    Files,
    Patch,
    Replacement,
    Template,
    Source,
    Map,
    List,
    Boolean,
}

#[derive(Clone)]
struct Definition {
    module: String,
    declaration: Declaration,
}

struct Checker {
    modules: BTreeMap<String, Module>,
    definitions: BTreeMap<String, Definition>,
    symbols: BTreeMap<String, BTreeMap<String, String>>,
    lock: Lockfile,
    intent: Intent,
    active: BTreeSet<String>,
    selected_sets: BTreeSet<String>,
    requests: Vec<Request>,
}
struct Request {
    origin: String,
    name: String,
    include: bool,
}

fn root_span() -> Span {
    Span { line: 1, column: 1 }
}
fn hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn rpm(value: &str) -> bool {
    value.len() <= 128
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"+._-".contains(&c))
}

impl Checker {
    fn conflict(&self, first: &str, second: &str, code: &str, message: &str) -> Diagnostic {
        let first = &self.definitions[first];
        let second = &self.definitions[second];
        let mut diagnostic = fail(&first.module, first.declaration.span, code, message);
        diagnostic.related.push(super::Location {
            file: second.module.clone(),
            line: second.declaration.span.line,
            column: second.declaration.span.column,
        });
        diagnostic
    }
    fn cycles(
        &self,
        key: &str,
        active: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<()> {
        if visited.contains(key) {
            return Ok(());
        }
        let definition = &self.definitions[key];
        if depth > MAX_DEPTH || !active.insert(key.into()) {
            return Err(fail(
                &definition.module,
                definition.declaration.span,
                "cycle",
                "declaration dependency cycle/depth exceeded",
            ));
        }
        for field in ["use", "build_deps", "runtime_deps"] {
            let Some(value) = definition.declaration.block.fields.get(field) else {
                continue;
            };
            let values: Vec<&Value> = match &value.kind {
                Kind::List(values) => values.iter().collect(),
                Kind::Block(block) => block.fields.values().collect(),
                _ => Vec::new(),
            };
            for value in values {
                let parts = match &value.kind {
                    Kind::Reference(parts) | Kind::Call(parts, _) => parts,
                    _ => {
                        return Err(self.error(
                            &definition.module,
                            value,
                            "dependency",
                            "invalid dependency reference",
                        ));
                    }
                };
                let dependency = self.lookup(&definition.module, value, parts)?;
                self.cycles(&dependency, active, visited, depth + 1)?;
            }
        }
        active.remove(key);
        visited.insert(key.into());
        Ok(())
    }
    fn imports(
        &self,
        module: &str,
        active: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_DEPTH || !active.insert(module.into()) {
            return Err(fail(
                module,
                root_span(),
                "cycle",
                "import cycle/depth limit",
            ));
        }
        let loaded = self
            .modules
            .get(module)
            .ok_or_else(|| fail(module, root_span(), "import", "missing entry/import module"))?;
        for import in &loaded.imports {
            let path = import_path(module, &import.path)
                .ok_or_else(|| fail(module, import.span, "import", "unsafe local import"))?;
            if !visited.contains(&path) {
                self.imports(&path, active, visited, depth + 1)?;
            }
        }
        active.remove(module);
        visited.insert(module.into());
        Ok(())
    }
    fn reference(
        &self,
        definition: &Definition,
        value: &Value,
        parameters: &BTreeMap<String, String>,
    ) -> Result<String> {
        let Kind::Reference(parts) = &value.kind else {
            return Err(self.error(
                &definition.module,
                value,
                "type",
                "expected image/set reference",
            ));
        };
        if parts.len() == 1
            && let Some(resolved) = parameters.get(&parts[0])
        {
            return Ok(resolved.clone());
        }
        self.lookup(&definition.module, value, parts)
    }
    fn strings(&self, definition: &Definition, field: &str) -> Result<Vec<String>> {
        match definition.declaration.block.fields.get(field) {
            Some(value) => self
                .list(&definition.module, value)?
                .iter()
                .map(|v| self.string(&definition.module, v))
                .collect(),
            None => Ok(Vec::new()),
        }
    }
    fn request(&mut self, origin: &str, name: String, include: bool) -> Result<()> {
        if !rpm(&name) {
            return Err(fail(
                origin.split('#').next().unwrap_or_default(),
                root_span(),
                "rpm",
                "invalid requested RPM name",
            ));
        }
        self.requests.push(Request {
            origin: origin.into(),
            name,
            include,
        });
        if self.requests.len() > super::MAX_VALUES {
            return Err(fail(
                "catalog",
                root_span(),
                "limit",
                "request limit exceeded",
            ));
        }
        Ok(())
    }
    fn select(&mut self, entry: &str, target: &str, policy: &TargetPolicy) -> Result<()> {
        let target_key = format!("{entry}#{target}");
        let definition = self.definitions.get(&target_key).cloned().ok_or_else(|| {
            fail(
                entry,
                root_span(),
                "target",
                "target is not declared in entry",
            )
        })?;
        if definition.declaration.kind != "target" {
            return Err(fail(
                entry,
                definition.declaration.span,
                "target",
                "target name has wrong declaration type",
            ));
        }
        let foundation = self.reference(
            &definition,
            self.field(&definition, "foundation")?,
            &BTreeMap::new(),
        )?;
        let base = self.definitions[&foundation].clone();
        if base.declaration.kind != "foundation"
            || self.integer(&base.module, self.field(&base, "fedora")?)? != 44
        {
            return Err(fail(
                &base.module,
                base.declaration.span,
                "foundation",
                "only explicit Fedora44 foundation is supported",
            ));
        }
        self.intent.foundation = foundation.clone();
        for name in self.strings(&base, "packages")? {
            self.request(&foundation, name, true)?;
        }
        for name in self.strings(&base, "remove")? {
            self.request(&foundation, name, false)?;
        }
        let mut replacements = Vec::new();
        self.select_body(&target_key, &mut replacements, 0)?;
        let mut replaced = BTreeSet::new();
        for (origin, name, action, location) in replacements {
            if !matches!(action.as_str(), "include" | "remove")
                || !replaced.insert((origin.clone(), name.clone()))
            {
                return Err(fail(
                    &location,
                    root_span(),
                    "replacement",
                    "invalid/chained/duplicate replacement",
                ));
            }
            let matching: Vec<_> = self
                .requests
                .iter_mut()
                .filter(|r| r.origin == origin && r.name == name)
                .collect();
            if matching.is_empty() {
                return Err(fail(
                    &location,
                    root_span(),
                    "replacement",
                    "replacement origin/request absent",
                ));
            }
            for request in matching {
                request.include = action == "include";
            }
        }
        for request in &self.requests {
            if request.include {
                self.intent.packages.insert(request.name.clone());
            } else {
                self.intent.remove.insert(request.name.clone());
            }
        }
        if let Some(name) = self
            .intent
            .packages
            .intersection(&self.intent.remove)
            .next()
        {
            let include = self
                .requests
                .iter()
                .find(|r| r.name == *name && r.include)
                .ok_or_else(|| fail(entry, root_span(), "selection", "missing include origin"))?;
            let remove = self
                .requests
                .iter()
                .find(|r| r.name == *name && !r.include)
                .ok_or_else(|| fail(entry, root_span(), "selection", "missing removal origin"))?;
            return Err(self.conflict(
                &include.origin,
                &remove.origin,
                "selection",
                "include/remove conflict",
            ));
        }
        if !policy.required_packages.is_subset(&self.intent.packages) {
            return Err(fail(
                entry,
                root_span(),
                "selection",
                "include/remove conflict or required base request missing",
            ));
        }
        let mut aliases = BTreeMap::new();
        for key in &self.intent.selected {
            for name in self.intent.recipes[key].exports.keys() {
                if let Some(previous) = aliases.insert(name, key) {
                    return Err(self.conflict(
                        previous,
                        key,
                        "export",
                        "ambiguous selected export",
                    ));
                }
            }
        }
        if self.intent.recipes.len() > 256
            || serde_json::to_vec(&self.intent)
                .map_err(|_| fail(entry, root_span(), "encode", "cannot serialize intent"))?
                .len()
                > super::MAX_INPUT
        {
            return Err(fail(
                entry,
                root_span(),
                "limit",
                "intent count/size limit exceeded",
            ));
        }
        Ok(())
    }
    fn select_body(
        &mut self,
        key: &str,
        replacements: &mut Vec<(String, String, String, String)>,
        depth: usize,
    ) -> Result<()> {
        if self.selected_sets.contains(key) {
            return Ok(());
        }
        if depth > MAX_DEPTH || !self.active.insert(key.into()) {
            return Err(fail(
                "catalog",
                root_span(),
                "cycle",
                "set cycle/depth exceeded",
            ));
        }
        let definition = self.definitions[key].clone();
        if let Some(use_sets) = definition.declaration.block.fields.get("use") {
            for value in self.list(&definition.module, use_sets)? {
                let key = self.reference(&definition, value, &BTreeMap::new())?;
                self.select_body(&key, replacements, depth + 1)?;
            }
        }
        for name in self.strings(&definition, "fedora")? {
            self.request(key, name, true)?;
        }
        for name in self.strings(&definition, "remove")? {
            self.request(key, name, false)?;
        }
        if let Some(packages) = definition.declaration.block.fields.get("packages") {
            for value in self.list(&definition.module, packages)? {
                let instance = self.instantiate(&definition, value, &BTreeMap::new(), depth + 1)?;
                if self.intent.recipes[&instance].library {
                    return Err(self.error(
                        &definition.module,
                        value,
                        "selection",
                        "library cannot be a selected command package",
                    ));
                }
                self.intent.selected.insert(instance);
            }
        }
        if let Some(values) = definition.declaration.block.fields.get("replace") {
            for value in self.list(&definition.module, values)? {
                let block = self.construct(
                    &definition.module,
                    value,
                    "replacement",
                    &["origin", "name", "action"],
                )?;
                replacements.push((
                    self.string(&definition.module, &block.fields["origin"])?,
                    self.string(&definition.module, &block.fields["name"])?,
                    self.string(&definition.module, &block.fields["action"])?,
                    definition.module.clone(),
                ));
            }
        }
        self.active.remove(key);
        self.selected_sets.insert(key.into());
        Ok(())
    }
    fn content(&self, definition: &Definition, value: &Value) -> Result<Content> {
        match &value.kind {
            Kind::Tagged(_, bytes) => Ok(Content::Inline {
                bytes: bytes.clone(),
                executable: false,
            }),
            Kind::Call(name, args) if name.as_slice() == ["file"] && args.len() == 1 => {
                let path = self.string(&definition.module, &args[0].1)?;
                let path = import_path(&definition.module, &path).ok_or_else(|| {
                    self.error(
                        &definition.module,
                        value,
                        "path",
                        "external resource must stay inside admitted root",
                    )
                })?;
                Ok(Content::File {
                    path,
                    executable: false,
                })
            }
            Kind::Call(name, args)
                if args.len() == 1
                    && matches!(
                        name.first().map(String::as_str),
                        Some("script" | "executable")
                    ) =>
            {
                let mut result = self.content(definition, &args[0].1)?;
                if name[0] == "executable" {
                    match &mut result {
                        Content::Inline { executable, .. } | Content::File { executable, .. } => {
                            *executable = true
                        }
                    }
                }
                Ok(result)
            }
            _ => Err(self.error(
                &definition.module,
                value,
                "content",
                "expected literal or admitted file content",
            )),
        }
    }
    fn files(&self, definition: &Definition, value: &Value) -> Result<BTreeMap<String, Content>> {
        let Kind::Construct(name, block) = &value.kind else {
            return Err(self.error(&definition.module, value, "type", "expected files"));
        };
        if name != "files" {
            return Err(self.error(&definition.module, value, "type", "expected files"));
        }
        block
            .fields
            .iter()
            .map(|(path, value)| Ok((path.clone(), self.content(definition, value)?)))
            .collect()
    }
    fn binding(
        &self,
        definition: &Definition,
        value: &Value,
        dependencies: &BTreeMap<String, String>,
        own: &str,
        config: bool,
    ) -> Result<Binding> {
        match &value.kind {
            Kind::String(text) => Ok(Binding::Literal(text.clone())),
            Kind::Reference(parts) if parts.as_slice() == ["source_revision"] && config => {
                Ok(Binding::SourceRevision)
            }
            Kind::Call(name, args) if name.as_slice() == ["path"] && args.len() == 2 => {
                let path = self.string(&definition.module, &args[1].1)?;
                if !relative(&path) {
                    return Err(self.error(
                        &definition.module,
                        value,
                        "path",
                        "unsafe output path",
                    ));
                }
                let Kind::Reference(parts) = &args[0].1.kind else {
                    return Err(self.error(
                        &definition.module,
                        value,
                        "type",
                        "path requires output/dependency reference",
                    ));
                };
                let package = if parts.as_slice() == ["self"] && config {
                    own.into()
                } else if parts.len() == 2 && parts[0] == "deps" {
                    dependencies.get(&parts[1]).cloned().ok_or_else(|| {
                        self.error(
                            &definition.module,
                            value,
                            "dependency",
                            "unknown path dependency",
                        )
                    })?
                } else if config {
                    self.lookup(&definition.module, value, parts)?
                } else {
                    return Err(self.error(
                        &definition.module,
                        value,
                        "phase",
                        "self/future output forbidden in build inputs",
                    ));
                };
                Ok(Binding::Path { package, path })
            }
            _ => Err(self.error(
                &definition.module,
                value,
                "binding",
                "unsupported binding or unavailable phase",
            )),
        }
    }
    fn instantiate(
        &mut self,
        caller: &Definition,
        value: &Value,
        outer: &BTreeMap<String, String>,
        depth: usize,
    ) -> Result<String> {
        if depth > MAX_DEPTH {
            return Err(self.error(
                &caller.module,
                value,
                "limit",
                "template expansion depth exceeded",
            ));
        }
        let Kind::Call(parts, args) = &value.kind else {
            return Err(self.error(&caller.module, value, "type", "expected package invocation"));
        };
        let key = self.lookup(&caller.module, value, parts)?;
        let definition = self.definitions[&key].clone();
        let params = self.template_arguments(caller, value, args, outer)?;
        let role = |ty: &str, field: &str| -> Result<String> {
            if let Some(v) = definition.declaration.block.fields.get(field) {
                return self.reference(&definition, v, &params);
            }
            let name = definition
                .declaration
                .parameters
                .iter()
                .find(|(_, t)| *t == ty)
                .map(|(n, _)| n)
                .ok_or_else(|| self.error(&caller.module, value, "role", "missing typed role"))?;
            params
                .get(name)
                .cloned()
                .ok_or_else(|| self.error(&caller.module, value, "role", "unbound typed role"))
        };
        let builder = role("Builder", "builder")?;
        let runtime = role("Foundation", "runtime")?;
        if runtime != self.intent.foundation {
            return Err(self.error(
                &caller.module,
                value,
                "foundation",
                "incompatible runtime foundation",
            ));
        }
        if let Some(existing) = self.intent.recipes.get(&key) {
            if existing.builder != builder || existing.runtime != runtime {
                return Err(self.error(
                    &caller.module,
                    value,
                    "identity",
                    "conflicting package role instantiation",
                ));
            }
            return Ok(key);
        }
        if !self.active.insert(key.clone()) {
            return Err(self.error(
                &caller.module,
                value,
                "cycle",
                "recursive package dependency",
            ));
        }
        if self.intent.recipes.len() >= 256 {
            return Err(self.error(
                &caller.module,
                value,
                "limit",
                "package expansion count exceeded",
            ));
        }
        let compiler = self.definitions[&builder].clone();
        if self.reference(&compiler, self.field(&compiler, "base")?, &BTreeMap::new())? != runtime {
            return Err(self.error(
                &caller.module,
                value,
                "builder",
                "builder base must equal selected foundation",
            ));
        }
        let requirements = self.fedora_requirements(&definition, &key, &compiler)?;
        self.intent
            .builders
            .entry(builder.clone())
            .or_default()
            .extend(requirements);
        let mut dependencies = BTreeMap::new();
        let build_deps = self.instantiate_dependencies(
            &definition,
            "build_deps",
            &params,
            &mut dependencies,
            depth,
        )?;
        let runtime_deps = self.instantiate_dependencies(
            &definition,
            "runtime_deps",
            &params,
            &mut dependencies,
            depth,
        )?;
        let source = self.source(&definition)?;
        let files = self.optional_files(&definition, "files")?;
        let replace_files = self.optional_files(&definition, "replace_files")?;
        if files.keys().any(|name| replace_files.contains_key(name)) {
            return Err(self.error(
                &caller.module,
                value,
                "overlay",
                "path selected by add and replace maps",
            ));
        }
        let patches = self.patches(&definition)?;
        if !files.is_empty() || !replace_files.is_empty() || !patches.is_empty() {
            self.require_overlay_tools(caller, value, &builder, !patches.is_empty())?;
        }
        let env = self.environment(&definition, &dependencies, &key)?;
        let configs = self.configs(&definition, &dependencies, &key)?;
        let timeout = definition
            .declaration
            .block
            .fields
            .get("timeout")
            .map(|v| self.integer(&definition.module, v))
            .transpose()?
            .unwrap_or(600);
        if !(1..=3600).contains(&timeout) {
            return Err(self.error(
                &caller.module,
                value,
                "timeout",
                "build timeout must be1..3600",
            ));
        }
        let recipe = RecipeIntent {
            module: definition.module.clone(),
            name: definition.declaration.name.clone(),
            library: definition.declaration.kind == "library",
            version: self.string(&definition.module, self.field(&definition, "version")?)?,
            summary: self.string(&definition.module, self.field(&definition, "summary")?)?,
            license: self.string(&definition.module, self.field(&definition, "license")?)?,
            source,
            build: self.content(&definition, self.field(&definition, "build")?)?,
            builder,
            runtime,
            env,
            build_deps,
            runtime_deps,
            files,
            replace_files,
            patches,
            exports: definition
                .declaration
                .block
                .exports
                .iter()
                .map(|e| {
                    (
                        e.name.clone(),
                        ExportIntent {
                            kind: e.kind.clone(),
                            path: e.path.clone(),
                        },
                    )
                })
                .collect(),
            configs,
            timeout,
        };
        self.intent.recipes.insert(key.clone(), recipe);
        self.active.remove(&key);
        Ok(key)
    }
    fn template_arguments(
        &self,
        caller: &Definition,
        value: &Value,
        args: &[(Option<String>, Value)],
        outer: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<String, String>> {
        let mut params = BTreeMap::new();
        for (name, argument) in args {
            params.insert(
                name.clone().ok_or_else(|| {
                    self.error(
                        &caller.module,
                        value,
                        "arguments",
                        "named template arguments required",
                    )
                })?,
                self.reference(caller, argument, outer)?,
            );
        }
        Ok(params)
    }
    fn fedora_requirements(
        &mut self,
        definition: &Definition,
        key: &str,
        compiler: &Definition,
    ) -> Result<BTreeSet<String>> {
        let mut requirements: BTreeSet<_> =
            self.strings(compiler, "packages")?.into_iter().collect();
        for (field, build) in [("build_requires", true), ("runtime_requires", false)] {
            if let Some(values) = definition.declaration.block.fields.get(field) {
                for v in self.list(&definition.module, values)? {
                    let name = self.requirement_name(definition, v)?;
                    if build {
                        requirements.insert(name);
                    } else {
                        self.request(key, name, true)?;
                    }
                }
            }
        }
        Ok(requirements)
    }
    fn requirement_name(&self, definition: &Definition, value: &Value) -> Result<String> {
        let Kind::Call(_, args) = &value.kind else {
            return Err(self.error(
                &definition.module,
                value,
                "rpm",
                "expected Fedora requirement",
            ));
        };
        let name = self.string(&definition.module, &args[0].1)?;
        if !rpm(&name) {
            return Err(self.error(&definition.module, value, "rpm", "invalid RPM requirement"));
        }
        Ok(name)
    }
    fn instantiate_dependencies(
        &mut self,
        definition: &Definition,
        field: &str,
        params: &BTreeMap<String, String>,
        dependencies: &mut BTreeMap<String, String>,
        depth: usize,
    ) -> Result<BTreeMap<String, String>> {
        let mut output = BTreeMap::new();
        if let Some(map) = definition.declaration.block.fields.get(field) {
            for (name, dependency) in self.map(&definition.module, map)? {
                if !rpm(name) || dependencies.contains_key(name) {
                    return Err(self.error(
                        &definition.module,
                        dependency,
                        "dependency",
                        "invalid or duplicate dependency alias",
                    ));
                }
                let key = self.instantiate(definition, dependency, params, depth + 1)?;
                dependencies.insert(name.clone(), key.clone());
                output.insert(name.clone(), key);
            }
        }
        Ok(output)
    }
    fn source(&self, definition: &Definition) -> Result<SourceIntent> {
        let source = self.field(definition, "source")?;
        if matches!(&source.kind, Kind::Construct(k, _) if k == "files") {
            return Ok(SourceIntent::Files(self.files(definition, source)?));
        }
        let block = self.construct(&definition.module, source, "archive", &["url", "pin"])?;
        let pin = self.reviewed_pin(&definition.module, source, block)?;
        Ok(SourceIntent::Archive(pin.clone()))
    }
    fn reviewed_pin(&self, module: &str, value: &Value, archive: &Block) -> Result<&SourcePin> {
        let pin_name = self.string(module, &archive.fields["pin"])?;
        let pin = self.lock.sources.get(&pin_name).ok_or_else(|| {
            self.error(
                module,
                value,
                "pin",
                "archive pin absent from reviewed lockfile",
            )
        })?;
        if pin.url != self.string(module, &archive.fields["url"])? {
            return Err(self.error(
                module,
                value,
                "pin",
                "archive URL differs from reviewed pin",
            ));
        }
        Ok(pin)
    }
    fn optional_files(
        &self,
        definition: &Definition,
        field: &str,
    ) -> Result<BTreeMap<String, Content>> {
        Ok(definition
            .declaration
            .block
            .fields
            .get(field)
            .map(|v| self.files(definition, v))
            .transpose()?
            .unwrap_or_default())
    }
    fn patches(&self, definition: &Definition) -> Result<Vec<PatchIntent>> {
        let mut patches = Vec::new();
        if let Some(values) = definition.declaration.block.fields.get("patches") {
            for value in self.list(&definition.module, values)? {
                let block =
                    self.construct(&definition.module, value, "patch", &["strip", "contents"])?;
                patches.push(PatchIntent {
                    strip: self.integer(&definition.module, &block.fields["strip"])?,
                    content: self.content(definition, &block.fields["contents"])?,
                });
            }
        }
        Ok(patches)
    }
    fn require_overlay_tools(
        &mut self,
        caller: &Definition,
        value: &Value,
        builder: &str,
        patched: bool,
    ) -> Result<()> {
        self.intent
            .builders
            .get_mut(builder)
            .ok_or_else(|| {
                fail(
                    &caller.module,
                    value.span,
                    "builder",
                    "missing builder requirement set",
                )
            })?
            .extend(["coreutils".into(), "findutils".into(), "python3".into()]);
        if patched {
            self.intent
                .builders
                .get_mut(builder)
                .ok_or_else(|| fail(&caller.module, value.span, "builder", "missing builder"))?
                .insert("patch".into());
        }
        Ok(())
    }
    fn environment(
        &self,
        definition: &Definition,
        dependencies: &BTreeMap<String, String>,
        own: &str,
    ) -> Result<BTreeMap<String, Binding>> {
        let mut env = BTreeMap::new();
        if let Some(values) = definition.declaration.block.fields.get("env") {
            for (name, v) in self.map(&definition.module, values)? {
                if matches!(name.as_str(), "src" | "out")
                    || name.is_empty()
                    || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
                {
                    return Err(self.error(
                        &definition.module,
                        v,
                        "env",
                        "invalid or reserved environment name",
                    ));
                }
                env.insert(
                    name.clone(),
                    self.binding(definition, v, dependencies, own, false)?,
                );
            }
        }
        Ok(env)
    }
    fn configs(
        &self,
        definition: &Definition,
        dependencies: &BTreeMap<String, String>,
        own: &str,
    ) -> Result<BTreeMap<String, TemplateIntent>> {
        let mut configs = BTreeMap::new();
        for (path, value) in &definition.declaration.block.configs {
            if !path.strip_prefix('/').is_some_and(relative) {
                return Err(self.error(
                    &definition.module,
                    value,
                    "path",
                    "unsafe absolute config path",
                ));
            }
            configs.insert(
                path.clone(),
                self.template(definition, value, dependencies, own)?,
            );
        }
        Ok(configs)
    }
    fn template(
        &self,
        definition: &Definition,
        value: &Value,
        dependencies: &BTreeMap<String, String>,
        own: &str,
    ) -> Result<TemplateIntent> {
        let block = self.construct(&definition.module, value, "template", &["body", "bindings"])?;
        let Kind::Tagged(_, body) = &block.fields["body"].kind else {
            return Err(self.error(
                &definition.module,
                value,
                "type",
                "template requires literal text",
            ));
        };
        let bindings = self
            .map(&definition.module, &block.fields["bindings"])?
            .iter()
            .map(|(name, v)| {
                Ok((
                    name.clone(),
                    self.binding(definition, v, dependencies, own, true)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let used = self.bound_placeholders(&definition.module, value, body, &bindings)?;
        if used != bindings.keys().cloned().collect() {
            return Err(self.error(
                &definition.module,
                value,
                "template",
                "extra named template binding",
            ));
        }
        Ok(TemplateIntent {
            body: body.clone(),
            bindings,
        })
    }
    fn bound_placeholders(
        &self,
        module: &str,
        value: &Value,
        body: &str,
        bindings: &BTreeMap<String, Binding>,
    ) -> Result<BTreeSet<String>> {
        let mut used = BTreeSet::new();
        let mut remaining = body;
        while let Some((_, tail)) = remaining.split_once("{{") {
            let (name, rest) = tail.split_once("}}").ok_or_else(|| {
                self.error(module, value, "template", "unterminated template binding")
            })?;
            if !bindings.contains_key(name) {
                return Err(self.error(
                    module,
                    value,
                    "template",
                    "missing named template binding",
                ));
            }
            used.insert(name.to_owned());
            remaining = rest;
        }
        Ok(used)
    }
    fn error(&self, module: &str, value: &Value, code: &str, message: &str) -> Diagnostic {
        fail(module, value.span, code, message)
    }
    fn field<'a>(&self, definition: &'a Definition, name: &str) -> Result<&'a Value> {
        definition
            .declaration
            .block
            .fields
            .get(name)
            .ok_or_else(|| {
                fail(
                    &definition.module,
                    definition.declaration.span,
                    "field",
                    "missing required field",
                )
            })
    }
    fn string(&self, module: &str, value: &Value) -> Result<String> {
        if let Kind::String(text) = &value.kind {
            Ok(text.clone())
        } else {
            Err(self.error(module, value, "type", "expected string"))
        }
    }
    fn integer(&self, module: &str, value: &Value) -> Result<u64> {
        if let Kind::Integer(n) = value.kind {
            Ok(n)
        } else {
            Err(self.error(module, value, "type", "expected integer"))
        }
    }
    fn list<'a>(&self, module: &str, value: &'a Value) -> Result<&'a [Value]> {
        if let Kind::List(values) = &value.kind {
            Ok(values)
        } else {
            Err(self.error(module, value, "type", "expected list"))
        }
    }
    fn map<'a>(&self, module: &str, value: &'a Value) -> Result<&'a BTreeMap<String, Value>> {
        if let Kind::Block(block) = &value.kind
            && block.exports.is_empty()
            && block.configs.is_empty()
        {
            return Ok(&block.fields);
        }
        Err(self.error(module, value, "type", "expected field map"))
    }
    fn lookup(&self, module: &str, value: &Value, parts: &[String]) -> Result<String> {
        if parts.len() != 1 {
            return Err(self.error(module, value, "reference", "unknown qualified reference"));
        }
        self.symbols
            .get(module)
            .and_then(|s| s.get(&parts[0]))
            .cloned()
            .ok_or_else(|| {
                self.error(
                    module,
                    value,
                    "reference",
                    "unknown or unimported declaration",
                )
            })
    }
    fn construct<'a>(
        &self,
        module: &str,
        value: &'a Value,
        name: &str,
        fields: &[&str],
    ) -> Result<&'a Block> {
        if let Kind::Construct(kind, block) = &value.kind
            && kind == name
            && block.exports.is_empty()
            && block.configs.is_empty()
            && block.fields.keys().all(|k| fields.contains(&k.as_str()))
        {
            return Ok(block);
        }
        Err(self.error(module, value, "type", "unsupported constructor or fields"))
    }
    fn typed(&self, definition: &Definition, value: &Value) -> Result<Type> {
        match &value.kind {
            Kind::String(_) => Ok(Type::String),
            Kind::Integer(_) => Ok(Type::Integer),
            Kind::Boolean(_) => Ok(Type::Boolean),
            Kind::Tagged(kind, _) => {
                if kind == "shell" {
                    Ok(Type::Script)
                } else {
                    Ok(Type::Content)
                }
            }
            Kind::List(values) => {
                for v in values {
                    self.typed(definition, v)?;
                }
                Ok(Type::List)
            }
            Kind::Block(block) => {
                if !block.configs.is_empty() || !block.exports.is_empty() {
                    return Err(self.error(
                        &definition.module,
                        value,
                        "type",
                        "declarations forbidden in value maps",
                    ));
                }
                for v in block.fields.values() {
                    self.typed(definition, v)?;
                }
                Ok(Type::Map)
            }
            Kind::Reference(parts) => self.typed_reference(definition, value, parts),
            Kind::Call(parts, args)
                if parts.len() == 1
                    && matches!(
                        parts[0].as_str(),
                        "fedora" | "file" | "script" | "executable" | "path"
                    ) =>
            {
                self.typed_builtin(definition, value, &parts[0], args)
            }
            Kind::Call(parts, args) => self.typed_invocation(definition, value, parts, args),
            Kind::Construct(name, block) => self.typed_construct(definition, value, name, block),
        }
    }
    fn typed_reference(
        &self,
        definition: &Definition,
        value: &Value,
        parts: &[String],
    ) -> Result<Type> {
        let module = &definition.module;
        if parts.len() == 1 && parts[0] == "source_revision" {
            return Ok(Type::String);
        }
        if parts.len() == 1
            && let Some(ty) = definition.declaration.parameters.get(&parts[0])
        {
            return Ok(if ty == "Builder" {
                Type::Builder
            } else {
                Type::Foundation
            });
        }
        if parts.len() == 2 && parts[0] == "deps" {
            let found = ["build_deps", "runtime_deps"].into_iter().any(|field| {
                definition.declaration.block.fields.get(field).is_some_and(
                    |v| matches!(&v.kind, Kind::Block(b) if b.fields.contains_key(&parts[1])),
                )
            });
            if found {
                return Ok(Type::Package);
            }
            return Err(self.error(module, value, "reference", "unknown dependency"));
        }
        let key = self.lookup(module, value, parts)?;
        match self.definitions[&key].declaration.kind.as_str() {
            "foundation" => Ok(Type::Foundation),
            "builder" => Ok(Type::Builder),
            "set" => Ok(Type::Set),
            _ => Err(self.error(
                module,
                value,
                "type",
                "package templates require explicit invocation",
            )),
        }
    }
    fn typed_builtin(
        &self,
        definition: &Definition,
        value: &Value,
        name: &str,
        args: &[(Option<String>, Value)],
    ) -> Result<Type> {
        let module = &definition.module;
        if args.iter().any(|(key, _)| key.is_some()) {
            return Err(self.error(
                module,
                value,
                "arguments",
                "built-in arguments are positional",
            ));
        }
        if name == "path" && args.len() == 2 {
            return self.typed_path(definition, value, args);
        }
        let types = args
            .iter()
            .map(|(_, v)| self.typed(definition, v))
            .collect::<Result<Vec<_>>>()?;
        match (name, types.as_slice()) {
            ("fedora", [Type::String]) => Ok(Type::Fedora),
            ("file", [Type::String]) => Ok(Type::Content),
            ("script", [Type::Content]) => Ok(Type::Script),
            ("executable", [Type::Content]) => Ok(Type::Content),
            ("path", [_, Type::String]) => Ok(Type::Path),
            _ => Err(self.error(module, value, "arguments", "wrong built-in arguments")),
        }
    }
    fn typed_path(
        &self,
        definition: &Definition,
        value: &Value,
        args: &[(Option<String>, Value)],
    ) -> Result<Type> {
        let module = &definition.module;
        self.string(module, &args[1].1)?;
        let Kind::Reference(reference) = &args[0].1.kind else {
            return Err(self.error(module, value, "type", "path requires a reference"));
        };
        if reference.as_slice() != ["self"] {
            if reference.len() == 2 && reference[0] == "deps" {
                self.typed(definition, &args[0].1)?;
            } else {
                self.lookup(module, value, reference)?;
            }
        }
        Ok(Type::Path)
    }
    fn typed_invocation(
        &self,
        definition: &Definition,
        value: &Value,
        parts: &[String],
        args: &[(Option<String>, Value)],
    ) -> Result<Type> {
        let module = &definition.module;
        let key = self.lookup(module, value, parts)?;
        let callee = &self.definitions[&key].declaration;
        if !matches!(callee.kind.as_str(), "package" | "library")
            || args.len() != callee.parameters.len()
        {
            return Err(self.error(
                module,
                value,
                "arguments",
                "not a package template or wrong argument count",
            ));
        }
        for (name, arg) in args {
            let ty = name
                .as_ref()
                .and_then(|name| callee.parameters.get(name))
                .ok_or_else(|| {
                    self.error(
                        module,
                        value,
                        "arguments",
                        "template requires exact named arguments",
                    )
                })?;
            let expected = if ty == "Builder" {
                Type::Builder
            } else {
                Type::Foundation
            };
            if self.typed(definition, arg)? != expected {
                return Err(self.error(module, arg, "type", "wrong image role type"));
            }
        }
        Ok(Type::Package)
    }
    fn typed_construct(
        &self,
        definition: &Definition,
        value: &Value,
        name: &str,
        block: &Block,
    ) -> Result<Type> {
        match name {
            "files" => self.typed_files(definition, value, block),
            "archive" => self.typed_archive(definition, value, block),
            "patch" => self.typed_patch(definition, value, block),
            "replacement" => self.typed_replacement(definition, value, block),
            "template" => self.typed_template(definition, value, block),
            _ => Err(self.error(
                &definition.module,
                value,
                "constructor",
                "unsupported constructor",
            )),
        }
    }
    fn typed_files(&self, definition: &Definition, value: &Value, block: &Block) -> Result<Type> {
        let module = &definition.module;
        self.construct(
            module,
            value,
            "files",
            &block.fields.keys().map(String::as_str).collect::<Vec<_>>(),
        )?;
        let mut names = BTreeSet::new();
        if block.fields.len() > super::MAX_RESOURCES {
            return Err(self.error(module, value, "limit", "resource count exceeded"));
        }
        for (path, content) in &block.fields {
            if !relative(path)
                || !super::public_input(path, &[])
                || !names.insert(path.to_ascii_lowercase())
            {
                return Err(self.error(
                    module,
                    content,
                    "path",
                    "unsafe or case-colliding resource path",
                ));
            }
            if self.typed(definition, content)? != Type::Content {
                return Err(self.error(
                    module,
                    content,
                    "type",
                    "file requires text or admitted content",
                ));
            }
        }
        Ok(Type::Files)
    }
    fn typed_archive(&self, definition: &Definition, value: &Value, block: &Block) -> Result<Type> {
        let module = &definition.module;
        self.construct(module, value, "archive", &["url", "pin"])?;
        for field in ["url", "pin"] {
            let v = block
                .fields
                .get(field)
                .ok_or_else(|| self.error(module, value, "field", "missing archive field"))?;
            self.string(module, v)?;
        }
        self.reviewed_pin(module, value, block)?;
        Ok(Type::Source)
    }
    fn typed_patch(&self, definition: &Definition, value: &Value, block: &Block) -> Result<Type> {
        let module = &definition.module;
        self.construct(module, value, "patch", &["strip", "contents"])?;
        let strip = block
            .fields
            .get("strip")
            .ok_or_else(|| self.error(module, value, "field", "missing patch strip"))?;
        if self.integer(module, strip)? > 64 {
            return Err(self.error(module, value, "patch", "patch strip exceeds path depth"));
        }
        let content = block
            .fields
            .get("contents")
            .ok_or_else(|| self.error(module, value, "field", "missing patch content"))?;
        if self.typed(definition, content)? != Type::Content {
            return Err(self.error(module, content, "type", "patch requires content"));
        }
        Ok(Type::Patch)
    }
    fn typed_replacement(
        &self,
        definition: &Definition,
        value: &Value,
        block: &Block,
    ) -> Result<Type> {
        let module = &definition.module;
        self.construct(module, value, "replacement", &["origin", "name", "action"])?;
        for field in ["origin", "name", "action"] {
            self.string(
                module,
                block.fields.get(field).ok_or_else(|| {
                    self.error(module, value, "field", "missing replacement field")
                })?,
            )?;
        }
        Ok(Type::Replacement)
    }
    fn typed_template(
        &self,
        definition: &Definition,
        value: &Value,
        block: &Block,
    ) -> Result<Type> {
        let module = &definition.module;
        self.construct(module, value, "template", &["body", "bindings"])?;
        if !matches!(block.fields.get("body").map(|v| &v.kind), Some(Kind::Tagged(k, _)) if k == "text")
        {
            return Err(self.error(module, value, "type", "template body requires text"));
        }
        let bindings = block
            .fields
            .get("bindings")
            .ok_or_else(|| self.error(module, value, "field", "missing template bindings"))?;
        for v in self.map(module, bindings)?.values() {
            if !matches!(self.typed(definition, v)?, Type::String | Type::Path) {
                return Err(self.error(
                    module,
                    v,
                    "type",
                    "template binding requires string or path",
                ));
            }
        }
        Ok(Type::Template)
    }
    fn validate(&self, definition: &Definition) -> Result<()> {
        let d = &definition.declaration;
        let module = &definition.module;
        let (required, optional): (&[&str], &[&str]) = match d.kind.as_str() {
            "foundation" => (&["fedora", "packages"], &["remove"]),
            "builder" => (&["base", "packages"], &[]),
            "set" => (&[], &["packages", "fedora", "remove", "use", "replace"]),
            "target" => (
                &["foundation"],
                &["packages", "fedora", "remove", "use", "replace"],
            ),
            _ => (
                &["version", "summary", "license", "source", "build"],
                &[
                    "builder",
                    "runtime",
                    "build_requires",
                    "runtime_requires",
                    "build_deps",
                    "runtime_deps",
                    "env",
                    "files",
                    "replace_files",
                    "patches",
                    "timeout",
                ],
            ),
        };
        for field in required {
            self.field(definition, field)?;
        }
        for (name, value) in &d.block.fields {
            if !required.contains(&name.as_str()) && !optional.contains(&name.as_str()) {
                return Err(self.error(module, value, "field", "unknown declaration field"));
            }
            self.validate_field(definition, name, value)?;
        }
        if matches!(d.kind.as_str(), "package" | "library") {
            self.validate_package(definition)
        } else if !d.block.exports.is_empty() || !d.block.configs.is_empty() {
            Err(fail(
                module,
                d.span,
                "type",
                "exports/config require a package declaration",
            ))
        } else {
            Ok(())
        }
    }
    fn validate_field(&self, definition: &Definition, name: &str, value: &Value) -> Result<()> {
        let d = &definition.declaration;
        let module = &definition.module;
        let ty = self.typed(definition, value)?;
        let expected = match name {
            "fedora" if d.kind == "foundation" => Type::Integer,
            "fedora" | "packages" | "remove" | "use" | "replace" | "build_requires"
            | "runtime_requires" | "patches" => Type::List,
            "foundation" | "base" | "runtime" => Type::Foundation,
            "builder" => Type::Builder,
            "source" if ty == Type::Files => Type::Files,
            "source" => Type::Source,
            "build" => Type::Script,
            "files" | "replace_files" => Type::Files,
            "env" | "build_deps" | "runtime_deps" => Type::Map,
            "timeout" => Type::Integer,
            _ => Type::String,
        };
        if ty != expected {
            return Err(self.error(module, value, "type", "wrong declaration field type"));
        }
        if matches!(name, "builder" | "runtime") && matches!(d.kind.as_str(), "package" | "library")
        {
            self.validate_role(definition, value)?;
        }
        if name == "timeout" && !(1..=3600).contains(&self.integer(module, value)?) {
            return Err(self.error(module, value, "timeout", "build timeout must be1..3600"));
        }
        if name == "fedora" && d.kind == "foundation" && self.integer(module, value)? != 44 {
            return Err(self.error(module, value, "foundation", "only Fedora44 is supported"));
        }
        if matches!(name, "build_deps" | "runtime_deps" | "env") {
            for (alias, item) in self.map(module, value)? {
                self.validate_input(definition, name, alias, item)?;
            }
        }
        if ty == Type::List {
            self.validate_items(definition, name, value)?;
        }
        Ok(())
    }
    fn validate_role(&self, definition: &Definition, value: &Value) -> Result<()> {
        let module = &definition.module;
        let Kind::Reference(parts) = &value.kind else {
            return Err(self.error(
                module,
                value,
                "role",
                "role field requires template parameter",
            ));
        };
        if parts.len() != 1 || !definition.declaration.parameters.contains_key(&parts[0]) {
            return Err(self.error(
                module,
                value,
                "role",
                "role field requires template parameter",
            ));
        }
        Ok(())
    }
    fn validate_input(
        &self,
        definition: &Definition,
        field: &str,
        alias: &str,
        item: &Value,
    ) -> Result<()> {
        let module = &definition.module;
        if alias.is_empty()
            || alias.len() > 128
            || alias.as_bytes()[0].is_ascii_digit()
            || !alias
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || field != "env" && c == b'-')
            || [
                "src",
                "out",
                "HOME",
                "PATH",
                "TMPDIR",
                "source",
                "resources",
            ]
            .contains(&alias)
        {
            return Err(self.error(
                module,
                item,
                "name",
                "invalid/reserved input or environment name",
            ));
        }
        let ty = self.typed(definition, item)?;
        if field == "env" {
            self.validate_environment(module, item, &ty)
        } else if ty != Type::Package {
            Err(self.error(
                module,
                item,
                "type",
                "dependency requires package invocation",
            ))
        } else {
            Ok(())
        }
    }
    fn validate_environment(&self, module: &str, item: &Value, ty: &Type) -> Result<()> {
        if matches!(&item.kind, Kind::String(text) if text.len() > 4096 || text.contains('\0')) {
            return Err(self.error(module, item, "env", "environment literal exceeds bound"));
        }
        if !matches!(ty, Type::String | Type::Path) {
            return Err(self.error(
                module,
                item,
                "type",
                "environment requires literal string or dependency path",
            ));
        }
        if matches!(&item.kind, Kind::Reference(_))
            || matches!(&item.kind, Kind::Call(_, args) if matches!(&args[0].1.kind, Kind::Reference(parts) if parts.as_slice() == ["self"]))
        {
            return Err(self.error(
                module,
                item,
                "phase",
                "future output/revision unavailable during build",
            ));
        }
        Ok(())
    }
    fn validate_items(&self, definition: &Definition, field: &str, value: &Value) -> Result<()> {
        let module = &definition.module;
        let item_type = match field {
            "packages" if matches!(definition.declaration.kind.as_str(), "set" | "target") => {
                Type::Package
            }
            "use" => Type::Set,
            "replace" => Type::Replacement,
            "patches" => Type::Patch,
            "build_requires" | "runtime_requires" => Type::Fedora,
            _ => Type::String,
        };
        for item in self.list(module, value)? {
            if self.typed(definition, item)? != item_type {
                return Err(self.error(module, item, "type", "wrong list item type"));
            }
            if item_type == Type::String && !rpm(&self.string(module, item)?) {
                return Err(self.error(module, item, "rpm", "expected RPM package name"));
            }
        }
        Ok(())
    }
    fn validate_package(&self, definition: &Definition) -> Result<()> {
        let d = &definition.declaration;
        let module = &definition.module;
        if d.parameters.len() != 2
            || d.parameters.values().filter(|t| *t == "Builder").count() != 1
            || d.parameters.values().filter(|t| *t == "Foundation").count() != 1
        {
            return Err(fail(
                module,
                d.span,
                "parameters",
                "one Builder and one Foundation parameter required",
            ));
        }
        for field in ["version", "summary", "license"] {
            let v = self.field(definition, field)?;
            let text = self.string(module, v)?;
            if text.is_empty() || text.len() > 4096 || text.chars().any(char::is_control) {
                return Err(self.error(module, v, "metadata", "invalid bounded metadata"));
            }
        }
        if d.block.exports.is_empty()
            || (d.kind == "package" && !d.block.exports.iter().any(|e| e.kind == "command"))
        {
            return Err(fail(
                module,
                d.span,
                "export",
                "package command or library output export required",
            ));
        }
        for export in &d.block.exports {
            self.export(module, export, d.kind == "library")?;
        }
        for value in d.block.configs.values() {
            if self.typed(definition, value)? != Type::Template {
                return Err(self.error(module, value, "type", "config requires template"));
            }
        }
        Ok(())
    }
    fn export(&self, module: &str, export: &Export, library: bool) -> Result<()> {
        if !matches!(export.kind.as_str(), "command" | "library" | "files")
            || (library && export.kind == "command")
            || !relative(&export.path)
            || !rpm(&export.name)
        {
            return Err(fail(
                module,
                export.span,
                "export",
                "unsupported export kind, name or path",
            ));
        }
        Ok(())
    }
}

pub fn compile(
    modules: &BTreeMap<String, String>,
    entry: &str,
    target: &str,
    lock: Lockfile,
    policy: &TargetPolicy,
) -> Result<Intent> {
    if modules.is_empty()
        || modules.len() > MAX_MODULES
        || modules.values().map(String::len).sum::<usize>() > MAX_TOTAL
    {
        return Err(fail(
            entry,
            root_span(),
            "limit",
            "module/aggregate input limit exceeded",
        ));
    }
    check_policy(entry, target, &lock, policy)?;
    check_pins(entry, &lock)?;
    let mut checker = Checker {
        modules: BTreeMap::new(),
        definitions: BTreeMap::new(),
        symbols: BTreeMap::new(),
        lock,
        intent: Intent {
            schema_version: 1,
            namespace: policy.namespace.clone(),
            target: target.into(),
            foundation: String::new(),
            packages: BTreeSet::new(),
            remove: BTreeSet::new(),
            builders: BTreeMap::new(),
            recipes: BTreeMap::new(),
            selected: BTreeSet::new(),
        },
        active: BTreeSet::new(),
        selected_sets: BTreeSet::new(),
        requests: Vec::new(),
    };
    checker.admit(modules, entry, policy)?;
    checker.imports(entry, &mut BTreeSet::new(), &mut BTreeSet::new(), 0)?;
    checker.link_imports()?;
    for definition in checker.definitions.values() {
        checker.check_parameter_shadowing(definition)?;
        checker.validate(definition)?;
    }
    let mut visited = BTreeSet::new();
    for key in checker.definitions.keys() {
        checker.cycles(key, &mut BTreeSet::new(), &mut visited, 0)?;
    }
    checker.select(entry, target, policy)?;
    Ok(checker.intent)
}

fn check_policy(entry: &str, target: &str, lock: &Lockfile, policy: &TargetPolicy) -> Result<()> {
    if !relative(entry)
        || policy.schema_version != 1
        || lock.schema_version != 1
        || policy.namespace.is_empty()
        || policy.namespace.len() > 128
        || !policy
            .namespace
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
        || !policy.targets.contains(target)
        || policy.targets.len() > 256
        || policy.required_packages.len() > 256
        || policy.repositories != BTreeSet::from(["fedora".into(), "updates".into()])
    {
        return Err(fail(
            entry,
            root_span(),
            "policy",
            "invalid policy or denied target",
        ));
    }
    Ok(())
}

fn check_pins(entry: &str, lock: &Lockfile) -> Result<()> {
    let mut pinned_objects = BTreeMap::<&str, &SourcePin>::new();
    for pin in lock.sources.values() {
        if lock.sources.len() > 256
            || !pin.url.starts_with("https://")
            || pin.url.len() > 4096
            || pin.url.chars().any(char::is_control)
            || !hex(&pin.sha256)
            || !pin.object.strip_prefix("src-").is_some_and(hex)
        {
            return Err(fail(entry, root_span(), "pin", "invalid exact source pin"));
        }
        if let Some(previous) = pinned_objects.insert(&pin.object, pin)
            && (previous.url != pin.url || previous.sha256 != pin.sha256)
        {
            return Err(fail(
                entry,
                root_span(),
                "pin",
                "conflicting archive provenance for one source identity",
            ));
        }
    }
    Ok(())
}

impl Checker {
    fn admit(
        &mut self,
        modules: &BTreeMap<String, String>,
        entry: &str,
        policy: &TargetPolicy,
    ) -> Result<()> {
        fn resources(value: &Value) -> usize {
            match &value.kind {
                Kind::Tagged(_, _) => 1,
                Kind::Call(name, _) if name.as_slice() == ["file"] => 1,
                Kind::Call(_, args) => args.iter().map(|(_, v)| resources(v)).sum(),
                Kind::List(values) => values.iter().map(resources).sum(),
                Kind::Construct(_, block) | Kind::Block(block) => block
                    .fields
                    .values()
                    .chain(block.configs.values())
                    .map(resources)
                    .sum(),
                _ => 0,
            }
        }
        let mut resource_count = 0;
        for (path, contents) in modules {
            if !relative(path) {
                return Err(fail(entry, root_span(), "path", "unsafe module path"));
            }
            let module = parse(path, contents)?;
            resource_count += module
                .declarations
                .values()
                .flat_map(|d| d.block.fields.values().chain(d.block.configs.values()))
                .map(resources)
                .sum::<usize>();
            if resource_count > super::MAX_RESOURCES {
                return Err(fail(
                    path,
                    root_span(),
                    "limit",
                    "declared resource count exceeded",
                ));
            }
            if (path == entry && module.namespace.as_deref() != Some(policy.namespace.as_str()))
                || (path != entry && module.namespace.is_some())
            {
                return Err(fail(
                    path,
                    root_span(),
                    "namespace",
                    "entry requires policy namespace; imports inherit it",
                ));
            }
            self.declare(path, module);
        }
        Ok(())
    }
    fn declare(&mut self, path: &str, module: Module) {
        let mut symbols = BTreeMap::new();
        for (name, declaration) in &module.declarations {
            let key = format!("{path}#{name}");
            symbols.insert(name.clone(), key.clone());
            self.definitions.insert(
                key,
                Definition {
                    module: path.into(),
                    declaration: declaration.clone(),
                },
            );
        }
        self.symbols.insert(path.into(), symbols);
        self.modules.insert(path.into(), module);
    }
    fn link_imports(&mut self) -> Result<()> {
        for (path, module) in &self.modules {
            let mut imported = BTreeMap::new();
            for import in &module.imports {
                let resolved = import_path(path, &import.path).ok_or_else(|| {
                    fail(
                        path,
                        import.span,
                        "import",
                        "imports require safe explicit local paths",
                    )
                })?;
                let other = self
                    .modules
                    .get(&resolved)
                    .ok_or_else(|| fail(path, import.span, "import", "missing admitted module"))?;
                for name in &import.names {
                    if !other.declarations.contains_key(name)
                        || self.symbols[path].contains_key(name)
                        || imported
                            .insert(name.clone(), format!("{resolved}#{name}"))
                            .is_some()
                    {
                        return Err(fail(
                            path,
                            import.span,
                            "import",
                            "missing import or shadowed declaration",
                        ));
                    }
                }
            }
            self.symbols
                .get_mut(path)
                .ok_or_else(|| fail(path, root_span(), "import", "missing symbol scope"))?
                .extend(imported);
        }
        Ok(())
    }
    fn check_parameter_shadowing(&self, definition: &Definition) -> Result<()> {
        if definition.declaration.parameters.keys().any(|name| {
            self.symbols[&definition.module].contains_key(name)
                || [
                    "self",
                    "deps",
                    "src",
                    "out",
                    "source_revision",
                    "file",
                    "path",
                    "text",
                    "shell",
                    "script",
                    "executable",
                    "fedora",
                ]
                .contains(&name.as_str())
        }) {
            return Err(fail(
                &definition.module,
                definition.declaration.span,
                "name",
                "parameter shadows declaration or reserved binding",
            ));
        }
        Ok(())
    }
}
