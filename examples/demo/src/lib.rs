#![doc = textus::include_doc!("docs/overview.md")]

#[doc = textus::include_doc!("docs/greet.md")]
pub fn greet() -> &'static str {
    "Hello!"
}

/// Ordinary Rust documentation remains unchanged.
#[doc = include_str!("../docs/builtin.md")]
pub struct Ordinary;
