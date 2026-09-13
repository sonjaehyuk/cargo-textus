//! 작업 명령을 공통 문서 생성 경로로 연결하는 실행 계층.
//! 언어는 입력 문서 선택이고 렌더링은 프로젝트 설정이므로 두 기능을 명령으로 분리하지 않는다.

use anyhow::{Context, Result};

use crate::{
    cli::{Action, Options},
    config, render,
};

/// 패키지를 한 번 읽고 작업의 대상 문서를 결정한다.
/// languages는 등록 순서의 코드만 출력하며 미등록 프로젝트에서는 빈 결과로 성공한다.
/// check는 기본 문서부터 등록 순서대로 검사하고 첫 실패에서 중단한다. 기본 문서 검사를
/// 포함하므로 번역 파일만 정상인 프로젝트도 오류를 발견할 수 있다.
pub fn run(options: Options) -> Result<()> {
    let project = config::load(&options)?;
    let i18n = project.i18n_config()?;
    if let Some(language) = &options.language {
        i18n.as_ref().context("no languages registered; configure [package.metadata.textus.i18n] languages before using --lang")?
            .require_language(language)?;
    }
    if options.action == Action::Languages {
        if let Some(config) = i18n {
            for language in config.languages {
                println!("{language}");
            }
        }
        return Ok(());
    }
    let mut languages = vec![options.language.as_deref()];
    if options.action == Action::Check
        && options.language.is_none()
        && let Some(config) = &i18n
    {
        languages.extend(
            config
                .languages
                .iter()
                .map(|language| Some(language.as_str())),
        );
    }
    for language in &languages {
        render::build(
            &project,
            &options,
            *language,
            options.action == Action::Open,
        )?;
    }
    if options.action == Action::Check {
        println!(
            "Checked {} documentation variant(s) for {} (rustdoc build only)",
            languages.len(),
            project.name
        );
    }
    Ok(())
}
