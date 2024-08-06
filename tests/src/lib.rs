use args::CliArguments;
use clap::Parser;
use once_cell::sync::Lazy;

pub mod args;
pub mod collect;
pub mod constants;
pub mod custom;
pub mod logger;
pub mod run;
pub mod world;

/// The parsed command line arguments.
pub static ARGS: Lazy<CliArguments> = Lazy::new(CliArguments::parse);
