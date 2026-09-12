use anyhow::{bail, Context, Result};

use crate::{
    cargo,
    cli::{Action, Options},
    config::{self, Project},
};

pub fn run(options: Options) -> Result<()> {
    let project = config::load(&options)?;
    if let Some(language) = &options.language {
        project.config.require_language(language)?;
    }
    match options.action {
        Action::List => {
            for language in &project.config.languages {
                println!("{language}");
            }
        }
        Action::Check => {
            let languages = options
                .language
                .as_ref()
                .map(std::slice::from_ref)
                .unwrap_or(&project.config.languages);
            for language in languages {
                build(&project, &options, language, false)?;
            }
            println!(
                "Checked {} language(s) for {}",
                languages.len(),
                project.name
            );
        }
        Action::Build | Action::Open => {
            let language = options.language.as_deref().context("--lang is required")?;
            build(
                &project,
                &options,
                language,
                options.open || options.action == Action::Open,
            )?;
        }
    }
    Ok(())
}

fn build(project: &Project, options: &Options, language: &str, open: bool) -> Result<()> {
    let target = project
        .target_directory
        .join("textus")
        .join(language)
        .join(&project.name);
    // Cargo's rustdoc freshness check can miss deleted included files. Rebuild
    // only our generated documentation, using Cargo's public interface rather
    // than touching source files or manipulating internal fingerprint files.
    // Package isolation also preserves other packages' localized documents.
    let status = cargo::command(options)
        .args(["clean", "--doc", "--manifest-path"])
        .arg(&project.manifest_path)
        .arg("--target-dir")
        .arg(&target)
        .status()
        .context("could not clean textus documentation output")?;
    if !status.success() {
        bail!("could not clean textus documentation output ({status})");
    }
    let mut command = cargo::command(options);
    command
        .args(["doc", "--no-deps", "--manifest-path"])
        .arg(&project.manifest_path)
        .arg("--package")
        .arg(&project.name)
        .arg("--target-dir")
        .arg(&target)
        // Only the child process receives the selection; no global env mutation.
        .env("TEXTUS_LANG", language);
    if open {
        // Delegate browser selection and platform behavior to Cargo.
        command.arg("--open");
    }
    eprintln!("Building {} documentation ({language})", project.name);
    let status = command.status().context("could not run cargo doc")?;
    if !status.success() {
        bail!("cargo doc failed for language {language:?} ({status})");
    }
    Ok(())
}
