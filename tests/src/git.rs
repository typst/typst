use std::path::Path;

use ecow::EcoString;
use typst::diag::{StrResult, bail};

/// Parse a commitish and resolve the revision.
pub fn resolve_commit(commitish: &str) -> StrResult<String> {
    let parse = format!("{commitish}^{{commit}}");
    let bytes = command(&["rev-parse", "--verify", &parse])?;
    let rev = String::from_utf8(bytes).map_err(|_| "commit hash isn't valid utf-8")?;
    Ok(rev)
}

/// Read a file from a specific git revision.
pub fn read_file(revision: &str, ref_path: &Path) -> Option<Vec<u8>> {
    let rev_file = format!("{revision}:{}", ref_path.display());
    command(&["show", &rev_file]).ok()
}

/// Run a git command
pub fn command(args: &[&str]) -> StrResult<Vec<u8>> {
    dbg!(args);
    let output = std::process::Command::new("git")
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;
    if !output.stderr.is_empty() {
        let message = match std::str::from_utf8(&output.stderr) {
            Ok(msg) => EcoString::from(msg),
            Err(err) => bail!("{err}"),
        };
        return Err(message);
    }
    if output.stdout.is_empty() {
        return Err("stdout is empty".into());
    }
    Ok(output.stdout)
}
