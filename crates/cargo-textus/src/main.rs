mod cargo;
mod cli;
mod config;
mod i18n;

fn main() -> std::process::ExitCode {
    match cli::parse(std::env::args_os().skip(1)).and_then(|invocation| match invocation {
        cli::Invocation::Help => {
            println!(include_str!("help.txt"));
            Ok(())
        }
        cli::Invocation::Run(options) => i18n::run(options),
    }) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
