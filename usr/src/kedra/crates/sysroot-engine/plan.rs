use crate::{
    Argument, BuildGraph, Error, Input, LOGICAL_PREFIX, MAX_JSON, PLATFORM, Plan,
    ResolvedBuildSpec, Result, Segment,
};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    path::Path,
};

pub(crate) fn hash(bytes: &[u8]) -> String {
    encode_hex(&Sha256::digest(bytes))
}
pub(crate) fn image_id(id: &str) -> Result<()> {
    if id.strip_prefix("sha256:").is_none_or(|s| !hex(s)) {
        return Err(Error::Invalid(format!(
            "expected immutable sha256 image identity: {id}"
        )));
    }
    Ok(())
}
pub(crate) fn hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn object_id(id: &str) -> Result<()> {
    if !(id
        .strip_prefix("src-")
        .or_else(|| id.strip_prefix("out-"))
        .is_some_and(hex))
    {
        return Err(Error::Invalid(format!("invalid object identity {id}")));
    }
    Ok(())
}
pub(crate) fn name(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(Error::Invalid(format!("invalid name {s:?}")));
    }
    Ok(())
}
pub(crate) fn relative(s: &str, empty: bool) -> Result<()> {
    if (s.is_empty() && !empty)
        || s.len() > 4096
        || s.starts_with('/')
        || s.bytes().any(|b| b < 32 || b == 127)
        || (!s.is_empty() && s.split('/').any(|p| p.is_empty() || p == "." || p == ".."))
    {
        return Err(Error::Invalid(format!("unsafe relative path {s:?}")));
    }
    Ok(())
}
pub fn read_graph(path: &Path) -> Result<BuildGraph> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_JSON + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_JSON {
        return Err(Error::Invalid("graph exceeds 8 MiB".into()));
    }
    Ok(serde_json::from_slice(&bytes)?)
}
pub fn plan(graph: &BuildGraph, root: &str) -> Result<Plan> {
    if graph.schema != 1 || graph.nodes.is_empty() || graph.nodes.len() > 256 {
        return Err(Error::Invalid(
            "graph schema/count unsupported (1..=256 nodes)".into(),
        ));
    }
    if serde_json::to_vec(graph)?.len() as u64 > MAX_JSON {
        return Err(Error::Invalid("graph exceeds 8 MiB".into()));
    }
    for (key, node) in &graph.nodes {
        name(key)?;
        image_id(&node.builder_image)?;
        image_id(&node.runtime_image)?;
        if node.inputs.len() > 256
            || node.argv.is_empty()
            || node.argv.len() > 256
            || node.env.len() > 256
            || node.runtime_inputs.len() > 256
        {
            return Err(Error::Invalid("input/argv/env limits exceeded".into()));
        }
        timeout(node.timeout_seconds)?;
        for (alias, input) in &node.inputs {
            name(alias)?;
            match input {
                Input::Object(id) => object_id(id)?,
                Input::Node(n) => {
                    if !graph.nodes.contains_key(n) {
                        return Err(Error::Invalid(format!("missing dependency {n}")));
                    }
                }
            }
        }
        for alias in &node.runtime_inputs {
            if !node.inputs.contains_key(alias) {
                return Err(Error::Invalid(format!(
                    "runtime input {alias} is undeclared"
                )));
            }
        }
        for (key, arg) in &node.env {
            if key.is_empty()
                || key.len() > 128
                || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
                || key.as_bytes()[0].is_ascii_digit()
                || ["HOME", "TMPDIR", "PATH"].contains(&key.as_str())
            {
                return Err(Error::Invalid(format!(
                    "invalid/reserved environment name {key}"
                )));
            }
            validate_argument(arg, &node.inputs)?;
        }
        for arg in &node.argv {
            validate_argument(arg, &node.inputs)?;
        }
    }
    let mut result = Plan {
        outputs: BTreeMap::new(),
        order: Vec::new(),
        specs: BTreeMap::new(),
    };
    let mut active = BTreeSet::new();
    resolve(graph, root, &mut active, &mut result)?;
    let mut validated = result.clone();
    for name in graph.nodes.keys() {
        resolve(graph, name, &mut active, &mut validated)?;
    }
    Ok(result)
}
fn validate_argument(arg: &Argument, inputs: &BTreeMap<String, Input>) -> Result<()> {
    if arg.0.is_empty() || arg.0.len() > 256 || serde_json::to_vec(arg)?.len() > 32768 {
        return Err(Error::Invalid(
            "argument exceeds 32 KiB/segment limit or is empty".into(),
        ));
    }
    for seg in &arg.0 {
        match seg {
            Segment::Literal { value } => {
                if value.contains('\0') || value.contains(LOGICAL_PREFIX) {
                    return Err(Error::Invalid(
                        "literal contains NUL or untyped store path".into(),
                    ));
                }
            }
            Segment::Input { name, path } => {
                if !inputs.contains_key(name) {
                    return Err(Error::Invalid(format!("unknown input {name}")));
                }
                relative(path, true)?;
            }
            Segment::Output { path } => relative(path, true)?,
        }
    }
    Ok(())
}
fn resolve(
    graph: &BuildGraph,
    key: &str,
    active: &mut BTreeSet<String>,
    result: &mut Plan,
) -> Result<()> {
    if result.outputs.contains_key(key) {
        return Ok(());
    }
    if !active.insert(key.into()) {
        return Err(Error::Invalid(format!("dependency cycle at {key}")));
    }
    let node = graph
        .nodes
        .get(key)
        .ok_or_else(|| Error::Invalid(format!("unknown root {key}")))?;
    let mut inputs = BTreeMap::new();
    for (alias, input) in &node.inputs {
        let id = match input {
            Input::Object(id) => id.clone(),
            Input::Node(n) => {
                resolve(graph, n, active, result)?;
                result.outputs[n].clone()
            }
        };
        inputs.insert(alias.clone(), id);
    }
    let mut runtime_inputs: Vec<_> = node
        .runtime_inputs
        .iter()
        .map(|a| inputs[a].clone())
        .collect();
    runtime_inputs.sort();
    runtime_inputs.dedup();
    for alias in &node.runtime_inputs {
        if let Input::Node(n) = &node.inputs[alias]
            && graph.nodes[n].runtime_image != node.runtime_image
        {
            return Err(Error::Invalid(
                "runtime closure has different image foundations".into(),
            ));
        }
    }
    let spec = ResolvedBuildSpec {
        schema: 1,
        platform: PLATFORM.into(),
        prefix: LOGICAL_PREFIX.into(),
        policy: "v1:uid-owner:memory1GiB:pids256:tmp256MiB:netnone:ro:timeout".into(),
        builder_image: node.builder_image.clone(),
        runtime_image: node.runtime_image.clone(),
        inputs,
        argv: node.argv.clone(),
        env: node.env.clone(),
        runtime_inputs,
        timeout_seconds: node.timeout_seconds,
    };
    result.outputs.insert(key.into(), derivation_id(&spec)?);
    result.specs.insert(key.into(), spec);
    result.order.push(key.into());
    active.remove(key);
    Ok(())
}
pub(crate) fn derivation_id(spec: &ResolvedBuildSpec) -> Result<String> {
    let mut bytes = b"sysroot-engine-derivation-v1\0".to_vec();
    bytes.extend(serde_json::to_vec(spec)?);
    Ok(format!("out-{}", hash(&bytes)))
}
pub(crate) fn timeout(seconds: u64) -> Result<()> {
    if seconds == 0 || seconds > 3600 {
        return Err(Error::Invalid("timeout must be 1..=3600 seconds".into()));
    }
    Ok(())
}
#[cfg(unix)]
pub(crate) fn render(arg: &Argument, spec: &ResolvedBuildSpec, output: &str) -> Result<String> {
    let mut s = String::new();
    for seg in &arg.0 {
        match seg {
            Segment::Literal { value } => s.push_str(value),
            Segment::Input { name, path } => {
                let id = spec
                    .inputs
                    .get(name)
                    .ok_or_else(|| Error::Invalid(format!("unknown input {name}")))?;
                s.push_str(&format!("{LOGICAL_PREFIX}/{id}"));
                if !path.is_empty() {
                    s.push('/');
                    s.push_str(path);
                }
            }
            Segment::Output { path } => {
                s.push_str(&format!("{LOGICAL_PREFIX}/{output}"));
                if !path.is_empty() {
                    s.push('/');
                    s.push_str(path);
                }
            }
        }
    }
    if s.len() > 32768 {
        return Err(Error::Invalid("expanded argument exceeds 32 KiB".into()));
    }
    Ok(s)
}
pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
