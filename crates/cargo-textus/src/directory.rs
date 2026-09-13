use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

struct DirectoryInput {
    path: LitStr,
    directories: Vec<(Ident, LitStr)>,
}

impl Parse for DirectoryInput {
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
