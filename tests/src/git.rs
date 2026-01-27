use std::path::Path;

use ecow::{EcoString, eco_format};
use time::format_description::well_known::Rfc3339;
use typst::diag::{StrResult, bail};
use unscanny::Scanner;

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

/// Runs git blame on the file path and returns a list of of git revision,
/// timestamp, and line-text tuples.
pub fn blame_file(path: &Path) -> StrResult<Vec<(EcoString, i64, EcoString)>> {
    let blame =
        command(&["blame", "-e", "--date=iso8601-strict", path.to_str().unwrap()])?;
    let blame = std::str::from_utf8(&blame).map_err(|err| err.to_string())?;

    // Format of git blame is:
    // `<hash> <padded_nr>) <text>
    blame
        .lines()
        .map(|line| {
            let mut s = Scanner::new(line);

            let hash = s.eat_until(char::is_whitespace);
            s.eat_whitespace();

            if !s.eat_if('(') {
                bail!("expected `(`");
            }

            // email
            if !s.eat_if('<') {
                bail!("expected email start (`<`)");
            }
            s.eat_until('>');
            if !s.eat_if('>') {
                bail!("expected email end (`<`)");
            }
            s.eat_whitespace();

            // date
            let date = s.eat_until(char::is_whitespace);
            s.eat_whitespace();

            // line number
            s.eat_until(')');
            if !s.eat_if(')') {
                bail!("expected `)`");
            }

            if !s.eat_if(' ') {
                bail!("expected single whitespace before line text");
            }

            // RFC 3339 is essentially the strict ISO-8601 format.
            let date = time::OffsetDateTime::parse(date, &Rfc3339)
                .map_err(|err| eco_format!("failed to parse date: {err}"))?;

            let text = s.after();

            Ok((hash.into(), date.unix_timestamp(), text.into()))
        })
        .collect::<StrResult<_>>()
}

/// Run a git command
pub fn command(args: &[&str]) -> StrResult<Vec<u8>> {
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
