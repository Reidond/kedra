//! Versioned, data-only package authoring. No module executes author code.
mod check;
mod format;
mod lower;
mod parser;
pub use lower::{Images, Lowered, lower};

pub use check::{
    Binding, Content, ExportIntent, Intent, Lockfile, PatchIntent, RecipeIntent, SourceIntent,
    SourcePin, TargetPolicy, TemplateIntent, compile,
};
pub use format::format;
pub use parser::{Block, Declaration, Export, Import, Module, Span, Value, parse};

use serde::{Deserialize, Serialize};

pub const MAX_INPUT: usize = 8 * 1024 * 1024;
pub const MAX_TOTAL: usize = 32 * 1024 * 1024;
pub const MAX_MODULES: usize = 128;
pub const MAX_RESOURCES: usize = 4096;
pub const MAX_DEPTH: usize = 64;
pub const MAX_VALUES: usize = 262_144;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub related: Vec<Location>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub file: String,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}: {}: {}",
            self.file, self.line, self.column, self.code, self.message
        )?;
        for origin in &self.related {
            write!(
                f,
                "; related {}:{}:{}",
                origin.file, origin.line, origin.column
            )?;
        }
        Ok(())
    }
}
impl std::error::Error for Diagnostic {}
pub type Result<T> = std::result::Result<T, Diagnostic>;

pub(crate) fn fail(file: &str, span: Span, code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        file: file.into(),
        line: span.line,
        column: span.column,
        message: message.into(),
        related: Vec::new(),
    }
}

pub fn relative(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && path.is_ascii()
        && !path
            .bytes()
            .any(|c| c.is_ascii_control() || b"\\:".contains(&c))
        && path
            .split('/')
            .all(|p| !matches!(p, "" | "." | ".." | ".git"))
}

pub fn public_input(path: &str, bytes: &[u8]) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    !parts.iter().any(|part| {
        matches!(
            *part,
            ".ssh"
                | ".codex"
                | ".claude"
                | ".cache"
                | "auth.json"
                | "credentials.json"
                | ".credentials.json"
        )
    }) && ![
        b"-----BEGIN PRIVATE KEY-----".as_slice(),
        b"-----BEGIN OPENSSH PRIVATE KEY-----",
        b"-----BEGIN RSA PRIVATE KEY-----",
        b"-----BEGIN EC PRIVATE KEY-----",
        b"-----BEGIN ENCRYPTED PRIVATE KEY-----",
    ]
    .iter()
    .any(|marker| bytes.windows(marker.len()).any(|window| window == *marker))
}

pub fn import_path(module: &str, path: &str) -> Option<String> {
    let local = path.strip_prefix("./")?;
    if !relative(local) {
        return None;
    }
    let prefix = module.rsplit_once('/').map_or("", |(dir, _)| dir);
    Some(if prefix.is_empty() {
        local.into()
    } else {
        format!("{prefix}/{local}")
    })
}
