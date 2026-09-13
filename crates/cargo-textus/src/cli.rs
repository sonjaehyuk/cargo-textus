use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, bail};

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

/// Parses command-line arguments into an `Invocation` for the textus i18n tool.
///
/// This function processes command-line arguments for a Cargo subcommand that handles
/// internationalization (i18n) operations. It expects arguments in the format:
/// `[textus] i18n <action> [options]`
///
/// ## Arguments
///
/// * `args` - An iterator of `OsString` values representing command-line arguments.
///   When invoked as a Cargo subcommand, Cargo passes the subcommand name
///   ("textus") as the first argument, which is automatically stripped.
///
/// ## Supported Actions
///
/// * `open` - Opens i18n documentation (**requires** `--lang`)
/// * `build` - Builds i18n resources (**requires** `--lang`)
/// * `list` - Lists available languages (**does not accept** `--lang`)
/// * `check` - Checks i18n configuration
///
/// ## Supported Options
///
/// * `--lang <LANGUAGE>` - Specifies the target language (required for `open` and `build`). It must be a lowercase valid ISO 639-1 code.
/// * `--manifest-path <PATH>` - Path to the Cargo.toml file
/// * `--package`, `-p <PACKAGE>` - Specific package to operate on
/// * `--open` - Opens documentation after building (only valid with `build` action)
/// * `--offline` - Run without accessing the network
/// * `--locked` - Require Cargo.lock to be up to date
/// * `--help`, `-h` - Display help information
///
/// ## Returns
///
/// * `Ok(Invocation::Help)` - If help is requested or no arguments are provided
/// * `Ok(Invocation::Run(options))` - If valid arguments are parsed
/// * `Err` - If invalid arguments, combinations, or missing required options are detected
///
/// ## Errors
///
/// This function returns an error if:
/// * The `i18n` subcommand is not specified
/// * An unknown action is provided
/// * An unknown argument is encountered
/// * Required options are missing (e.g., `--lang` for `build`/`open`)
/// * Incompatible options are combined (e.g., `--open` with `list`)
/// * An option is specified multiple times when only one value is allowed
/// * The language code fails validation
///
/// ## Examples
///
/// ```no_run
/// # use std::ffi::OsString;
/// let args = vec![
///     OsString::from("textus"),
///     OsString::from("i18n"),
///     OsString::from("build"),
///     OsString::from("--lang"),
///     OsString::from("es"),
/// ];
/// let invocation = parse(args)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
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

    // NOTE: Change the if statement below when add new features.
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
                if options.language.is_none() {
                    options.language = Some(value);
                } else {
                    // Already Given.
                    bail!("--lang may only be specified once");
                }
            }
            Some("--manifest-path") => {
                let value = args.next().context("--manifest-path requires a path")?;
                if options.manifest_path.is_none() {
                    options.manifest_path = Some(value.into());
                } else {
                    // Already Given.
                    bail!("--manifest-path may only be specified once");
                }
            }
            Some("--package" | "-p") => {
                let value = string_value(&mut args, "--package")?;
                if options.package.is_none() {
                    options.package = Some(value);
                } else {
                    // Already Given.
                    bail!("--package may only be specified once");
                }
            }
            Some("--open") => options.open = true,
            Some("--offline") => options.offline = true,
            Some("--locked") => options.locked = true,
            _ => bail!("unknown argument {arg:?}; see --help"),
        }
    }
    // Check the invalid combines with options and an action.
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

/// Extracts and validates the next command-line argument as a UTF-8 string.
///
/// ## Arguments
///
/// * `args` - A mutable iterator over command-line arguments as `OsString`s
/// * `flag` - The name of the flag being processed (used for error messages)
///
/// ## Returns
///
/// * `Ok(String)` - The next argument successfully converted to a UTF-8 string
/// * `Err` - If no argument is available or if the argument contains invalid UTF-8
///
/// ## Errors
///
/// This function will return an error if:
/// * The iterator is exhausted (no value provided after the flag)
/// * The `OsString` cannot be converted to valid UTF-8
///
/// ## Examples
///
/// ```no_run
/// use std::ffi::OsString;
///
/// let mut args = vec![OsString::from("value1")].into_iter();
/// let result = string_value(&mut args, "--output");
/// assert_eq!(result.unwrap(), "value1");
/// ```
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
