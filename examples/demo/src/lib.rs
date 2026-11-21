#![doc = cargo_textus::include_str!("docs/overview.md")]

#[doc = cargo_textus::include_str!("docs/greet.md")]
pub fn greet() -> &'static str {
    "Hello!"
}

/// Ordinary Rust documentation remains unchanged.
#[doc = include_str!("../docs/builtin.md")]
pub struct Ordinary;

#[doc = cargo_textus::include_str_from_dir!(
    "docs/greet.md", ko = "docs/ko/", en = "docs/english",
)]
pub fn greet_from_directory() -> &'static str {
    "Hello!"
}

#[doc = cargo_textus::localized_str!(
    "Returns a greeting, documented without a separate Markdown file.",
    ko = r#"별도의 Markdown 파일 없이 인사말을 문서화합니다.

일반 문서처럼 **강조**와 `코드`를 쓸 수 있습니다."#,
    en = "Returns a greeting with inline English documentation.",
)]
pub fn greet_with_inline_docs() -> &'static str {
    "Hello!"
}
