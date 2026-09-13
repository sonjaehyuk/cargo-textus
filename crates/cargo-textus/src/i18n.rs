//! 등록된 언어를 대상으로 rustdoc 생성과 검사를 수행하는 CLI 기능 계층.
//! 매크로의 파일 선택은 빌드에 전달한 언어로 이루어지며 이 모듈은 원본 문서를 수정하지 않는다.

use anyhow::{Context, Result, bail};

use crate::{
    cargo,
    cli::{Action, Options},
    config::{self, Project},
};

/// 설정을 읽고 요청 언어의 등록 여부를 확인한 뒤 작업을 수행한다.
///
/// `list`는 선언만 출력한다. `check`는 지정 언어 또는 등록 순서대로 모든 언어를
/// 실제로 빌드하며 첫 실패에서 멈춘다. 검사도 산출물을 만들지만 doctest나 번역
/// 완전성 검사는 수행하지 않는다. `open`과 `build --open`은 같은 빌드 경로를 사용한다.
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

/// 한 패키지의 한 언어에 대한 API 문서를 격리된 산출물 경로에 생성한다.
///
/// 대상은 `<target>/textus/<언어>/<패키지>`이며 Cargo의 타깃 설정에 따라 그 아래
/// 추가 경로가 생길 수 있다. 삭제된 포함 파일을 rustdoc 캐시가 놓치지 않도록
/// 해당 경로의 문서 산출물을 먼저 Cargo로 정리한다. 원본과 다른 언어의 결과는 보존한다.
///
/// 언어는 자식 `cargo doc --no-deps`에만 전달하며 의존성 API 문서는 생성하지 않는다.
/// `open`이 참이면 브라우저 선택도 Cargo에 맡긴다. 정리나 빌드의 실행·종료 실패를
/// 오류로 반환하며 정리에 실패하면 빌드를 진행하지 않는다.
fn build(project: &Project, options: &Options, language: &str, open: bool) -> Result<()> {
    let target = project
        .target_directory
        .join("textus")
        .join(language)
        .join(&project.name);
    // 삭제된 포함 파일도 재검사하도록 이 패키지·언어의 문서 산출물만 정리한다.
    // Cargo의 공개 명령을 사용해 내부 캐시 형식에 의존하지 않는다.
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
        // 부모 프로세스의 환경은 바꾸지 않고 자식 빌드에만 언어를 전달한다.
        .env("TEXTUS_LANG", language);
    if open {
        // 플랫폼별 브라우저 실행과 사용자 설정 해석은 Cargo에 위임한다.
        command.arg("--open");
    }
    eprintln!("Building {} documentation ({language})", project.name);
    let status = command.status().context("could not run cargo doc")?;
    if !status.success() {
        bail!("cargo doc failed for language {language:?} ({status})");
    }
    Ok(())
}
