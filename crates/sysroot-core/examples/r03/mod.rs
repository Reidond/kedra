//! Deliberately narrow research machinery, excluded from the production library.
mod git;
#[cfg(test)]
mod tests;

use git::Git;
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Git {
        command: String,
        code: Option<i32>,
        diagnostic: String,
    },
    Unsupported(&'static str),
    Classification(&'static str),
    ContentConflict,
    DirtySource,
    MissingSelection,
    Evidence(&'static str),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "fixture I/O: {e}"),
            Self::Git {
                command,
                code,
                diagnostic,
            } => write!(f, "git {command} exited {code:?}: {diagnostic}"),
            Self::Unsupported(s) => write!(f, "unsupported fixture input: {s}"),
            Self::Classification(s) => write!(f, "classification conflict; review again: {s}"),
            Self::ContentConflict => write!(
                f,
                "content conflict; candidate retained away from live/source files"
            ),
            Self::DirtySource => write!(f, "source has existing changes; export refused"),
            Self::MissingSelection => write!(f, "no reviewed selection"),
            Self::Evidence(s) => write!(f, "experiment assertion failed: {s}"),
        }
    }
}
impl std::error::Error for Error {}

const BASE: &str = "font=12\nborder=2\nanimation=200\n";
const LIVE: &str = "font=14\nborder=4\nanimation=150\n";
const SOURCE_PATH: &str = "hosts/desktop/home/.config/demo.conf";
const LIVE_PATH: &str = ".config/demo.conf";

/// Content anchors bound to an entire baseline, never persisted line numbers.
#[derive(Clone, Debug)]
struct Change {
    baseline: String,
    before: String,
    after: String,
}

impl Change {
    fn new(baseline: &str, before: &str, after: &str) -> Self {
        Self {
            baseline: baseline.into(),
            before: before.into(),
            after: after.into(),
        }
    }

    fn validate(&self, baseline: &str, live: &str) -> Result<()> {
        validate_text(live)?;
        if self.baseline != baseline || self.before == self.after {
            return Err(Error::Classification("baseline changed or empty decision"));
        }
        // The bounded experiment handles one full-line replacement. Insert/delete
        // and relocation need new review, not fuzzy context matching.
        let old: Vec<_> = baseline.split_inclusive('\n').collect();
        let new: Vec<_> = live.split_inclusive('\n').collect();
        if old.len() != new.len()
            || self.before.split_inclusive('\n').count() != 1
            || self.after.split_inclusive('\n').count() != 1
        {
            return Err(Error::Classification(
                "line insertion/deletion or multiline decision",
            ));
        }
        let old_matches: Vec<_> = old
            .iter()
            .enumerate()
            .filter(|(_, s)| **s == self.before)
            .collect();
        let new_matches: Vec<_> = new
            .iter()
            .enumerate()
            .filter(|(_, s)| **s == self.after)
            .collect();
        if old_matches.len() != 1 || new_matches.len() != 1 {
            return Err(Error::Classification(
                "changed value or ambiguous content anchor",
            ));
        }
        if old_matches[0].0 != new_matches[0].0 {
            return Err(Error::Classification(
                "anchor moved; explicit review required",
            ));
        }
        Ok(())
    }
}

fn validate_text(text: &str) -> Result<()> {
    if text.contains(['\r', '\0', '\u{feff}']) {
        return Err(Error::Unsupported("only UTF-8 LF text without BOM/NUL"));
    }
    Ok(())
}

fn validate_path(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains(['\\', ':', '\n', '\r', '\0'])
        || Path::new(path)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path
            .split('/')
            .any(|c| c.is_empty() || c == "." || c.eq_ignore_ascii_case(".git"))
    {
        return Err(Error::Unsupported(
            "path must be a safe relative UTF-8 fixture path",
        ));
    }
    Ok(())
}

fn write(root: &Path, path: &str, text: &str) -> Result<()> {
    validate_path(path)?;
    let file = root.join(path);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, text)?;
    Ok(())
}

struct Selection {
    tree: String,
    text: String,
}
struct Export {
    tree: String,
    parent: String,
}
struct Published {
    selection: String,
    commit: String,
}

struct Fixture {
    root: PathBuf,
    live: PathBuf,
    review: Git,
    source: Git,
    source_path: String,
    baseline: String,
    baseline_commit: String,
    source_base: String,
    selected: Option<Selection>,
    ignored: Vec<Change>,
    published: Option<Published>,
    attempts: u32,
}

