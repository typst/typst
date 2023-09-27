use clap::Parser;
use ecow::EcoString;
use typst_format::{format, Command};

fn main() -> Result<(), EcoString> {
    format(&Command::parse())?;
    Ok(())
}
