//! 사용자가 직접 작성한 문서 파일이나 문자열을 언어별로 선택하는 절차적 매크로 라이브러리.
//!
//! 기본 방식인 [`include_str!`]는 `docs/guide.md`에서 `docs/guide.ko.md`를 선택한다.
//! [`include_str_from_dir!`]는 언어별로 지정한 디렉토리에서 같은 이름의 파일을 선택한다.
//! 두 파일 방식은 호출 패키지의 `Cargo.toml`을 경로 기준으로 사용한다.
//! [`localized_str!`]는 파일 없이 기본 문자열과 언어별 문자열 중 하나를 선택한다.
//! Rust 기본 매크로나 `doc` 속성의 의미를 바꾸지 않으며 자동 번역과 묵시적 언어 대체는 수행하지 않는다.
//!
//! 같은 패키지의 CLI는 기본 활성화된 `cli` feature로 제공된다. 매크로만 사용하는
//! 프로젝트는 `default-features = false`로 CLI 전용 의존성을 제외할 수 있다.

use proc_macro::TokenStream;
use quote::quote;
use syn::{LitStr, parse_macro_input};

/// 매크로 크레이트를 컴파일할 때 확정되는 문서 선택 언어.
///
/// `option_env!`가 rustc의 의존성 정보에 환경변수를 기록하므로 stable Rust에서도
/// 언어 변경과 변수 설정·해제 시 Cargo가 매크로 및 소비 크레이트를 다시 빌드한다.
/// 확장 함수 안에서 `std::env`로 읽으면 이 변경 추적을 얻을 수 없어 여기서 평가한다.
/// CLI가 자식 빌드에 전달하는 내부 규약이며 프로젝트별 독립 선택 값은 아니다.
const LANGUAGE: Option<&str> = option_env!("TEXTUS_LANG");

/// 기본 파일의 마지막 확장자 앞에 선택 언어를 붙여 사용자 작성 문서를 포함한다.
///
/// 문자열 리터럴 하나를 받는다. `"docs/api.guide.md"`는 언어 미선택 시 그대로,
/// 한국어 선택 시 `"docs/api.guide.ko.md"`가 된다. 경로는 호출 패키지 기준의
/// `/` 구분 상대 경로이며 파일명과 확장자가 필요하다. 절대 경로와 `..`는 거부한다.
///
/// `#[doc = cargo_textus::include_str!("docs/guide.md")]`로 API 설명에 붙이거나
/// 문자열 표현식으로 사용할 수 있다. 파일 내용 읽기와 변경 추적은 생성된 기본
/// `include_str!`에 맡긴다. 잘못된 경로나 언어 코드는 컴파일 오류이며, 선택한
/// 파일이 없어도 기본 파일로 대체하지 않고 rustc의 파일 읽기 오류를 발생시킨다.
#[proc_macro]
pub fn include_str(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as LitStr);
    match textus_core::i18n::document_path(&input.value(), LANGUAGE) {
        Ok(path) => {
            let selected = LitStr::new(&path, input.span());
            quote! {
                ::core::include_str!(::core::concat!(
                    ::core::env!("CARGO_MANIFEST_DIR"), "/", #selected
                ))
            }
            .into()
        }
        Err(message) => syn::Error::new(input.span(), message)
            .to_compile_error()
            .into(),
    }
}

mod directory;
mod localized;

/// 명시한 언어별 디렉토리에서 기본 파일과 이름이 같은 문서를 포함한다.
///
/// 기본 파일 뒤에 하나 이상의 `언어 = "디렉토리"` 매핑을 받는다. 키는 중복 없는
/// 소문자 ISO 639-1 코드이며 디렉토리 이름 자체가 언어 코드일 필요는 없다.
/// 모든 경로는 호출 패키지의 `Cargo.toml` 기준이다. 디렉토리 끝의 `/` 하나는 허용한다.
///
/// 아래 예시는 한국어 선택 시 `docs/ko/guide.md`, 영어 선택 시
/// `translations/english/guide.md`를 포함한다. 기본 경로의 상위 디렉토리는 붙이지 않는다.
/// 예시 파일은 소비 프로젝트가 직접 작성해야 하므로 이 코드 블록은 독립 실행하지 않는다.
///
/// ```ignore
/// #[doc = cargo_textus::include_str_from_dir!(
///     "docs/guide.md", ko = "docs/ko", en = "translations/english",
/// )]
/// pub fn example() {}
/// ```
///
/// 언어 미선택 시 기본 파일을 사용한다. 선택하지 않은 매핑도 코드·중복·경로를
/// 검증하지만 파일 존재 여부는 선택한 파일만 검사한다. 선택 언어의 매핑이나 파일이
/// 없으면 컴파일 오류이며 자동 대체하지 않는다. CLI로 선택하려면 해당 언어를
/// `package.metadata.textus.i18n.languages`에도 등록해야 한다.
#[proc_macro]
pub fn include_str_from_dir(input: TokenStream) -> TokenStream {
    directory::expand(input, LANGUAGE)
}

/// 기본 문서 문자열 또는 선택 언어에 연결된 문자열을 그대로 반환한다.
///
/// 기본 문자열 뒤에 하나 이상의 `언어 = "문서 내용"` 매핑을 받는다. 일반 문자열과
/// 여러 줄 raw 문자열 리터럴을 허용하며 빈 문자열과 공백도 내용으로 보존한다.
/// 변수·상수·`concat!` 등 리터럴 이외의 표현식은 받지 않는다. 파일 읽기는 수행하지 않는다.
///
/// ```
/// #[doc = cargo_textus::localized_str!(
///     "Default documentation.",
///     ko = "한국어 문서입니다.",
///     en = r#"English documentation with **Markdown**."#,
/// )]
/// pub fn example() {}
/// ```
///
/// 언어 미선택 시 기본 문자열을 사용하며 `#[doc = ...]`와 일반 `&'static str`
/// 표현식에서 모두 쓸 수 있다. 문서에 쓰면 선택한 문자열의 Markdown은 rustdoc이 처리한다.
/// 모든 키는 중복 없는 소문자 ISO 639-1 코드여야 한다. `as`처럼 Rust 예약어와 같은
/// 코드도 허용하며 raw 식별자 `r#as`와 `as`는 같은 코드다. 선택하지 않은 매핑도 검증한다.
///
/// 잘못된 코드·중복 키·빈 매핑 목록이나 선택 언어의 매핑 누락은 컴파일 오류이며
/// 기본 문자열로 자동 대체하지 않는다. CLI에서 선택할 언어는 별도로
/// `package.metadata.textus.i18n.languages`에 등록해야 하며 매크로가 자동 등록하지 않는다.
#[proc_macro]
pub fn localized_str(input: TokenStream) -> TokenStream {
    localized::expand(input, LANGUAGE)
}
