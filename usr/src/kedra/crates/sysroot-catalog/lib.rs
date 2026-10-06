//! Typed package collections over the independent engine's fixed store namespace.
//!
//! Resolution is pure: authority is checked before callers open or create a store.
//! An allowlisted source identity authorizes bytes, not an upstream publisher.

mod recipes;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use sysroot_engine::{BuildGraph, Input, Plan};

pub use recipes::{JQ_SOURCE, SQLITE_SOURCE, builtin};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub namespace: String,
    #[serde(deserialize_with = "unique_packages")]
    pub packages: BTreeMap<String, Package>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub version: String,
    pub summary: String,
    pub license: String,
    pub sources: Vec<Source>,
    pub recipe: Recipe,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub object: String,
    pub origin: SourceOrigin,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceOrigin {
    Archive { url: String, sha256: String },
    Local { description: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    pub graph: BuildGraph,
    pub root: String,
    pub program: String,
}

/// Exact caller-owned allowlists. Empty sets deny everything in that category.
/// The namespace scopes names; it never changes the engine's logical prefix.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub namespace: String,
    pub packages: BTreeSet<String>,
    pub builder_images: BTreeSet<String>,
    pub runtime_images: BTreeSet<String>,
    pub source_objects: BTreeSet<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ResolvedPackage {
    pub namespace: String,
    pub package: String,
    pub version: String,
    pub recipe: Recipe,
    pub plan: Plan,
}

#[derive(Debug)]
pub enum Error {
    Invalid(String),
    Denied(String),
    Engine(sysroot_engine::Error),
    Json(serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(f, "invalid catalog: {message}"),
            Self::Denied(message) => write!(f, "catalog policy denied: {message}"),
            Self::Engine(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "catalog JSON: {error}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<sysroot_engine::Error> for Error {
    fn from(error: sysroot_engine::Error) -> Self {
        Self::Engine(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl Catalog {
    pub fn resolve(&self, package: &str, policy: &Policy) -> Result<ResolvedPackage> {
        name(&self.namespace)?;
        name(package)?;
        policy.validate()?;
        if self.namespace != policy.namespace || !policy.packages.contains(package) {
            return Err(Error::Denied(format!(
                "package {}/{package}",
                self.namespace
            )));
        }
        if self.packages.is_empty() || self.packages.len() > 256 {
            return Err(Error::Invalid("expected 1..=256 packages".into()));
        }
        if serde_json::to_vec(self)?.len() as u64 > sysroot_engine::MAX_JSON {
            return Err(Error::Invalid("catalog exceeds 8 MiB".into()));
        }
        for key in self.packages.keys() {
            name(key)?;
        }
        let selected = self
            .packages
            .get(package)
            .ok_or_else(|| Error::Invalid(format!("unknown package {package}")))?;
        text(&selected.version)?;
        text(&selected.summary)?;
        text(&selected.license)?;
        program(&selected.recipe.program)?;
        if selected.sources.len() > 256 {
            return Err(Error::Invalid("too many package sources".into()));
        }
        let mut declared = BTreeSet::new();
        for source in &selected.sources {
            identity(&source.object, "src-")?;
            if !declared.insert(source.object.clone()) {
                return Err(Error::Invalid("duplicate package source".into()));
            }
            match &source.origin {
                SourceOrigin::Archive { url, sha256 } => {
                    text(url)?;
                    if !url.starts_with("https://") || url.len() <= 8 {
                        return Err(Error::Invalid("source URL must use HTTPS".into()));
                    }
                    identity(sha256, "")?;
                }
                SourceOrigin::Local { description } => text(description)?,
            }
            if !policy.source_objects.contains(&source.object) {
                return Err(Error::Denied(format!("source {}", source.object)));
            }
        }
        let mut used = BTreeSet::new();
        for node in selected.recipe.graph.nodes.values() {
            if !policy.builder_images.contains(&node.builder_image) {
                return Err(Error::Denied(format!(
                    "builder image {}",
                    node.builder_image
                )));
            }
            if !policy.runtime_images.contains(&node.runtime_image) {
                return Err(Error::Denied(format!(
                    "runtime image {}",
                    node.runtime_image
                )));
            }
            for input in node.inputs.values() {
                if let Input::Object(object) = input {
                    identity(object, "src-")?;
                    if !declared.contains(object) || !policy.source_objects.contains(object) {
                        return Err(Error::Denied(format!("undeclared source {object}")));
                    }
                    used.insert(object.clone());
                }
            }
        }
        if used != declared {
            return Err(Error::Invalid(
                "source declarations differ from recipe inputs".into(),
            ));
        }
        let plan = sysroot_engine::plan(&selected.recipe.graph, &selected.recipe.root)?;
        Ok(ResolvedPackage {
            namespace: self.namespace.clone(),
            package: package.into(),
            version: selected.version.clone(),
            recipe: selected.recipe.clone(),
            plan,
        })
    }
}

impl Policy {
    fn validate(&self) -> Result<()> {
        name(&self.namespace)?;
        if [
            self.packages.len(),
            self.builder_images.len(),
            self.runtime_images.len(),
            self.source_objects.len(),
        ]
        .into_iter()
        .any(|count| count > 256)
        {
            return Err(Error::Invalid(
                "policy allowlist exceeds 256 entries".into(),
            ));
        }
        for package in &self.packages {
            name(package)?;
        }
        for image in self.builder_images.iter().chain(&self.runtime_images) {
            identity(image, "sha256:")?;
        }
        for source in &self.source_objects {
            identity(source, "src-")?;
        }
        Ok(())
    }
}

fn identity(value: &str, prefix: &str) -> Result<()> {
    if value.strip_prefix(prefix).is_none_or(|digest| {
        digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }) {
        return Err(Error::Invalid(format!(
            "expected exact {prefix} identity: {value}"
        )));
    }
    Ok(())
}

fn name(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(Error::Invalid(format!("invalid name {value:?}")));
    }
    Ok(())
}

fn text(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "empty, oversized or control-bearing metadata".into(),
        ));
    }
    Ok(())
}

fn program(value: &str) -> Result<()> {
    text(value)?;
    if value.starts_with('/') || value.split('/').any(|part| matches!(part, "" | "." | "..")) {
        return Err(Error::Invalid(
            "program must be an output-relative path".into(),
        ));
    }
    Ok(())
}

fn unique_packages<'de, D>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, Package>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Packages;
    impl<'de> serde::de::Visitor<'de> for Packages {
        type Value = BTreeMap<String, Package>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("up to 256 uniquely named packages")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = access.next_entry()? {
                if values.len() >= 256 || values.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate package or catalog limit exceeded",
                    ));
                }
            }
            Ok(values)
        }
    }
    deserializer.deserialize_map(Packages)
}
