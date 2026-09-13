use std::{collections::BTreeSet, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{cargo, cli::Options};

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct I18nConfig {
    pub languages: Vec<String>,
}

pub struct Project {
    pub name: String,
    pub manifest_path: PathBuf,
    pub target_directory: PathBuf,
    pub config: I18nConfig,
}

impl I18nConfig {
    /// Validates the i18n (internationalization) configuration.
    ///
    /// This method ensures that the language configuration is valid by performing
    /// the following checks:
    ///
    /// ## Validation Rules
    ///
    /// 1. **Non-empty languages**: The languages list must contain at least one language
    /// 2. **Valid language codes**: Each language must pass `textus_core::i18n::validate_language`
    /// 3. **No duplicates**: Each language can only appear once in the configuration
    ///
    /// ## Returns
    ///
    /// * `Ok(())` - If all validation checks pass
    /// * `Err` - If any validation rule is violated, with a descriptive error message
    ///
    /// ## Errors
    ///
    /// This function will return an error if:
    ///
    /// * The `languages` collection is empty
    /// * Any language code fails validation according to `textus_core::i18n::validate_language`
    /// * A duplicate language code is detected in the configuration
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let config = I18nConfig {
    ///     languages: vec!["en".to_string(), "fr".to_string()],
    /// };
    /// config.validate()?; // Ok
    ///
    /// let invalid = I18nConfig {
    ///     languages: vec![],
    /// };
    /// invalid.validate()?; // Error: languages must not be empty
    /// ```
    fn validate(&self) -> Result<()> {
        if self.languages.is_empty() {
            bail!("textus i18n languages must not be empty");
        }
        let mut seen = BTreeSet::new();
        for language in &self.languages {
            textus_core::i18n::validate_language(language).map_err(anyhow::Error::msg)?;
            if !seen.insert(language) {
                bail!("duplicate language {language:?} in textus i18n configuration");
            }
        }
        Ok(())
    }

    /// Verifies that a language is registered in the available languages list.
    ///
    /// ## Arguments
    ///
    /// * `language` - The language identifier to check for registration
    ///
    /// ## Returns
    ///
    /// * `Ok(())` - If the language is found in the registered languages
    /// * `Err` - If the language is not registered, with an error message listing all available languages
    ///
    /// ## Errors
    ///
    /// Returns an error if the specified language is not found in `self.languages`,
    /// including a message that shows the requested language and all available options.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// // Assuming "ko" is registered
    /// manager.require_language("ko")?; // Ok(())
    ///
    /// // Assuming "unknown" is not registered
    /// manager.require_language("unknown")?; // Err: language "unknown" is not registered
    /// ```
    pub fn require_language(&self, language: &str) -> Result<()> {
        if !self.languages.iter().any(|entry| entry == language) {
            bail!(
                "language {language:?} is not registered; available languages: {}",
                self.languages.join(", ")
            );
        }
        Ok(())
    }
}

pub fn load(options: &Options) -> Result<Project> {
    let mut command = cargo::command(options);
    command.args(["metadata", "--format-version", "1", "--no-deps"]);
    if let Some(path) = &options.manifest_path {
        command.arg("--manifest-path").arg(path);
    }
    let output = command.output().context("could not run cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .context("could not decode cargo metadata --format-version 1")?;
    let members: Vec<_> = metadata
        .packages
        .into_iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .collect();
    let package = select_package(&members, options)?;
    let value = package
        .metadata
        .pointer("/textus/i18n")
        .context("missing [package.metadata.textus.i18n]; set languages = [\"en\", \"ko\"]")?;
    let config: I18nConfig = serde_json::from_value(value.clone())
        .context("invalid [package.metadata.textus.i18n] configuration")?;
    config.validate()?;
    Ok(Project {
        name: package.name.clone(),
        manifest_path: package.manifest_path.clone(),
        target_directory: metadata.target_directory,
        config,
    })
}

fn select_package<'a>(members: &'a [Package], options: &Options) -> Result<&'a Package> {
    if let Some(name) = &options.package {
        return members
            .iter()
            .find(|package| &package.name == name)
            .with_context(|| format!("workspace package {name:?} not found"));
    }
    let manifest = match &options.manifest_path {
        Some(path) => path.clone(),
        None => {
            let current =
                std::env::current_dir().context("could not determine current directory")?;
            current
                .ancestors()
                .map(|path| path.join("Cargo.toml"))
                .find(|path| path.is_file())
                .context("could not locate Cargo.toml")?
        }
    };
    let manifest = manifest
        .canonicalize()
        .context("could not resolve Cargo.toml path")?;
    if let Some(package) = members
        .iter()
        .find(|package| package.manifest_path.canonicalize().ok().as_ref() == Some(&manifest))
    {
        return Ok(package);
    }
    if let [package] = members {
        return Ok(package);
    }
    bail!(
        "select one workspace package with --package; available packages: {}",
        members
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_rejects_invalid_duplicate_and_empty_languages() {
        for languages in [vec![], vec!["ko", "ko"], vec!["zz"], vec!["ko-KR"]] {
            let config = I18nConfig {
                languages: languages.into_iter().map(String::from).collect(),
            };
            assert!(config.validate().is_err());
        }
        let config = I18nConfig {
            languages: vec!["en".into(), "ko".into()],
        };
        config.validate().unwrap();
        config.require_language("ko").unwrap();
        assert!(config.require_language("ja").is_err());
    }
}
