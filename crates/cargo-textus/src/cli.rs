//! 작업 중심 CLI 인자를 검증하고 실행 계층에 전달할 요청으로 변환한다.
//! 파일과 Cargo 프로세스에 접근하지 않아 구문 오류를 실행 전에 판별할 수 있다.

use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, bail};

/// 사용자가 문서에 수행할 작업. 언어와 렌더링 기능의 조합을 별도 명령으로 나누지 않는다.
/// `Open`만 생성 후 브라우저를 열고 나머지 작업은 브라우저 실행 없이 끝난다.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// 기본 또는 선택 언어 문서를 생성하고 자산 배치 후 브라우저로 연다.
    Open,
    /// 기본 또는 선택 언어 문서를 패키지의 렌더링 설정과 함께 생성한다.
    Build,
    /// 등록된 추가 언어만 출력한다. 기본 문서는 특정 언어 코드에 대응시키지 않는다.
    Languages,
    /// 언어 미지정 시 기본 문서와 모든 등록 언어를, 지정 시 해당 언어만 빌드 검사한다.
    Check,
}

/// 구문 검증을 마친 CLI 요청과 Cargo 공통 옵션을 보관한다.
///
/// 언어 코드의 유효성은 파싱 중 확인하지만 프로젝트 등록 여부는 설정 로딩 뒤 확인한다.
/// 이 구분으로 잘못된 코드와 아직 지원하지 않는 언어를 서로 다른 오류로 안내한다.
#[derive(Debug)]
pub struct Options {
    /// 실행할 작업. 옵션 조합의 허용 여부를 판단하는 기준이기도 하다.
    pub action: Action,
    /// 요청한 ISO 639-1 코드. 없으면 build/open은 기본 문서를, check는 전체 문서를 대상으로 한다.
    pub language: Option<String>,
    /// Cargo에 전달할 매니페스트 경로. 운영체제 경로를 보존하려고 UTF-8 문자열로 제한하지 않는다.
    pub manifest_path: Option<PathBuf>,
    /// 워크스페이스에서 선택할 패키지 이름. 생략하면 매니페스트 위치와 구성원 수로 결정한다.
    pub package: Option<String>,
    /// 네트워크 없이 실행하도록 metadata 조회와 빌드 등 모든 자식 Cargo 명령에 전달한다.
    pub offline: bool,
    /// 잠금 파일 변경을 금지하도록 모든 자식 Cargo 명령에 전달한다.
    pub locked: bool,
}

/// 인자 처리가 끝난 뒤 진입점이 수행할 동작.
///
/// 도움말을 일반 실행과 분리하여 프로젝트 설정이 없어도 사용법을 볼 수 있게 한다.
#[derive(Debug)]
pub enum Invocation {
    /// 인자가 없거나 도움말 플래그가 있으면 설정 로딩 없이 사용법을 출력한다.
    Help,
    /// 검증된 옵션으로 기능 실행 계층에 처리를 위임한다.
    Run(Options),
}

