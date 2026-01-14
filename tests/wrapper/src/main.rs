//! A wrapper arond the test runner that allows rerunning an old version of the
//! test suite when old live output is missing.

use std::io::{IsTerminal, Write as _};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

/// The exit code used by the test runner to prompt regeneration.
const PROMPT_REGEN_EXIT_CODE: i32 = 15;

fn main() {
    let mut args = std::env::args().peekable();
    args.next();

    if args.peek().is_some_and(|arg| arg == "regen") {
        args.next();
        let cmd = parse_regen_command_args(args);
        regen(cmd);
        return;
    }

    let other_args = args.collect::<Vec<_>>();

    // Forward the args to real test runner.
    let status = Command::new("cargo")
        .args(["test", "--workspace", "--test=tests", "--"])
        .args(&other_args)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    if status.success() {
        return;
    }

    if status.code() == Some(PROMPT_REGEN_EXIT_CODE) {
        eprintln!();

        let stdin = std::io::stdin();
        if !stdin.is_terminal() || !confirm_input(stdin, "generate missing live output") {
            eprintln!("  run `cargo testit regen` to generate missing old live output");
            std::process::exit(status.code().unwrap_or(1));
        }

        regen(RegenCommand { rerun: true, other_args, ..Default::default() });
    }
}

#[derive(Default)]
struct RegenCommand {
    base_revision: Option<String>,
    rerun: bool,
    other_args: Vec<String>,
}

fn parse_regen_command_args(mut args: impl Iterator<Item = String>) -> RegenCommand {
    let mut cmd = RegenCommand::default();
    while let Some(arg) = args.next() {
        if let Some(val) = parse_arg_value(&arg, &mut args, "base-revision") {
            cmd.base_revision = Some(val);
        } else if arg == "--rerun" {
            cmd.rerun = true;
        } else if arg == "--" {
            cmd.other_args = args.collect();
            break;
        } else {
            eprintln!("unexpected argument {arg}");
            std::process::exit(1);
        }
    }
    cmd
}

fn parse_arg_value(
    arg: &str,
    mut args: impl Iterator<Item = String>,
    name: &str,
) -> Option<String> {
    let arg = arg.strip_prefix("--")?;
    let remainder = arg.strip_prefix(name)?;

    if let Some(val) = remainder.strip_prefix("=") {
        Some(val.to_string())
    } else {
        Some(args.next().unwrap_or_else(|| {
            eprintln!("expected value of `--{name}`");
            std::process::exit(1);
        }))
    }
}

fn regen(cmd: RegenCommand) {
    // Make sure the work tree is clean.
    Command::new("git")
        .args(["diff-index", "--quiet", "HEAD", "--"])
        .run()
        .exit_on_failure("git work tree is dirty, commit or stash your changes");

    let text;
    let (revs, mut missing) = if let Some(base_rev) = &cmd.base_revision {
        // When comparing against a base revision, only rerun if the test
        // references actually changed.
        let status = Command::new("git")
            .args(["diff-tree", "--quiet", "-r", "HEAD", base_rev, "--", "tests/ref"])
            .run();
        if status.success() {
            println!("test references weren't changed");
            return;
        }

        (vec![base_rev.as_str()], Vec::new())
    } else {
        text = match std::fs::read_to_string("tests/store/missing.txt") {
            Ok(text) => text,
            Err(_) => {
                println!("no `tests/store/missing.txt`");
                return;
            }
        };

        parse_missing(&text).unwrap_or_else(|err| {
            println!("failed to parse `tests/store/missing.txt`: {err}");
            std::process::exit(1);
        })
    };

    // Run tests for at most 3 old revisions.
    for rev in revs.iter().take(3) {
        read_tree(rev);

        let missing_names = missing
            .chunk_by(|a, b| test_name(a) == test_name(b))
            .map(|paths| test_name(paths[0]));
        // Allow a failing test suite.
        let status = Command::new("cargo")
            .args(["test", "--workspace", "--test=tests"])
            .args(["--", "--no-report", "--stages=svg,pdf", "--exact"])
            .args(["--"])
            .args(missing_names)
            .run();

        // If the test suite passes, there aren't any more missing live output.
        if status.success() {
            break;
        }

        // Even a failing test suite could produce all missing live output.
        // Check if there are any more paths missing.
        missing.retain(|path| !Path::new(path).exists());
        if missing.is_empty() {
            break;
        }
    }

    read_tree("HEAD");

    if cmd.rerun {
        // Allow a failing test suite.
        let base_rev = cmd.base_revision.map(|rev| format!("--base-revision={rev}"));
        Command::new("cargo")
            .args(["test", "--workspace", "--test=tests"])
            .arg("--")
            .args(&base_rev)
            .args(&cmd.other_args)
            .run();
    }
}

