use std::io::stdout;

use clap::CommandFactory;
use clap_complete::generate;

use crate::args::{CliArguments, CompletionsCommand};

/// Execute the completions command.
pub fn completions(command: &CompletionsCommand) {
    let mut cmd = CliArguments::command();
    let bin_name = cmd.get_name().to_string();
    generate(command.shell, &mut cmd, bin_name, &mut stdout());
}
