use std::process::Command;

use crate::cli::Options;

pub fn command(options: &Options) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    if options.offline {
        command.arg("--offline");
    }
    if options.locked {
        command.arg("--locked");
    }
    command
}
