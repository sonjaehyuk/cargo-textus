//! This target also runs with --no-default-features, without CLI dependencies.

#[doc = cargo_textus::include_str!("tests/fixtures/guide.md")]
struct Documented;

#[test]
fn macro_is_usable_as_an_attribute_and_string_expression() {
    let _ = Documented;
    let expected = match option_env!("TEXTUS_LANG") {
        None => "Default documentation\n",
        Some("en") => "English documentation\n",
        Some("ko") => "한국어 문서\n",
        Some(language) => panic!("no test fixture for language {language}"),
    };
    assert_eq!(
        cargo_textus::include_str!("tests/fixtures/guide.md"),
        expected
    );
}

#[doc = cargo_textus::include_str_from_dir!(
    "tests/fixtures/guide.md", ko = "tests/fixtures/ko/", en = "tests/fixtures/english",
)]
struct DirectoryDocumented;

#[test]
fn directory_macro_selects_explicit_mapping() {
    let _ = DirectoryDocumented;
    let expected = match option_env!("TEXTUS_LANG") {
        None => "Default documentation\n",
        Some("en") => "English directory documentation\n",
        Some("ko") => "한국어 디렉토리 문서\n",
        Some(language) => panic!("no test fixture for language {language}"),
    };
    assert_eq!(
        cargo_textus::include_str_from_dir!(
            "tests/fixtures/guide.md",
            ko = "tests/fixtures/ko/",
            en = "tests/fixtures/english",
        ),
        expected
    );
}

#[doc = cargo_textus::localized_str!(
    "Default inline documentation.",
    ko = "한국어 직접 문서입니다.",
    en = "English inline documentation.",
)]
struct InlineDocumented;

#[test]
fn localized_string_works_in_attributes_and_constant_expressions() {
    let _ = InlineDocumented;
    const TEXT: &str = cargo_textus::localized_str!(
        "  Default\n\"quoted\" \\ text\0 ",
        ko = r#"  한국어
    "인용" \ 문서 "#,
        en = "",
        as = "অসমীয়া",
    );
    let expected = match option_env!("TEXTUS_LANG") {
        None => "  Default\n\"quoted\" \\ text\0 ",
        Some("ko") => "  한국어\n    \"인용\" \\ 문서 ",
        Some("en") => "",
        Some(language) => panic!("no test fixture for language {language}"),
    };
    assert_eq!(TEXT, expected);
    assert_eq!(cargo_textus::localized_str!("", ko = "", en = ""), "");
}
