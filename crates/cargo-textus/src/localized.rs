//! 직접 작성한 문서 문자열의 구문을 해석하고 선택 결과를 문자열 리터럴로 확장한다.
//! 언어 검증과 선택은 textus-core에 맡기며 파일 경로나 호출 패키지 환경을 읽지 않는다.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// 문자열 내용과 위치를 함께 보존해 리터럴만 허용하고 오류를 호출 지점에 연결한다.
struct LocalizedInput {
    /// 언어 미선택 시 사용할 문서. 빈 문자열도 명시적으로 작성한 내용으로 취급한다.
    default: LitStr,
    /// 중복을 소거하지 않은 언어 키와 문서 리터럴 목록. 의미 검증 전에 정렬하지 않는다.
    translations: Vec<(Ident, LitStr)>,
}

impl Parse for LocalizedInput {
    /// 기본 문자열 뒤의 `언어 = 문자열` 목록을 읽고 마지막 쉼표를 허용한다.
    ///
    /// ISO 코드 `as`는 Rust 예약어이므로 일반 식별자 파서 대신 parse_any를 쓴다.
    /// 실제 배정 코드인지와 매핑이 하나 이상 있는지는 토큰 처리 후 코어에서 검증한다.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let default = input.parse()?;
        let mut translations = Vec::new();
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let code = input.call(Ident::parse_any)?;
            input.parse::<Token![=]>()?;
            translations.push((code, input.parse()?));
        }
        Ok(Self {
            default,
            translations,
        })
    }
}

/// 추적된 LANGUAGE 값으로 내용을 선택해 doc 속성과 문자열 표현식에 쓸 리터럴을 만든다.
///
/// 문자열을 경로나 Markdown으로 해석하지 않는다. 코어의 검증 오류는 기본 문자열에
/// 연결하고, 토큰 구문 오류는 파서가 잘못된 입력 위치에 표시한다.
pub(crate) fn expand(input: TokenStream, language: Option<&str>) -> TokenStream {
    let input = parse_macro_input!(input as LocalizedInput);
    let default = input.default.value();
    let owned: Vec<_> = input
        .translations
        .iter()
        .map(|(code, text)| (code.unraw().to_string(), text.value()))
        .collect();
    let translations: Vec<_> = owned
        .iter()
        .map(|(code, text)| (code.as_str(), text.as_str()))
        .collect();
    match textus_core::i18n::localized_document(&default, &translations, language) {
        Ok(text) => {
            let selected = LitStr::new(text, input.default.span());
            quote! { #selected }.into()
        }
        Err(message) => syn::Error::new(input.default.span(), message)
            .to_compile_error()
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_literals_keyword_codes_and_trailing_comma() {
        let input = syn::parse_str::<LocalizedInput>(
            r##""default\n", ko = r#"여러 줄
"인용"과 \ 경로"#, as = "অসমীয়া", r#en = "","##,
        )
        .unwrap();
        assert_eq!(input.default.value(), "default\n");
        assert_eq!(
            input.translations[0].1.value(),
            "여러 줄\n\"인용\"과 \\ 경로"
        );
        assert_eq!(input.translations[1].0.unraw().to_string(), "as");
        assert_eq!(input.translations[2].0.unraw().to_string(), "en");
        assert_eq!(input.translations[2].1.value(), "");
    }

    #[test]
    fn rejects_expressions_and_malformed_mappings() {
        for input in [
            "",
            r#"123, ko = "한국어""#,
            r#"b"bytes", ko = "한국어""#,
            r#"DEFAULT, ko = "한국어""#,
            r#""default", ko = concat!("한", "국어")"#,
            r#""default", ko = DOCUMENT"#,
            r#""default", "ko" = "한국어""#,
            r#""default", ko = 1"#,
            r#""default", ko"#,
            r#""default", ko = "한국어" en = "English""#,
        ] {
            assert!(syn::parse_str::<LocalizedInput>(input).is_err(), "{input}");
        }
    }
}
