//! HTML test report generation logic.
//!
//! When the test suite fails because of mismatched output, a HTML test report
//! is generated that allows inspecting diffs of the failed tests. This module
//! is able to generate text and image diffs that are combined into a static
//! self-contained HTML report.
//!
//! The report is stored in `tests/store/report.html` alongside an optional
//! `missing.txt` file. If the `missing.txt` file is present, there is at least
//! one old live output from the [`HashedRefs`] missing, meaning the full diff
//! can't be generated. In that case the test wrapper in `tests/wrapper` will
//! ask if the missing live output should be regenerated and rerun an old git
//! revision of the test suite.
//! This can also be initiated manually using the `cargo testit regen` command.

mod diff;
mod html;

pub use self::diff::{DiffKind, File, Old, ReportFile, image_diff, text_diff};

use std::fmt::Write as _;
use std::path::Path;

use ecow::EcoString;
use indexmap::IndexMap;
use rustc_hash::FxBuildHasher;

use crate::output::{HASH_OUTPUTS, HashedRefs};
use crate::{ARGS, STORE_PATH, git};

/// A test report for a single test.
pub struct TestReport {
    pub name: EcoString,
    pub files: Vec<ReportFile>,
}

impl TestReport {
    pub fn new(name: EcoString) -> Self {
        Self { name, files: Vec::new() }
    }
}

/// Returns whether to prompt the user for regeneration of missing old live
/// output, to generate a full test report.
pub fn write(mut reports: Vec<TestReport>) -> Result<bool, ()> {
    let report_path = Path::new(STORE_PATH).join("report.html");
    let missing_path = Path::new(STORE_PATH).join("missing.txt");

    reports.sort_by(|a, b| a.name.cmp(&b.name));
    let html = html::generate(&reports);
    std::fs::write(report_path, html).unwrap();

    // Check if old live output is missing, and could be regenerated.
    let mut missing_live = (reports.iter())
        .flat_map(|report| std::iter::repeat(&report.name).zip(report.files.iter()))
        .filter_map(|(name, file)| {
            let hash_ref = file.diffs.iter().find_map(DiffKind::missing_old)?;
            Some(((name.as_str(), file.output), hash_ref))
        })
        .collect::<IndexMap<_, _, FxBuildHasher>>();
    if missing_live.is_empty() {
        std::fs::remove_file(missing_path).ok();
        return Ok(false);
    }
    missing_live.sort_by(|a, _, b, _| a.cmp(b));

    // Find the git revisions in which the hash references that are missing live
    // output were committed.
    let mut newest_update_rev = None;
    let mut missing_old_revs = Vec::new();
    for output in HASH_OUTPUTS {
        let base_rev = ARGS.base_revision.as_deref();
        let blame_lines = git::blame_file(base_rev, &output.hash_refs_path()).unwrap();
        for (rev_hash, timestamp, text) in blame_lines {
            let (name, _) = HashedRefs::parse_line(&text).unwrap();

            if newest_update_rev.as_ref().is_none_or(|(_, t)| *t < timestamp) {
                newest_update_rev = Some((rev_hash.clone(), timestamp));
            }

            if !missing_live.contains_key(&(name.as_str(), output)) {
                continue;
            }
            if missing_old_revs.iter().any(|(r, _)| r == &rev_hash) {
                continue;
            }

            missing_old_revs.push((rev_hash, timestamp));
        }
    }
    let Some((newest_update_rev, _)) = newest_update_rev else {
        eprintln!("couldn't find git revision at which hash references were updated");
        std::fs::remove_file(missing_path).ok();
        return Err(());
    };
    if missing_old_revs.is_empty() {
        eprintln!("warning: couldn't find git revisions for missing live output");
    }
    missing_old_revs.sort_by_key(|(_, timestamp)| *timestamp);

    let mut text = String::new();
    writeln!(text, "newest-update-rev: {newest_update_rev}").unwrap();
    writeln!(text, "missing-old-revs:").unwrap();
    for (rev_hash, _) in missing_old_revs.iter().rev() {
        writeln!(text, "- {rev_hash}").unwrap();
    }
    writeln!(text, "missing-live:").unwrap();
    for ((name, output), hash_ref) in missing_live.iter() {
        let path = output.hash_path(*hash_ref, name);
        writeln!(text, "- {}", path.display()).unwrap();
    }

    std::fs::write(missing_path, text).unwrap();
    Ok(true)
}
