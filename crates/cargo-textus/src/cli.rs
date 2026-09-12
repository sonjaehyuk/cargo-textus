use std::{ffi::OsString, path::PathBuf};

use anyhow::{bail, Context, Result};

pub const HELP: &str = "cargo-textus: user-authored localized Rust documentation

Usage: cargo textus i18n <COMMAND> [OPTIONS]
       cargo-textus i18n <COMMAND> [OPTIONS]

Commands:
  open  --lang CODE           Build documentation and open it in a browser
  build --lang CODE [--open]  Build documentation, optionally opening it
  list                       List languages registered in package metadata
  check [--lang CODE]         Run rustdoc for one or all registered languages

Options:
  --manifest-path PATH  Path to Cargo.toml
  --package, -p NAME    Select one workspace package
  --offline            Run Cargo without network access
  --locked             Require the existing Cargo.lock to remain unchanged
  --help, -h           Show this help

Language codes must be assigned lowercase ISO 639-1 codes, such as ko or en.
Configuration: [package.metadata.textus.i18n] languages = [\"en\", \"ko\"]";

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Open,
    Build,
    List,
    Check,
}

#[derive(Debug)]
pub struct Options {
    pub action: Action,
    pub language: Option<String>,
    pub open: bool,
    pub manifest_path: Option<PathBuf>,
    pub package: Option<String>,
    pub offline: bool,
    pub locked: bool,
}

#[derive(Debug)]
pub enum Invocation {
    Help,
    Run(Options),
}

pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Invocation> {
    let mut args = args.into_iter().peekable();
    // Cargo passes the external subcommand name as the first argument.
    if args.peek().is_some_and(|arg| arg == "textus") {
        args.next();
    }
    let args: Vec<_> = args.collect();
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Invocation::Help);
    }
    let mut args = args.into_iter();
    if args.next().as_deref() != Some(std::ffi::OsStr::new("i18n")) {
        bail!("expected the i18n subcommand; see --help");
    }
    let action = match args.next().as_deref().and_then(|arg| arg.to_str()) {
        Some("open") => Action::Open,
        Some("build") => Action::Build,
        Some("list") => Action::List,
        Some("check") => Action::Check,
        _ => bail!("expected open, build, list, or check after i18n; see --help"),
    };
    let mut options = Options {
        action,
        language: None,
        open: false,
        manifest_path: None,
        package: None,
        offline: false,
        locked: false,
    };
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--lang") => {
                let value = string_value(&mut args, "--lang")?;
                if options.language.replace(value).is_some() {
                    bail!("--lang may only be specified once");
                }
            }
            Some("--manifest-path") => {
                let value = args.next().context("--manifest-path requires a path")?;
                if options.manifest_path.replace(value.into()).is_some() {
                    bail!("--manifest-path may only be specified once");
                }
            }
            Some("--package" | "-p") => {
                let value = string_value(&mut args, "--package")?;
                if options.package.replace(value).is_some() {
                    bail!("--package may only be specified once");
                }
            }
            Some("--open") => options.open = true,
            Some("--offline") => options.offline = true,
            Some("--locked") => options.locked = true,
            _ => bail!("unknown argument {arg:?}; see --help"),
        }
    }
    if options.open && options.action != Action::Build {
        bail!("--open is only valid with i18n build; i18n open already opens documentation");
    }
    if options.action == Action::List && options.language.is_some() {
        bail!("i18n list does not accept --lang");
    }
    if matches!(options.action, Action::Build | Action::Open) && options.language.is_none() {
        bail!("--lang is required for i18n build and i18n open");
    }
    if let Some(language) = &options.language {
        textus_core::i18n::validate_language(language).map_err(anyhow::Error::msg)?;
    }
    Ok(Invocation::Run(options))
}

fn string_value(args: &mut impl Iterator<Item = OsString>, flag: &str) -> Result<String> {
    args.next()
        .with_context(|| format!("{flag} requires a value"))?
        .into_string()
        .map_err(|_| anyhow::anyhow!("{flag} requires a UTF-8 value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(words: &str) -> Result<Invocation> {
        parse(words.split_whitespace().map(OsString::from))
    }

    #[test]
    fn accepts_cargo_and_direct_invocations() {
        for prefix in ["", "textus "] {
            let Invocation::Run(options) =
                parse_words(&format!("{prefix}i18n build --lang ko --open -p demo")).unwrap()
            else {
                panic!("expected a command");
            };
            assert_eq!(options.action, Action::Build);
            assert_eq!(options.language.as_deref(), Some("ko"));
            assert!(options.open);
            assert_eq!(options.package.as_deref(), Some("demo"));
        }
    }

    #[test]
    fn rejects_invalid_commands_and_options() {
        for words in [
            "doc open",
            "i18n",
            "i18n open",
            "i18n build --lang zz",
            "i18n build --lang ko-KR",
            "i18n build --lang",
            "i18n list --lang ko",
            "i18n check --open",
            "i18n check --unknown",
            "i18n check --lang ko --lang en",
        ] {
            assert!(parse_words(words).is_err(), "{words}");
        }
    }
}