/// 실행 파일 이름을 제외한 `[textus] <작업> [옵션]`을 해석한다.
///
/// Cargo가 추가하는 `textus` 접두사를 허용하여 직접 실행과 같은 문법을 사용한다.
/// 인자가 없거나 도움말 플래그가 있으면 프로젝트를 읽지 않고 전체 도움말을 반환한다.
/// 언어는 build/open/check에서 선택 사항이고 languages에는 허용하지 않는다.
/// 구문과 코드 유효성을 여기서 검사하며 프로젝트의 언어 등록 여부는 실행 계층에서 확인한다.
/// 이전 i18n/render 계층과 --open은 실행하지 않고 새 명령으로의 변경 방법을 안내한다.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Invocation> {
    let mut args = args.into_iter().peekable();
    // Cargo 외부 명령 호출에서는 첫 인자로 하위 명령 이름이 전달된다.
    if args.peek().is_some_and(|arg| arg == "textus") {
        args.next();
    }

    let args: Vec<_> = args.collect();
    if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Ok(Invocation::Help);
    }

    let mut args = args.into_iter();
    let action = match args.next().as_deref().and_then(|arg| arg.to_str()) {
        Some("open") => Action::Open,
        Some("build") => Action::Build,
        Some("languages") => Action::Languages,
        Some("check") => Action::Check,
        Some("i18n" | "render") => bail!(
            "i18n/render commands were removed; use cargo textus build, open, check, or languages; see --help"
        ),
        Some("list") => bail!("list was replaced by cargo textus languages; see --help"),
        _ => bail!("expected build, open, check, or languages; see --help"),
    };
    let mut options = Options {
        action,
        language: None,
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
                    // 값이 있는 옵션을 조용히 덮어쓰지 않고 중복 입력을 알린다.
                    bail!("--lang may only be specified once");
                }
            }
            Some("--manifest-path") => {
                let value = args.next().context("--manifest-path requires a path")?;
                if options.manifest_path.is_none() {
                    options.manifest_path = Some(value.into());
                } else {
                    // 값이 있는 옵션을 조용히 덮어쓰지 않고 중복 입력을 알린다.
                    bail!("--manifest-path may only be specified once");
                }
            }
            Some("--package" | "-p") => {
                let value = string_value(&mut args, "--package")?;
                if options.package.is_none() {
                    options.package = Some(value);
                } else {
                    // 값이 있는 옵션을 조용히 덮어쓰지 않고 중복 입력을 알린다.
                    bail!("--package may only be specified once");
                }
            }
            Some("--open") => bail!("--open was removed; use cargo textus open instead"),
            Some("--offline") => options.offline = true,
            Some("--locked") => options.locked = true,
            _ => bail!("unknown argument {arg:?}; see --help"),
        }
    }
    if options.action == Action::Languages && options.language.is_some() {
        bail!("languages does not accept --lang; it lists registered languages");
    }

    if let Some(language) = &options.language {
        textus_core::i18n::validate_language(language).map_err(anyhow::Error::msg)?;
    }

    Ok(Invocation::Run(options))
}

/// 다음 인자를 UTF-8 옵션 값으로 소비하며 누락과 인코딩 오류에 플래그 이름을 붙인다.
///
/// 언어 코드와 패키지 이름처럼 문자열 비교가 필요한 값에 사용한다.
/// 파일 경로는 이 함수를 거치지 않고 `OsString`에서 `PathBuf`로 변환한다.
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
    fn accepts_task_commands_with_optional_language() {
        for prefix in ["", "textus "] {
            for (command, expected) in [
                ("build", Action::Build),
                ("open", Action::Open),
                ("check", Action::Check),
            ] {
                let Invocation::Run(options) = parse_words(&format!(
                    "{prefix}{command} --lang ko -p demo --offline --locked"
                ))
                .unwrap() else {
                    panic!("expected command");
                };
                assert_eq!(options.action, expected);
                assert_eq!(options.language.as_deref(), Some("ko"));
                assert_eq!(options.package.as_deref(), Some("demo"));
                assert!(options.offline && options.locked);
                let Invocation::Run(options) = parse_words(&format!("{prefix}{command}")).unwrap()
                else {
                    panic!("expected command");
                };
                assert!(options.language.is_none());
            }
            assert!(matches!(
                parse_words(&format!("{prefix}languages")).unwrap(),
                Invocation::Run(Options {
                    action: Action::Languages,
                    ..
                })
            ));
        }
    }

    #[test]
    fn rejects_removed_commands_and_invalid_options() {
        for words in [
            "i18n open --lang ko",
            "render build",
            "list",
            "build --open",
            "open --open",
            "build --lang zz",
            "build --lang ko-KR",
            "build --lang",
            "languages --lang ko",
            "check --unknown",
            "check --lang ko --lang en",
            "build -p a -p b",
            "build --manifest-path a --manifest-path b",
            "build --manifest-path",
            "build -p",
            "doc",
        ] {
            assert!(parse_words(words).is_err(), "{words}");
        }
        assert!(
            parse_words("build --open")
                .unwrap_err()
                .to_string()
                .contains("cargo textus open")
        );
    }

    #[test]
    fn help_does_not_require_a_project_or_valid_command() {
        for words in ["", "textus", "--help", "build -h", "i18n --help"] {
            assert!(matches!(parse_words(words).unwrap(), Invocation::Help));
        }
    }
}
