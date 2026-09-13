//! 디렉토리 매크로의 구문을 해석하고 검증된 경로를 Rust 기본 매크로로 확장한다.
//! 언어와 경로의 의미 검증은 textus-core에 두어 컴파일러 토큰 없이도 검사할 수 있게 한다.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// 기본 파일 리터럴과 `ko = "docs/ko"` 형태의 명시적 매핑을 보관한다.
///
/// 리터럴의 위치 정보를 유지해 확장 오류를 호출 지점에 표시한다.
/// 목록을 그대로 보존하므로 중복 키도 의미 검증 단계에서 발견할 수 있다.
struct DirectoryInput {
    /// 언어 미선택 시 포함할 기본 파일. 번역 파일을 선택할 때는 여기서 파일명만 가져온다.
    path: LitStr,
    /// 언어 식별자와 디렉토리 리터럴의 입력 순서 목록. 구문 해석만으로 유효성을 보장하지 않는다.
    directories: Vec<(Ident, LitStr)>,
}

impl Parse for DirectoryInput {
    /// 첫 문자열 뒤에 쉼표로 구분한 `언어 = 디렉토리 문자열`을 읽는다.
    ///
    /// 마지막 쉼표를 허용해 여러 줄 호출을 편집하기 쉽게 한다. 언어 코드의 배정 여부,
    /// 중복과 빈 매핑은 코어에서 검증하고, 여기서는 토큰 종류와 구분자 오류를 보고한다.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let path = input.parse()?;
        let mut directories = Vec::new();
        while !input.is_empty() {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let language = input.parse()?;
            input.parse::<Token![=]>()?;
            directories.push((language, input.parse()?));
        }
        Ok(Self { path, directories })
    }
}

/// 구문을 파싱하고 선택된 언어에 맞는 경로를 기본 `include_str!` 호출로 바꾼다.
///
/// `CARGO_MANIFEST_DIR`는 소비 패키지에서 평가되도록 토큰으로 내보낸다.
/// 파일 읽기와 변경 추적도 rustc에 맡긴다. 구문 오류는 해당 토큰에, 코어의 의미
/// 검증 오류는 기본 경로 리터럴에 연결된 컴파일 오류로 반환한다.
pub(crate) fn expand(input: TokenStream, language: Option<&str>) -> TokenStream {
    let input = parse_macro_input!(input as DirectoryInput);
    let owned: Vec<_> = input
        .directories
        .iter()
        .map(|(code, directory)| (code.to_string(), directory.value()))
        .collect();
    let directories: Vec<_> = owned
        .iter()
        .map(|(code, directory)| (code.as_str(), directory.as_str()))
        .collect();
    match textus_core::i18n::directory_document_path(&input.path.value(), &directories, language) {
        Ok(path) => {
            let selected = LitStr::new(&path, input.path.span());
            quote! { ::core::include_str!(::core::concat!(
                ::core::env!("CARGO_MANIFEST_DIR"), "/", #selected
            )) }
            .into()
        }
        Err(message) => syn::Error::new(input.path.span(), message)
            .to_compile_error()
            .into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_explicit_mappings_and_rejects_malformed_arguments() {
        let parsed = syn::parse_str::<DirectoryInput>(
            r#""docs/guide.md", ko = "docs/ko/", en = "translations/english","#,
        )
        .unwrap();
        assert_eq!(parsed.directories.len(), 2);
        for input in [
            r#""a.md", "ko" = "ko""#,
            r#""a.md", ko = 1"#,
            r#""a.md", ko"#,
        ] {
            assert!(syn::parse_str::<DirectoryInput>(input).is_err());
        }
    }
}