impl Fixture {
    fn new(baseline: &str, source_path: &str) -> Result<Self> {
        validate_text(baseline)?;
        validate_path(source_path)?;
        let parent = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/r03");
        fs::create_dir_all(&parent)?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Evidence("clock before epoch"))?
            .as_nanos();
        let root = parent.join(format!("{}-{stamp}", std::process::id()));
        // create_dir refuses existing roots. No fixture adopts or deletes a path.
        fs::create_dir(&root)?;
        let root = root.canonicalize()?;
        fs::write(root.join("empty-config"), "")?;
        fs::create_dir(root.join("empty-hooks"))?;
        let live = root.join("live-home");
        fs::create_dir(&live)?;
        write(&live, LIVE_PATH, baseline)?;
        let review = Git::new(root.join("private-review"), &root)?;
        let source = Git::new(root.join("source"), &root)?;
        for (git, label) in [(&review, "private baseline"), (&source, "source baseline")] {
            write(&git.root, source_path, baseline)?;
            git.stage(source_path, baseline)?;
            git.run(&["commit", "--quiet", "-m", label], None, None)?;
        }
        let baseline_commit = review.oid(&["rev-parse", "HEAD"])?;
        let source_base = source.oid(&["rev-parse", "HEAD"])?;
        Ok(Self {
            root,
            live,
            review,
            source,
            source_path: source_path.into(),
            baseline: baseline.into(),
            baseline_commit,
            source_base,
            selected: None,
            ignored: vec![],
            published: None,
            attempts: 0,
        })
    }

    fn app_write(&self, text: &str) -> Result<()> {
        write(&self.live, LIVE_PATH, text)
    }
    fn live_text(&self) -> Result<String> {
        Ok(fs::read_to_string(self.live.join(LIVE_PATH))?)
    }

    fn capture(&self) -> Result<()> {
        let text = self.live_text()?;
        validate_text(&text)?;
        // Exactly the one fixture allowlist entry; never walk a home directory.
        write(&self.review.root, &self.source_path, &text)
    }

    fn ignore(&mut self, change: Change) -> Result<()> {
        change.validate(&self.baseline, &self.live_text()?)?;
        if let Some(selected) = &self.selected
            && !selected
                .text
                .split_inclusive('\n')
                .any(|s| s == change.before)
        {
            return Err(Error::Classification(
                "local-only overlaps selected content",
            ));
        }
        if self.ignored.iter().any(|i| i.before == change.before) {
            return Err(Error::Classification("duplicate local-only decision"));
        }
        self.ignored.push(change);
        Ok(())
    }

    fn stage(&mut self, changes: &[Change]) -> Result<()> {
        let live = self.live_text()?;
        let mut text = self.baseline.clone();
        for change in changes {
            change.validate(&self.baseline, &live)?;
            if self.ignored.iter().any(|i| i.before == change.before) {
                return Err(Error::Classification(
                    "selection overlaps local-only policy",
                ));
            }
            if text
                .split_inclusive('\n')
                .filter(|s| *s == change.before)
                .count()
                != 1
            {
                return Err(Error::Classification("overlapping selections"));
            }
            text = text
                .split_inclusive('\n')
                .map(|line| {
                    if line == change.before {
                        change.after.as_str()
                    } else {
                        line
                    }
                })
                .collect();
        }
        self.review.stage(&self.source_path, &text)?;
        self.selected = Some(Selection {
            tree: self.review.oid(&["write-tree"])?,
            text,
        });
        Ok(())
    }

    fn patch(&self) -> Result<String> {
        let selected = self.selected.as_ref().ok_or(Error::MissingSelection)?;
        self.review.run(
            &[
                "diff",
                "--binary",
                "--full-index",
                "--no-ext-diff",
                "--no-textconv",
                &self.baseline_commit,
                &selected.tree,
                "--",
                &self.source_path,
            ],
            None,
            None,
        )
    }

    fn export(&mut self) -> Result<Export> {
        let selected = self.selected.as_ref().ok_or(Error::MissingSelection)?;
        let live = self.live_text()?;
        for ignored in &self.ignored {
            ignored.validate(&self.baseline, &live)?;
            if !selected
                .text
                .split_inclusive('\n')
                .any(|s| s == ignored.before)
            {
                return Err(Error::Classification("export overlaps local-only policy"));
            }
        }
        if !self
            .source
            .run(
                &["status", "--porcelain", "--untracked-files=all"],
                None,
                None,
            )?
            .is_empty()
        {
            return Err(Error::DirtySource);
        }
        let parent = self.source.oid(&["rev-parse", "HEAD"])?;
        self.source.run(
            &["merge-base", "--is-ancestor", &self.source_base, &parent],
            None,
            None,
        )?;
        self.attempts += 1;
        let index = self.root.join(format!("export-{}.index", self.attempts));
        self.source
            .run(&["read-tree", &parent], None, Some(&index))?;
        let patch = self.patch()?;
        fs::write(self.root.join("selected.patch"), &patch)?;
        if !patch.is_empty() {
            let output = self.source.output(
                &["apply", "--cached", "--3way", "--whitespace=nowarn", "-"],
                Some(patch.as_bytes()),
                Some(&index),
            )?;
            fs::write(self.root.join("export-diagnostic.txt"), &output.stderr)?;
            if !output.status.success() {
                let unmerged = self
                    .source
                    .run(&["ls-files", "--unmerged"], None, Some(&index))?;
                if !unmerged.is_empty() {
                    return Err(Error::ContentConflict);
                }
                return Err(Error::Git {
                    command: "apply --cached --3way".into(),
                    code: output.status.code(),
                    diagnostic: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
        }
        let tree = self
            .source
            .run(&["write-tree"], None, Some(&index))?
            .trim()
            .to_owned();
        Ok(Export { tree, parent })
    }

    /// Synthetic publication only. Source ancestry starts at the source parent;
    /// there is no fetch/remote/object-store sharing with private review.
    fn publish(&mut self, export: Export) -> Result<()> {
        if self.source.oid(&["rev-parse", "HEAD"])? != export.parent {
            return Err(Error::Classification(
                "source advanced after export preparation",
            ));
        }
        if !self
            .source
            .run(&["status", "--porcelain"], None, None)?
            .is_empty()
        {
            return Err(Error::DirtySource);
        }
        let commit = if self.source.oid(&["rev-parse", "HEAD^{tree}"])? == export.tree {
            export.parent.clone()
        } else {
            let commit = self
                .source
                .run(
                    &["commit-tree", &export.tree, "-p", &export.parent],
                    Some(b"selected fixture export\n"),
                    None,
                )?
                .trim()
                .to_owned();
            self.source
                .run(&["update-ref", "HEAD", &commit, &export.parent], None, None)?;
            self.source.run(&["read-tree", &export.tree], None, None)?;
            let text = self.source.run(
                &["show", &format!("{}:{}", export.tree, self.source_path)],
                None,
                None,
            )?;
            write(&self.source.root, &self.source_path, &text)?;
            commit
        };
        self.published = Some(Published {
            selection: self
                .selected
                .as_ref()
                .ok_or(Error::MissingSelection)?
                .tree
                .clone(),
            commit,
        });
        Ok(())
    }

    fn merge_candidate(&self, next: &str) -> Result<String> {
        validate_text(next)?;
        fs::write(self.root.join("B"), &self.baseline)?;
        fs::write(self.root.join("L"), self.live_text()?)?;
        fs::write(self.root.join("N"), next)?;
        let output = self.review.output(
            &[
                "merge-file",
                "-p",
                "-L",
                "live",
                "-L",
                "baseline",
                "-L",
                "next",
                "../L",
                "../B",
                "../N",
            ],
            None,
            None,
        )?;
        fs::write(self.root.join("merge-candidate.txt"), &output.stdout)?;
        match output.status.code() {
            Some(0) => {
                String::from_utf8(output.stdout).map_err(|_| Error::Unsupported("non-UTF-8 merge"))
            }
            Some(1..=127) => Err(Error::ContentConflict),
            code => Err(Error::Git {
                command: "merge-file".into(),
                code,
                diagnostic: String::from_utf8_lossy(&output.stderr).into_owned(),
            }),
        }
    }
}

fn require(value: bool, message: &'static str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::Evidence(message))
    }
}

pub fn demo() -> Result<()> {
    let mut fixture = Fixture::new(BASE, SOURCE_PATH)?;
    println!("Synthetic evidence: {}", fixture.root.display());
    fixture.app_write(LIVE)?;
    fixture.capture()?;
    fixture.ignore(Change::new(BASE, "border=2\n", "border=4\n"))?;
    fixture.stage(&[Change::new(BASE, "font=12\n", "font=14\n")])?;
    let staged = fixture.review.oid(&["write-tree"])?;
    let patch = fixture.patch()?;
    fixture.app_write("font=16\nborder=4\nanimation=150\n")?;
    fixture.capture()?;
    require(
        fixture.review.oid(&["write-tree"])? == staged,
        "app write changed staged tree",
    )?;
    println!("pass: one selected line; later font=16 remains unstaged; S={staged}");
    println!("Selected export patch:\n{patch}");
    let export = fixture.export()?;
    fixture.publish(export)?;
    let actual = fs::read_to_string(fixture.source.root.join(SOURCE_PATH))?;
    require(
        actual == "font=14\nborder=2\nanimation=200\n",
        "source contains unselected content",
    )?;
    if let Some(published) = &fixture.published {
        println!(
            "pass: source={} contains only S={}; published, not deployed",
            published.commit, published.selection
        );
    }
    let before = fixture.live_text()?;
    require(
        matches!(
            fixture.merge_candidate("font=12\nborder=3\nanimation=250\n"),
            Err(Error::ContentConflict)
        ),
        "expected baseline conflict",
    )?;
    require(
        fixture.live_text()? == before && fixture.baseline == BASE,
        "conflict modified live or accepted baseline",
    )?;
    println!("pass: baseline conflict explicit; L and B unchanged; candidate retained privately");
    Ok(())
}
