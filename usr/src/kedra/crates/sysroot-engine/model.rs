use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LOGICAL_PREFIX: &str = "/usr/lib/sysroot/store";
pub const MAX_JSON: u64 = 8 * 1024 * 1024;
/// The engine is native: stores, images, builds and derivations all use the
/// architecture this binary was compiled for. Cross-architecture work is refused.
#[cfg(target_arch = "aarch64")]
pub const PLATFORM: &str = "aarch64-linux";
#[cfg(target_arch = "x86_64")]
pub const PLATFORM: &str = "x86_64-linux";
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
compile_error!("the engine supports native aarch64 and x86_64 only");
/// RPM architecture of [`PLATFORM`] (`uname -m`, Kedra target architecture).
#[cfg(target_arch = "aarch64")]
pub const RPM_ARCHITECTURE: &str = "aarch64";
#[cfg(target_arch = "x86_64")]
pub const RPM_ARCHITECTURE: &str = "x86_64";
/// OCI/Docker architecture of [`PLATFORM`] images.
#[cfg(target_arch = "aarch64")]
pub const OCI_ARCHITECTURE: &str = "arm64";
#[cfg(target_arch = "x86_64")]
pub const OCI_ARCHITECTURE: &str = "amd64";
/// `--platform` value of [`PLATFORM`] builds and containers.
#[cfg(target_arch = "aarch64")]
pub const OCI_PLATFORM: &str = "linux/arm64";
#[cfg(target_arch = "x86_64")]
pub const OCI_PLATFORM: &str = "linux/amd64";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildGraph {
    pub schema: u32,
    #[serde(deserialize_with = "unique_map")]
    pub nodes: BTreeMap<String, BuildNode>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildNode {
    pub builder_image: String,
    pub runtime_image: String,
    #[serde(deserialize_with = "unique_map")]
    pub inputs: BTreeMap<String, Input>,
    pub argv: Vec<Argument>,
    #[serde(default, deserialize_with = "unique_map")]
    pub env: BTreeMap<String, Argument>,
    #[serde(default)]
    pub runtime_inputs: Vec<String>,
    #[serde(default = "default_build_timeout")]
    pub timeout_seconds: u64,
}
fn default_build_timeout() -> u64 {
    120
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Input {
    Object(String),
    Node(String),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Argument(pub Vec<Segment>);
impl Argument {
    pub fn literal(value: impl Into<String>) -> Self {
        Self(vec![Segment::Literal {
            value: value.into(),
        }])
    }
    pub fn input(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self(vec![Segment::Input {
            name: name.into(),
            path: path.into(),
        }])
    }
    pub fn output(path: impl Into<String>) -> Self {
        Self(vec![Segment::Output { path: path.into() }])
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Segment {
    Literal { value: String },
    Input { name: String, path: String },
    Output { path: String },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedBuildSpec {
    pub schema: u32,
    pub platform: String,
    pub prefix: String,
    pub policy: String,
    pub builder_image: String,
    pub runtime_image: String,
    #[serde(deserialize_with = "unique_map")]
    pub inputs: BTreeMap<String, String>,
    pub argv: Vec<Argument>,
    #[serde(deserialize_with = "unique_map")]
    pub env: BTreeMap<String, Argument>,
    pub runtime_inputs: Vec<String>,
    pub timeout_seconds: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub outputs: BTreeMap<String, String>,
    pub order: Vec<String>,
    pub specs: BTreeMap<String, ResolvedBuildSpec>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectReceipt {
    pub schema: u32,
    pub object: String,
    pub tree_sha256: String,
    pub references: Vec<String>,
    pub runtime_image: Option<String>,
    pub derivation: Option<ResolvedBuildSpec>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImageReceipt {
    pub schema: u32,
    pub image: String,
    pub platform: String,
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct BuildResult {
    pub outputs: BTreeMap<String, String>,
    pub built: Vec<String>,
    pub reused: Vec<String>,
    pub reproduced: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Closure {
    pub objects: Vec<String>,
    pub images: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ExportResult {
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Clone, Debug, Serialize)]
pub struct ImportResult {
    pub roots: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct GcResult {
    pub objects: Vec<String>,
    pub images: Vec<String>,
    pub deleted: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generation {
    pub generation: u64,
    pub object: String,
    pub program: String,
    pub args: Vec<String>,
    pub runtime_image: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct Profile {
    pub name: String,
    pub selected: u64,
    pub object: String,
    pub generations: Vec<Generation>,
}
#[derive(Clone, Debug, Serialize)]
pub struct RunResult {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Recovery {
    pub recovered: Vec<String>,
}
pub fn unique_map<'de, D, T>(deserializer: D) -> std::result::Result<BTreeMap<String, T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Unique<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for Unique<T> {
        type Value = BTreeMap<String, T>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a map with unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> std::result::Result<Self::Value, A::Error> {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, T>()? {
                if map.len() >= 100000 || map.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom(
                        "duplicate map key or map limit exceeded",
                    ));
                }
            }
            Ok(map)
        }
    }
    deserializer.deserialize_map(Unique(std::marker::PhantomData))
}
