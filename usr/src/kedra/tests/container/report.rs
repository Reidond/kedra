//! Results and artifacts for one execution.
//!
//! Layout: `<artifacts>/runs/<execution>/report.json` and one
//! directory per test with `steps.json`, screenshots and, on failure, the
//! journal and unit state collected before the container was removed.

use std::fs;
use std::path::Path;

use crate::Result;
use crate::image::LabImage;
use crate::scenario::StepRecord;

fn replace(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    use std::io::Write;
    let pending = path.with_extension("pending");
    let mut file = fs::File::create(&pending)?;
    file.write_all(contents.as_ref())?;
    file.sync_all()?;
    fs::rename(pending, path)?;
    Ok(())
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct TestResult {
    pub name: String,
    pub kind: &'static str,
    pub outcome: &'static str,
    pub duration_ms: u128,
    pub failure: Option<String>,
    /// Reported separately; never replaces a test failure.
    pub cleanup_failure: Option<String>,
    pub retained_container: Option<String>,
    pub artifacts: String,
    pub steps: Vec<StepRecord>,
}

#[derive(Debug, serde::Serialize)]
pub struct Report<'a> {
    pub schema_version: u32,
    pub interrupted: bool,
    pub selected_count: usize,
    pub execution: &'a str,
    pub image: Option<&'a LabImage>,
    pub results: &'a [TestResult],
}

/// Write report.json atomically.
pub fn write(directory: &Path, report: &Report<'_>) -> Result<()> {
    fs::create_dir_all(directory)?;
    replace(
        &directory.join("report.json"),
        serde_json::to_vec_pretty(report).map_err(|error| crate::invalid(error.to_string()))?,
    )
}