/// Parse the `tests/store/missing.txt` file generated by the tests suite report.
fn parse_missing(text: &str) -> Result<(Vec<&str>, Vec<&str>), &'static str> {
    // Although we have the exact commits at which the hash refernces were
    // added, it's probably a good idea to recompile a more recent commit that
    // won't have to compile a completely different dependency tree.
    //
    // That's why we try the last commit at which a hash reference was added
    // first. Unless the test suite was failing at that commit this will produce
    // all missing references.
    let mut lines = text.lines().peekable();
    let newest_update_rev = (lines.next())
        .and_then(|line| line.strip_prefix("newest-update-rev: "))
        .ok_or("expected newest-update-rev")?;

    let mut revs = vec![newest_update_rev];
    if lines.next() != Some("missing-old-revs:") {
        return Err("expected missing-old-revs");
    }
    while let Some(line) = lines.peek() {
        let Some(rev) = line.strip_prefix("- ") else { break };
        if !revs.contains(&rev) {
            revs.push(rev);
        }
        lines.next();
    }

    let mut missing = Vec::new();
    if lines.next() != Some("missing-live:") {
        return Err("expected missing-live");
    }
    while let Some(line) = lines.peek() {
        let Some(path) = line.strip_prefix("- ") else { break };
        missing.push(path);
        lines.next();
    }

    Ok((revs, missing))
}

fn test_name(hash_path: &str) -> &str {
    let start = hash_path.rfind('_').unwrap() + 1;
    let end = hash_path.rfind('.').unwrap();
    &hash_path[start..end]
}

fn read_tree(rev: &str) {
    println!("checking out `{rev}`");
    Command::new("git")
        .args(["read-tree", "-um", rev])
        .stdin(Stdio::null())
        .run()
        .exit_on_failure("failed to checkout `{rev}`");
}

trait Run {
    fn run(&mut self) -> ExitStatus;
}

impl Run for Command {
    fn run(&mut self) -> ExitStatus {
        let stdout = std::io::stdout();
        let use_colors = stdout.is_terminal();
        if use_colors {
            print!("\x1b[36m");
        }
        print!("> {}", self.get_program().display());
        for arg in self.get_args() {
            print!(" {}", arg.display());
        }
        if use_colors {
            print!("\x1b[0m");
        }
        println!();
        self.spawn().unwrap().wait().unwrap()
    }
}

trait ExitOnFailure {
    fn exit_on_failure(self, msg: &str);
}

impl ExitOnFailure for ExitStatus {
    fn exit_on_failure(self, msg: &str) {
        if !self.success() {
            eprintln!("{msg}");
            std::process::exit(self.code().unwrap_or(1));
        }
    }
}

fn confirm_input(stdin: std::io::Stdin, str: &str) -> bool {
    loop {
        eprint!("{str} [y/N]? ");
        std::io::stderr().flush().ok();

        let mut input = String::new();
        if let Err(e) = stdin.read_line(&mut input) {
            eprintln!("error:\n {e}");
        } else {
            let input = input.trim().to_lowercase();

            if input.is_empty() || input == "n" {
                return false;
            } else if input == "y" {
                return true;
            } else {
                eprintln!("invalid input");
            }
        }
    }
}
