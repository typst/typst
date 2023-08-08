use std::env;
use std::fs::{create_dir_all, File};
use std::path::Path;
use std::process::Command;

use clap::{CommandFactory, ValueEnum};
use clap_complete::{generate_to, Shell};
use clap_mangen::Man;

#[path = "src/args.rs"]
#[allow(dead_code)]
mod args;

fn main() {
    // https://stackoverflow.com/a/51311222/11494565
    println!("cargo:rustc-env=TARGET={}", env::var("TARGET").unwrap());
    println!("cargo:rerun-if-env-changed=TYPST_VERSION");
    println!("cargo:rerun-if-env-changed=GEN_ARTIFACTS");

    if option_env!("TYPST_VERSION").is_none() {
        println!("cargo:rustc-env=TYPST_VERSION={}", typst_version());
    }

    if let Some(dir) = env::var_os("GEN_ARTIFACTS") {
        let out = &Path::new(&dir);
        create_dir_all(out).unwrap();
        let cmd = &mut args::CliArguments::command();

        Man::new(cmd.clone())
            .render(&mut File::create(out.join("typst.1")).unwrap())
            .unwrap();

        for subcmd in cmd.get_subcommands() {
            let name = format!("typst-{}", subcmd.get_name());
            Man::new(subcmd.clone().name(&name))
                .render(&mut File::create(out.join(format!("{name}.1"))).unwrap())
                .unwrap();
        }

        for shell in Shell::value_variants() {
            generate_to(*shell, cmd, "typst", out).unwrap();
        }
    }
}

/// Also used by `args.rs`.
fn typst_version() -> String {
    if let Some(version) = option_env!("TYPST_VERSION") {
        return version.to_owned();
    }

    let pkg = env!("CARGO_PKG_VERSION");
    let hash = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout.get(..8)?.into()).ok())
        .unwrap_or_else(|| "unknown hash".into());

    format!("{pkg} ({hash})")
}
