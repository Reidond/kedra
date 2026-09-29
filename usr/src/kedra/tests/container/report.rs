//! Results, artifacts and CI reports for one execution.
//!
//! Layout: `<artifacts>/runs/<execution>/report.json`, `junit.xml` and one
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

fn escape(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .map(|c| match c {
            '&' => "&amp;".to_owned(),
            '<' => "&lt;".to_owned(),
            '>' => "&gt;".to_owned(),
            '"' => "&quot;".to_owned(),
            '\'' => "&apos;".to_owned(),
            other => other.to_string(),
        })
        .collect()
}

/// Write report.json and a JUnit XML file for CI test reporters.
pub fn write(directory: &Path, report: &Report<'_>) -> Result<()> {
    fs::create_dir_all(directory)?;
    replace(
        &directory.join("report.json"),
        serde_json::to_vec_pretty(report).map_err(|error| crate::invalid(error.to_string()))?,
    )?;
    let failures = report
        .results
        .iter()
        .filter(|result| result.outcome == "failed")
        .count();
    let total_ms: u128 = report.results.iter().map(|result| result.duration_ms).sum();
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<testsuite name=\"kedra-container\" tests=\"{}\" failures=\"{failures}\" time=\"{:.3}\">\n",
        report.results.len(),
        total_ms as f64 / 1000.0
    );
    if let Some(image) = report.image {
        xml.push_str(&format!(
            "  <properties><property name=\"image\" value=\"{}\"/><property name=\"base\" value=\"{}\"/><property name=\"source\" value=\"{}\"/></properties>\n",
            escape(&image.reference()),
            escape(&image.base),
            escape(&image.source)
        ));
    }
    for result in report.results {
        xml.push_str(&format!(
            "  <testcase classname=\"kedra.{}\" name=\"{}\" time=\"{:.3}\">\n",
            result.kind,
            escape(&result.name),
            result.duration_ms as f64 / 1000.0
        ));
        if let Some(failure) = &result.failure {
            xml.push_str(&format!(
                "    <failure message=\"{}\">{}</failure>\n",
                escape(failure.lines().next().unwrap_or_default()),
                escape(failure)
            ));
        }
        if let Some(cleanup) = &result.cleanup_failure {
            xml.push_str(&format!(
                "    <system-err>cleanup: {}</system-err>\n",
                escape(cleanup)
            ));
        }
        xml.push_str(&format!(
            "    <system-out>artifacts: {}</system-out>\n",
            escape(&result.artifacts)
        ));
        xml.push_str("  </testcase>\n");
    }
    xml.push_str("</testsuite>\n");
    replace(&directory.join("junit.xml"), xml)?;
    Ok(())
}
