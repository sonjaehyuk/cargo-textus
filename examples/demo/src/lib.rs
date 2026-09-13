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
