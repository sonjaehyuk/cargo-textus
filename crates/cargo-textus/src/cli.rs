//! i18n과 전체 문서 render의 CLI 인자를 검증하고 실행 계층에 전달할 요청으로 변환한다.
//! 파일과 Cargo 프로세스에 접근하지 않아 구문 오류를 실행 전에 판별할 수 있다.

use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result, bail};

/// CLI 기능이 수행할 작업을 구분한다. render는 빌드와 열기만 허용한다.
///
/// 브라우저 실행 여부와 대상 언어는 `Options`에 따로 보관해
/// `build --open`과 `open`이 같은 빌드 경로를 사용하게 한다.
#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    /// 선택한 언어의 API 문서를 빌드한 뒤 Cargo를 통해 브라우저로 연다.
    Open,
    /// 선택한 언어의 API 문서를 생성한다. 브라우저 실행은 별도 옵션으로 결정한다.
    Build,
    /// 설정에 등록된 언어를 출력한다. 문서 파일의 존재 여부는 검사하지 않는다.
    List,
    /// 지정한 언어 또는 모든 등록 언어로 rustdoc을 실행해 문서 포함을 검사한다.
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
    /// 요청한 ISO 639-1 코드. `check`에서 없으면 모든 등록 언어를 검사한다.
    pub language: Option<String>,
    /// `build --open` 요청 여부. `open` 작업 자체의 브라우저 실행과는 별도로 기록한다.
    pub open: bool,
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
    /// 전체 문서 렌더링을 설정한 rustdoc 빌드. i18n 지원 선언 없이 실행할 수 있다.
    Render(Options),
}

/// 실행 파일 이름을 제외한 `[textus] <i18n|render> <작업> [옵션]`을 해석한다.
///
/// Cargo가 추가하는 `textus` 접두사를 허용해 Cargo 외부 명령과 바이너리 직접 실행을
/// 같이 지원한다. 인자가 없거나 어느 위치에든 `--help` 또는 `-h`가 있으면 도움말을 반환한다.
///
/// i18n의 `build`와 `open`에는 `--lang`이 필요하고, `list`에는 허용하지 않는다.
/// render는 `build`와 `open`만 지원하며 `--lang`은 선택적 i18n 조합에 사용한다.
/// `--open`은 `build` 전용이다. 값을 받는 옵션의 중복, 값 누락, 알 수 없는 인자,
/// 유효하지 않은 언어 코드는 오류로 반환한다. 값은 옵션과 공백으로 구분해야 한다.
/// 프로젝트의 언어 등록 여부나 파일 존재 여부는 이 단계에서 확인하지 않는다.
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
    let render = match args.next().as_deref().and_then(|arg| arg.to_str()) {
        Some("i18n") => false,
        Some("render") => true,
        _ => bail!("expected the i18n or render subcommand; see --help"),
    };

    let action = match args.next().as_deref().and_then(|arg| arg.to_str()) {
        Some("open") => Action::Open,
        Some("build") => Action::Build,
        Some("list") => Action::List,
        Some("check") => Action::Check,
        _ if render => bail!("expected build or open after render; see --help"),
        _ => bail!("expected open, build, list, or check after i18n; see --help"),
    };
    if render && !matches!(action, Action::Build | Action::Open) {
        bail!("render supports build and open only");
    }
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
            Some("--open") => options.open = true,
            Some("--offline") => options.offline = true,
            Some("--locked") => options.locked = true,
            _ => bail!("unknown argument {arg:?}; see --help"),
        }
    }
    // 프로젝트를 읽기 전에 작업과 옵션의 잘못된 조합을 거부한다.
    if options.open && options.action != Action::Build {
        bail!("--open is only valid with build; open already opens documentation");
    }
    if options.action == Action::List && options.language.is_some() {
        bail!("i18n list does not accept --lang");
    }
    if !render
        && matches!(options.action, Action::Build | Action::Open)
        && options.language.is_none()
    {
        bail!("--lang is required for i18n build and i18n open");
    }

    if let Some(language) = &options.language {
        textus_core::i18n::validate_language(language).map_err(anyhow::Error::msg)?;
    }

    Ok(if render {
        Invocation::Render(options)
    } else {
        Invocation::Run(options)
    })
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
    fn render_works_without_language_and_rejects_i18n_only_actions() {
        for command in [
            "render build",
            "textus render open",
            "render build --open --lang ko",
        ] {
            assert!(matches!(
                parse_words(command).unwrap(),
                Invocation::Render(_)
            ));
        }
        for command in ["render list", "render check", "render build --lang zz"] {
            assert!(parse_words(command).is_err());
        }
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
