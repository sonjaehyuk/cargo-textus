//! Opt-in, file-based localized documentation without replacing Rust built-ins.
//!
//! `#[doc = textus::include_doc!("docs/guide.md")]` uses `docs/guide.md`
//! normally and `docs/guide.ko.md` when built through textus with `--lang ko`.
//! Paths are relative to the consuming package's `Cargo.toml`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

// Deliberately evaluated when Cargo compiles this macro crate, NOT by reading
// std::env during macro expansion. option_env! records an env dependency in
// rustc dep-info, so Cargo rebuilds the macro and its consumers on changes,
// including transitions to/from an unset variable on stable Rust.
const LANGUAGE: Option<&str> = option_env!("TEXTUS_LANG");

/// Include a user-authored documentation file for the selected language.
///
/// Accepts one string literal. Missing localized files are compilation errors;
/// there is no implicit fallback or translation. The emitted built-in
/// `include_str!` reads the file and tracks it for incremental builds.
#[proc_macro]
pub fn include_doc(input: TokenStream) -> TokenStream {
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
