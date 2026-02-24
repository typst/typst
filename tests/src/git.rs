use std::path::Path;

use ecow::{EcoString, eco_format};
use time::format_description::well_known::Rfc3339;
use typst::diag::{StrResult, bail};
use unscanny::Scanner;

/// Parse a commitish and resolve the revision.
pub fn resolve_commit(commitish: &str) -> StrResult<String> {
    let parse = format!("{commitish}^{{commit}}");
    let bytes = git().args(["rev-parse", "--verify", &parse]).run()?;
    let rev = String::from_utf8(bytes).map_err(|_| "commit hash isn't valid utf-8")?;
    Ok(rev)
}

/// Read a file from a specific git revision.
pub fn read_file(revision: &str, ref_path: &Path) -> Option<Vec<u8>> {
    let rev_file = format!("{revision}:{}", ref_path.display());
    git().args(["show", &rev_file]).run().ok()
}

/// Runs git blame on the file path and returns a list of of git revision,
/// timestamp, and line-text tuples.
pub fn blame_file(
    revision: Option<&str>,
    path: &Path,
) -> StrResult<Vec<(EcoString, i64, EcoString)>> {
    let blame = git()
        .args(["blame", "-e", "--date=iso8601-strict"])
        .args(revision)
        .arg(path)
        .run()?;
    let blame = std::str::from_utf8(&blame).map_err(|err| err.to_string())?;

    // Format of git blame is:
    // `{hash} (<{email}> {date} {padded_nr}) {text}`
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
                bail!("expected email end (`>`)");
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

fn git() -> std::process::Command {
    std::process::Command::new("git")
}

trait Run {
    fn run(&mut self) -> StrResult<Vec<u8>>;
}

impl Run for std::process::Command {
    fn run(&mut self) -> StrResult<Vec<u8>> {
        let output = self.output().map_err(|err| err.to_string())?;
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
}
