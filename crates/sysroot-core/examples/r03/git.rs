use super::{Error, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

// Rust canonicalize returns a Windows verbatim path. Git's config/index path
// parser needs the ordinary slash form for these generated local fixture paths.
fn git_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    #[cfg(windows)]
    {
        value
            .strip_prefix(r"\\?\")
            .unwrap_or(&value)
            .replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        value.into_owned()
    }
}

pub struct Git {
    pub root: PathBuf,
    config: PathBuf,
    hooks: PathBuf,
}

impl Git {
    pub fn new(root: PathBuf, sandbox: &Path) -> Result<Self> {
        std::fs::create_dir(&root)?;
        let git = Self {
            root,
            config: sandbox.join("empty-config"),
            hooks: sandbox.join("empty-hooks"),
        };
        git.run(
            &["init", "--quiet", "--initial-branch=fixture", "--template="],
            None,
            None,
        )?;
        Ok(git)
    }

    pub fn output(
        &self,
        args: &[&str],
        input: Option<&[u8]>,
        index: Option<&Path>,
    ) -> Result<Output> {
        let mut command = Command::new("git");
        // Do not inherit an index, object store, config injection or repository override.
        for (key, _) in std::env::vars_os() {
            if key
                .to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GIT_")
            {
                command.env_remove(key);
            }
        }
        command
            .current_dir(&self.root)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_ATTR_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", git_path(&self.config))
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_AUTHOR_NAME", "R03 Fixture")
            .env("GIT_AUTHOR_EMAIL", "r03@example.invalid")
            .env("GIT_COMMITTER_NAME", "R03 Fixture")
            .env("GIT_COMMITTER_EMAIL", "r03@example.invalid")
            .env("GIT_AUTHOR_DATE", "2000-01-01T00:00:00Z")
            .env("GIT_COMMITTER_DATE", "2000-01-01T00:00:00Z")
            .args([
                "-c",
                "core.autocrlf=false",
                "-c",
                "core.safecrlf=false",
                "-c",
                "commit.gpgsign=false",
            ])
            .arg("-c")
            .arg(format!("core.hooksPath={}", git_path(&self.hooks)))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(index) = index {
            command.env("GIT_INDEX_FILE", git_path(index));
        }
        let mut child = command.spawn()?;
        if let Some(mut stdin) = child.stdin.take()
            && let Some(input) = input
        {
            stdin.write_all(input)?;
        }
        Ok(child.wait_with_output()?)
    }

    pub fn run(&self, args: &[&str], input: Option<&[u8]>, index: Option<&Path>) -> Result<String> {
        let output = self.output(args, input, index)?;
        if !output.status.success() {
            return Err(Error::Git {
                command: args.join(" "),
                code: output.status.code(),
                diagnostic: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        String::from_utf8(output.stdout).map_err(|_| Error::Unsupported("non-UTF-8 Git output"))
    }

    pub fn oid(&self, args: &[&str]) -> Result<String> {
        Ok(self.run(args, None, None)?.trim().to_owned())
    }

    pub fn blob(&self, text: &str) -> Result<String> {
        Ok(self
            .run(
                &["hash-object", "-w", "--stdin"],
                Some(text.as_bytes()),
                None,
            )?
            .trim()
            .to_owned())
    }

    pub fn stage(&self, path: &str, text: &str) -> Result<()> {
        let oid = self.blob(text)?;
        self.run(
            &["update-index", "--add", "--cacheinfo", "100644", &oid, path],
            None,
            None,
        )?;
        Ok(())
    }
}
