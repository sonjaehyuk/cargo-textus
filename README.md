# cargo-textus

**For Rustaceans who are truly serious about writing.**

## What is this?

If you are not a native English speaker, you will likely find it inconvenient that much of the Reference material, including docs.rs, is only available in English. This is because most of the Reference is generated through comments (`///`, `//!` in Rust), and providing i18n in such an environment is practically challenging.

**cargo-textus** is a tool for managing localized documentation and some additional documentation rendering features for Rust projects. Specifically, they are as follows:
* You can set additional languages in Cargo.toml. Then, when you call cargo-textus macros for documentation, you can use CLI commands to build documentation specifically for those additional languages (while the default documentation remains intact, of course).
* You can render **Mermaid** diagrams in your documentation.
* You can render **mathematical formulas** in your documentation.
* You can use the **blockquote highlights** provided like GitHub in your documentation.

## Table of Contents

1. [Quickstart](#quickstart)
   1. [Installation](#installation)
   2. [Configuration](#configuration)
   3. [Command-line usage](#command-line-usage)
2. [Document Writing](#document-writing)
   1. [Automatic language extension](#automatic-language-extension)
   2. [Directory-based](#directory-based)
   3. [Inline strings](#inline-strings)
3. [Development](#development)
4. [Deployment](#deployment)


## Quickstart

### Installation

You need to simultaneously install the binary via `cargo install` and the library via `cargo add`.

```shell
cargo install cargo-textus
cargo add cargo-textus # or just add it to your Cargo.toml
```

> [!NOTE]
> By configuring Cargo.toml as follows, you can remove unnecessary CLI executable code from the library (because you've already installed the CLI executable via `cargo install`): `cargo-textus = { default-features = false }`

### Configuration

Check the example `Cargo.toml` below.

```toml
[package.metadata.textus.i18n]
languages = ["ja", "ko"] # It must be a list of lowercase ISO 639-1 codes.

[package.metadata.textus.render]
mermaid = true
math = true
alerts = true
footnotes = true
# It must be a list of relative paths base on the Cargo.toml file location.
css = ["docs/custom.css"] 
js = ["docs/custom.js"]
```

`textus.render` works on the principle that additional features are injected into rustdoc by the `cargo textus` command. Therefore, render requires no additional configuration. However, to use `textus.i18n`, you must use the macros provided by the cargo-textus crate.
There are three ways to supply localized documentation:

* `#[doc = cargo_textus::include_str!("docs/greet.md")]`: 

In this case, the `docs/greet.md` file is used as the default documentation. So when you just run `cargo doc`, the `docs/greet.md` file is used. But if you run `cargo textus open --lang ko`, the **`docs/greet.ko.md`** file is used automatically. We recommend using `cargo textus open` (no `--lang` option) to open the default documentation.

```text
my-crate/
├── Cargo.toml
├── src/
└── docs/
    ├── guide.md       # Just normal cargo build / doc / test
    ├── guide.ja.md    # cargo textus COMMAND --lang ja
    └── guide.ko.md    # cargo textus COMMAND --lang ko
```

* `#[doc = cargo_textus::include_str_from_dir!("docs/greet.md", ko = "docs/ko/", ja = "docs/japanese",)]`

Also, in this case, the `docs/greet.md` file is used as the default documentation. When you just run `cargo textus open --lang ja`, the **`docs/japanese/greet.md`** file is used automatically.

```text
my-crate/
├── Cargo.toml
├── src/
└── docs/
    ├── guide.md       # Just normal cargo build / doc / test
    ├── japanese    
        └── guide.md    # cargo textus COMMAND --lang ja
```

* `#[doc = cargo_textus::localized_str!("Default documentation", ko = "한국어 문서", ja = "日本語の文書")]`

Supply documentation directly as string literals instead of separate files. With no language
selected, the first string is used; `cargo textus open --lang ko` selects the `ko` string.
Languages selected through the CLI must also be registered in the metadata above.

Check the [Document Writing](#document-writing) below for more details.

### Command-line usage

You can see the full list of commands by running [cargo textus --help](crates/cargo-textus/src/help.txt).

```shell
cargo textus --help           # Show the full list and description of commands.
cargo textus open             # Build and open the default documentation.
cargo textus open --lang ko   # Build and open the Korean documentation
cargo textus build --lang ko  # Build the Korean documentation
cargo textus languages       # See the list of registered languages.
cargo textus check            # Check the base document and all registered languages
```

## Document Writing

### Automatic language extension

```rust
#![doc = cargo_textus::include_str!("docs/overview.md")]

#[doc = cargo_textus::include_str!("docs/greet.md")]
pub fn greet() -> &'static str {
    "Hello!"
}

#[doc = cargo_textus::include_str!("docs/guide.md")]
pub fn example() {}
```

The language code is inserted before the final extension, so `api.guide.md` becomes `api.guide.ko.md`. The language of the default document is not enforced; if `--lang en` is requested, an `.en.md` file is required even if the default file was written in English.

File paths are **relative to the `Cargo.toml` of the package calling the macro**.
This may differ from the source-file-relative paths used by standard `include_str!`. Nested modules use the same package-relative standard. It accepts only a single string literal, requiring a relative path separated by `/` and a file extension. Absolute paths and `..` are not allowed.

The macro passes the selected path to the standard `include_str!`. It does not modify the `doc` attribute, standard doc comments, or default macros. If the selected file does not exist, a compilation error occurs, and it does not automatically fall back to another language. The file content is interpreted as rustdoc Markdown.

### Directory-based

```rust
#[doc = cargo_textus::include_str_from_dir!(
    "docs/greet.md",
    ko = "docs/ko/",
    en = "translations/english",
)]
pub fn greet() {}
```

The existing `include_str!` will remain the default method for inserting language codes into filenames. To separate directories, explicitly link the language and directory in the new macro.

| Language Selection | Included File                   |
|--------------------|---------------------------------|
| None               | `docs/greet.md`                 |
| `--lang ko`        | `docs/ko/greet.md`              |
| `--lang en`        | `translations/english/greet.md` |

Both the default path and directories are relative to the `Cargo.toml` of the calling package. The **full filename** of the default file is appended to the directory. For example, even if the default path is `docs/api/greet.md`, `ko = "docs/ko"` will select `docs/ko/greet.md`. Do not add the language code before the extension. A single trailing `/` in the directory is allowed. Absolute paths, empty directories, `..`, backslashes, and empty path segments are not permitted. The default file path requires a filename and extension, just like the existing macro.

At least one mapping is required, and the keys must be actual assigned lowercase ISO 639-1 codes. Duplicate keys and invalid codes or paths will cause a compilation error even if that language is not selected. If there is no mapping for the selected language or the selected file does not exist, an error is raised and no automatic fallback occurs. If no language is selected, only the default file is included. The existence of files that are not selected is not checked.

### Inline strings

Use `localized_str!` to keep a short description and its translations in Rust source:

```rust
#[doc = cargo_textus::localized_str!(
    "Returns a greeting.",
    ko = r#"인사말을 반환합니다.

여러 줄의 **Markdown**과 `코드`도 쓸 수 있습니다."#,
    en = "Returns a greeting in English.",
)]
pub fn greet() -> &'static str {
    "Hello!"
}
```

The first argument is the default documentation. It is followed by one or more
`language = "content"` mappings, with an optional trailing comma. Ordinary and raw
string literals are supported; whitespace, line breaks, escapes, and empty content
are preserved according to Rust string literal rules. Variables, constants, byte strings,
and expressions such as `concat!` are not accepted. The macro also works as a
`&'static str` expression, including in constants.

Keys must be assigned lowercase ISO 639-1 codes. All keys are validated, including
unselected ones; duplicates and an empty mapping list are compilation errors.
Rust keyword codes such as Assamese `as` are accepted; `as` and `r#as` refer to the
same code and cannot both be mapped in one call. If a selected language has no mapping,
compilation fails without falling back to the default or another language. An empty
mapped string is valid content and does not trigger fallback.

As with the file macros, no language selection means the default string is used.
To select `ko` through the CLI, also register `"ko"` in
`[package.metadata.textus.i18n].languages`. Macro mappings do not register languages
automatically, and registered languages do not supply missing macro strings.
`cargo textus check` builds the default documentation and all registered languages.

This macro performs compile-time string selection without reading or writing document
files. Rustdoc renders the selected Markdown through the same pipeline as file-based
documentation. `include_str!` and `include_str_from_dir!` remain available with their
existing behavior, and all three macros work with `default-features = false`.

### Footnote previews

Write standard rustdoc Markdown footnotes and build with `cargo textus build` or `cargo textus open`:

```markdown
See the details[^details].

[^details]: Footnotes can include **formatting**, links, code, and multiple paragraphs.

    Long notes remain scrollable inside the preview.
```

Hover over a footnote reference or focus it with the keyboard to open a preview.
The preview stays within the visible viewport, moving to the left near the right edge
or above/below on narrow screens. You can move the pointer into it, select text, follow
links, and scroll long or wide content. The page remains scrollable.
Press Tab or ArrowDown on the reference to enter the preview, and Escape to close it.
Tab continues through preview links and back into the document without trapping focus.
Clicking, pressing Enter, or tapping the original reference still jumps to the original footnote.

Previews are enabled by default. Set `footnotes = false` under
`[package.metadata.textus.render]` to disable them. They use rustdoc's generated footnote
HTML and bundled local CSS/JS; no extra macros or network services are needed.
Without JavaScript, the original footnotes remain available. Ordinary `cargo doc` does
not add previews. See the [rendering design](docs/rustdoc-rendering-plan.md) for details and limitations.

## Development

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo clippy -p cargo-textus --all-targets --no-default-features -- -D warnings
cargo test -p cargo-textus --no-default-features
python3 -m unittest discover -s scripts/tests
```
Integration tests run actual Cargo/rustdoc, so they require the Rust toolchain and dependency cache. Internal Cargo commands are executed with `--offline`. We verify Korean generation, preservation of standard `include_str!`, language switching, file modifications and omissions, inline string selection and mapping errors, and CLI and package selection. On Unix, we inspect the delivery path of `open` and the asset placement order using a test browser. Macro-only tests are also run with `--no-default-features`. Procedural macro libraries do not export general public function/type APIs; common logic is kept in `textus-core`.

| Directory             | Responsibility                                                       |
|-----------------------|----------------------------------------------------------------------|
| `crates/cargo-textus` | CLI binary and file/string documentation selection macros |
| `crates/textus-core`  | Language code validation and document path/string selection          |
| `examples/...`        | Executable example                                                   |

The language table was extracted from the `alpha_2` field of the system `iso-codes`' `iso_639-2.json` (2026-09-12, 183 entries). The source is the [ISO 639 Registration Authority's list of language codes](https://www.loc.gov/standards/iso639-2/php/code_list.php), and both the source and changes are reviewed together when the list is updated.

### AI

This project was something of a pipe dream. I used AI to quickly build a proof of concept, but even though Rust is a language AI handles well, there were points where the harness just couldn't overcome certain limitations. So, while this project was built with AI for now, the goal is to potentially remove the AI components later. This isn't some kind of exclusionary manifesto or a scientific theory; it's simply the maintainer's personal preference.

## Deployment

Deployed from the maintainer's Forgejo Actions. You can find it on Crates.io: https://crates.io/crates/cargo-textus
