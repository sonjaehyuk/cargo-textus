//! Exercise real Cargo/rustdoc invocations, including incremental rebuilds.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("textus test {} {nonce}", std::process::id()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir(root.join("docs")).unwrap();
        let macro_crate = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        fs::write(
            root.join("Cargo.toml"),
            format!(
            "[workspace]\n[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\
             [dependencies]\ncargo-textus = {{ path = {}, default-features = false }}\n\
             [package.metadata.textus.i18n]\nlanguages = [\"en\", \"ko\"]\n",
            serde_json::to_string(&macro_crate).unwrap()
        ),
        )
        .unwrap();
        fs::write(
            root.join("src/lib.rs"),
            r##"#![doc = cargo_textus::include_str!("docs/guide.md")]
#[doc = cargo_textus::include_str!(r#"docs/guide.md"#)]
pub fn greet() {}
/// Ordinary documentation is preserved.
#[doc = include_str!("../docs/builtin.md")]
pub struct Ordinary;
pub mod nested;
"##,
        )
        .unwrap();
        fs::write(
            root.join("src/nested.rs"),
            r#"#[doc = cargo_textus::include_str!("docs/guide.md")]
pub struct Nested {
    #[doc = cargo_textus::include_str!("docs/guide.md")]
    pub field: u8,
}
"#,
        )
        .unwrap();
        fs::write(root.join("docs/guide.md"), "Default guide marker").unwrap();
        fs::write(root.join("docs/guide.en.md"), "English guide marker").unwrap();
        fs::write(root.join("docs/guide.ko.md"), "한국어 문서 표식").unwrap();
        fs::write(root.join("docs/builtin.md"), "Built-in marker").unwrap();
        Self { root }
    }

    fn prepare(&self, command: &mut Command) {
        command
            .current_dir(&self.root)
            .env_remove("TEXTUS_LANG")
            .env("CARGO_TARGET_DIR", self.root.join("target"));
    }

    fn cli(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-textus"));
        self.prepare(&mut command);
        command.arg("textus").args(arguments).arg("--offline");
        command
    }

    fn cargo_doc(&self, language: Option<&str>) -> Output {
        let mut command = Command::new(env!("CARGO"));
        self.prepare(&mut command);
        command.args(["doc", "--no-deps", "--offline"]);
        if let Some(language) = language {
            command.env("TEXTUS_LANG", language);
        }
        command.output().unwrap()
    }

    fn html(&self, language: Option<&str>, page: &str) -> String {
        let target = match language {
            Some(language) => self
                .root
                .join("target/textus")
                .join("fixture")
                .join(language),
            None => self.root.join("target"),
        };
        fs::read_to_string(target.join("doc/fixture").join(page)).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if std::thread::panicking() {
            eprintln!("Failed test fixture retained at {}", self.root.display());
            return;
        }
        // This directory is created exclusively by this test.
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn failure(output: Output, message: &str) {
    assert!(
        !output.status.success(),
        "expected failure containing {message:?}; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(message),
        "expected {message:?}, got:\n{stderr}"
    );
}

#[test]
fn localized_rustdoc_and_incremental_rebuilds() {
    let fixture = Fixture::new();
    let original_source = fs::read(fixture.root.join("src/lib.rs")).unwrap();
    let original_manifest = fs::read(fixture.root.join("Cargo.toml")).unwrap();

    let list = success(fixture.cli(&["languages"]).output().unwrap());
    assert_eq!(String::from_utf8(list.stdout).unwrap(), "en\nko\n");
    failure(
        fixture.cli(&["build", "--lang", "zz"]).output().unwrap(),
        "invalid ISO 639-1",
    );
    failure(
        fixture.cli(&["build", "--lang", "ja"]).output().unwrap(),
        "not registered",
    );

    success(fixture.cargo_doc(None));
    assert!(
        fixture
            .html(None, "index.html")
            .contains("Default guide marker")
    );
    let checked = success(fixture.cli(&["check"]).output().unwrap());
    assert!(
        String::from_utf8_lossy(&checked.stdout).contains("Checked 3 documentation variant(s)")
    );
    let default_html = fs::read_to_string(
        fixture
            .root
            .join("target/textus/fixture/default/doc/fixture/index.html"),
    )
    .unwrap();
    assert!(default_html.contains("Default guide marker"));
    assert!(default_html.contains("data-textus-render"));
    assert!(
        fixture
            .html(Some("en"), "index.html")
            .contains("English guide marker")
    );
    for page in ["index.html", "fn.greet.html", "nested/struct.Nested.html"] {
        assert!(
            fixture.html(Some("ko"), page).contains("한국어 문서 표식"),
            "{page}"
        );
    }
    let ordinary = fixture.html(Some("ko"), "struct.Ordinary.html");
    assert!(ordinary.contains("Built-in marker"));
    assert!(ordinary.contains("Ordinary documentation is preserved."));
    assert!(
        fixture
            .html(None, "index.html")
            .contains("Default guide marker")
    );

    // Force language changes within the SAME target directory. This catches
    // untracked proc-macro environment reads hidden by per-language targets.
    for (language, marker) in [
        (Some("ko"), "한국어 문서 표식"),
        (Some("en"), "English guide marker"),
        (None, "Default guide marker"),
    ] {
        success(fixture.cargo_doc(language));
        assert!(fixture.html(None, "index.html").contains(marker));
    }

    // The built-in include_str! must track document edits without source edits.
    fs::write(
        fixture.root.join("docs/guide.ko.md"),
        "수정된 한국어 문서 표식",
    )
    .unwrap();
    success(fixture.cli(&["build", "--lang", "ko"]).output().unwrap());
    assert!(
        fixture
            .html(Some("ko"), "index.html")
            .contains("수정된 한국어 문서 표식")
    );
    assert!(
        fixture
            .html(Some("en"), "index.html")
            .contains("English guide marker")
    );

    #[cfg(unix)]
    check_browser(&fixture);

    // A registered but missing document must fail, even with old HTML present.
    fs::remove_file(fixture.root.join("docs/guide.ko.md")).unwrap();
    failure(
        fixture.cli(&["check", "--lang", "ko"]).output().unwrap(),
        "guide.ko.md",
    );

    assert_eq!(
        fs::read(fixture.root.join("src/lib.rs")).unwrap(),
        original_source
    );
    assert_eq!(
        fs::read(fixture.root.join("Cargo.toml")).unwrap(),
        original_manifest
    );

    // Malformed macro input is a useful compiler diagnostic, not a panic.
    fs::write(
        fixture.root.join("src/lib.rs"),
        "#[doc = cargo_textus::include_str!(123)]\npub struct Invalid;\n",
    )
    .unwrap();
    failure(fixture.cargo_doc(None), "expected string literal");
}

#[cfg(unix)]
fn check_browser(fixture: &Fixture) {
    use std::os::unix::fs::PermissionsExt;
    let script = fixture.root.join("browser.sh");
    let log = fixture.root.join("browser.log");
    fs::write(
        &script,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$TEXTUS_TEST_BROWSER_LOG\"\n",
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    for args in [&["open", "--lang", "ko"][..]] {
        let mut command = fixture.cli(args);
        // Pass the executable path directly, including spaces.
        command
            .env("BROWSER", &script)
            .env("TEXTUS_TEST_BROWSER_LOG", &log);
        let output = success(command.output().unwrap());
        for _ in 0..100 {
            if log.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let opened = fs::read_to_string(&log).unwrap_or_else(|error| {
            panic!(
                "Cargo should invoke the configured browser: {error}; {}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
        assert!(
            opened.contains("/textus/fixture/ko/doc/fixture/index.html"),
            "{opened}"
        );
        fs::remove_file(&log).unwrap();
    }
}

#[test]
fn selects_workspace_packages_explicitly() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let command = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_cargo-textus"))
            .current_dir(&root)
            .args(["languages", "--offline"])
            .args(args)
            .output()
            .unwrap()
    };
    failure(command(&[]), "select one workspace package");
    failure(command(&["-p", "does-not-exist"]), "not found");
    assert!(success(command(&["-p", "cargo-textus"])).stdout.is_empty());
    let result = success(command(&["-p", "textus-demo"]));
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "en\nko\n");
    let result = success(command(&["--manifest-path", "examples/demo/Cargo.toml"]));
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "en\nko\n");
}

#[test]
fn directory_documents_rebuild_and_report_missing_mappings_and_files() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join("docs/ko")).unwrap();
    fs::create_dir(fixture.root.join("docs/english")).unwrap();
    fs::write(
        fixture.root.join("docs/ko/guide.md"),
        "Directory Korean marker",
    )
    .unwrap();
    fs::write(
        fixture.root.join("docs/english/guide.md"),
        "Directory English marker",
    )
    .unwrap();
    fs::write(fixture.root.join("src/lib.rs"), r#"#![doc = cargo_textus::include_str_from_dir!("docs/guide.md", ko = "docs/ko/", en = "docs/english")]
"#).unwrap();
    for (language, marker) in [
        (None, "Default guide marker"),
        (Some("ko"), "Directory Korean marker"),
        (Some("en"), "Directory English marker"),
        (None, "Default guide marker"),
    ] {
        success(fixture.cargo_doc(language));
        assert!(fixture.html(None, "index.html").contains(marker));
    }
    failure(
        fixture.cargo_doc(Some("ja")),
        "no document directory registered",
    );
    fs::remove_file(fixture.root.join("docs/ko/guide.md")).unwrap();
    failure(
        fixture.cli(&["check", "--lang", "ko"]).output().unwrap(),
        "docs/ko/guide.md",
    );
}

#[test]
fn rendering_is_global_preserves_flags_and_composes_with_i18n() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join("extra assets")).unwrap();
    fs::write(
        fixture.root.join("extra assets/style.css"),
        ".docblock { color: red; }",
    )
    .unwrap();
    fs::write(
        fixture.root.join("extra assets/custom.js"),
        "window.customLoaded = true;",
    )
    .unwrap();
    let manifest = fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap();
    let manifest = manifest
        + "\n[package.metadata.textus.render]\ncss = [\"extra assets/style.css\"]\njs = [\"extra assets/custom.js\"]\n";
    fs::write(fixture.root.join("Cargo.toml"), manifest).unwrap();
    let extra = fixture.root.join("extra assets/header.html");
    fs::write(&extra, "<meta name=existing-header content=preserved>").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-textus"));
    fixture.prepare(&mut command);
    command.args(["build", "--lang", "ko", "--offline"]).env(
        "CARGO_ENCODED_RUSTDOCFLAGS",
        format!("--html-in-header\x1f{}", extra.display()),
    );
    success(command.output().unwrap());
    let doc = fixture.root.join("target/textus/fixture/ko/doc");
    for page in ["index.html", "fn.greet.html", "nested/struct.Nested.html"] {
        let html = fs::read_to_string(doc.join("fixture").join(page)).unwrap();
        assert!(html.contains("data-textus-render"));
        assert!(html.contains("existing-header"));
        assert!(html.contains("한국어 문서 표식"));
    }
    assert!(doc.join("textus-assets/mermaid.js").is_file());
    assert!(
        doc.join("textus-assets/fonts/KaTeX_Main-Regular.woff2")
            .is_file()
    );
    assert_eq!(
        fs::read_to_string(doc.join("textus-assets/custom-0.js")).unwrap(),
        "window.customLoaded = true;"
    );
    fs::remove_file(fixture.root.join("extra assets/custom.js")).unwrap();
    failure(command.output().unwrap(), "could not read render asset");
    fs::write(
        fixture.root.join("extra assets/custom.js"),
        "window.customLoaded = true;",
    )
    .unwrap();
    let manifest = fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap();
    fs::write(
        fixture.root.join("Cargo.toml"),
        manifest.replace(
            "[package.metadata.textus.render]",
            "[package.metadata.textus.render]\nalerts = false",
        ),
    )
    .unwrap();
    success(command.output().unwrap());
    let html = fs::read_to_string(doc.join("fixture/index.html")).unwrap();
    assert!(html.contains("\"alerts\":false"));
}

#[cfg(unix)]
#[test]
fn render_opens_with_assets_ready_and_without_i18n_configuration() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let manifest = fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap();
    fs::write(
        fixture.root.join("Cargo.toml"),
        manifest
            .split("[package.metadata.textus.i18n]")
            .next()
            .unwrap(),
    )
    .unwrap();
    let script = fixture.root.join("render browser.sh");
    let log = fixture.root.join("render-browser.log");
    let asset = fixture
        .root
        .join("target/textus/fixture/default/doc/textus-assets/mermaid.js");
    fs::write(&script, "#!/bin/sh\nif [ -f \"$TEXTUS_TEST_ASSET\" ]; then printf '%s\\n' \"$@\" > \"$TEXTUS_TEST_BROWSER_LOG\"; fi\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    for args in [&["open"][..]] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_cargo-textus"));
        fixture.prepare(&mut command);
        command
            .args(args)
            .arg("--offline")
            .env("TEXTUS_LANG", "ko")
            .env("BROWSER", &script)
            .env("TEXTUS_TEST_ASSET", &asset)
            .env("TEXTUS_TEST_BROWSER_LOG", &log);
        success(command.output().unwrap());
        for _ in 0..100 {
            if log.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert!(
            fs::read_to_string(&log)
                .unwrap()
                .contains("/textus/fixture/default/doc/fixture/index.html")
        );
        let html = fs::read_to_string(
            fixture
                .root
                .join("target/textus/fixture/default/doc/fixture/index.html"),
        )
        .unwrap();
        assert!(html.contains("Default guide marker"));
        fs::remove_file(&log).unwrap();
    }
}

#[test]
fn unified_commands_work_without_language_metadata() {
    let fixture = Fixture::new();
    let manifest = fs::read_to_string(fixture.root.join("Cargo.toml")).unwrap();
    fs::write(
        fixture.root.join("Cargo.toml"),
        manifest
            .split("[package.metadata.textus.i18n]")
            .next()
            .unwrap(),
    )
    .unwrap();
    assert!(
        success(fixture.cli(&["languages"]).output().unwrap())
            .stdout
            .is_empty()
    );
    success(
        fixture
            .cli(&["build"])
            .env("TEXTUS_LANG", "ko")
            .output()
            .unwrap(),
    );
    let html = fs::read_to_string(
        fixture
            .root
            .join("target/textus/fixture/default/doc/fixture/index.html"),
    )
    .unwrap();
    assert!(html.contains("Default guide marker") && html.contains("data-textus-render"));
    let result = success(fixture.cli(&["check"]).output().unwrap());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Checked 1 documentation variant(s)"));
    failure(
        fixture.cli(&["build", "--lang", "ko"]).output().unwrap(),
        "no languages registered",
    );
}

#[test]
fn check_includes_default_but_selected_language_does_not() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.root.join("docs/guide.md")).unwrap();
    let result = success(fixture.cli(&["check", "--lang", "ko"]).output().unwrap());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Checked 1 documentation variant(s)"));
    assert!(
        !fixture
            .root
            .join("target/textus/fixture/default/doc/fixture/index.html")
            .exists()
    );
    failure(fixture.cli(&["check"]).output().unwrap(), "docs/guide.md");
    assert!(
        !fixture
            .root
            .join("target/textus/fixture/en/doc/fixture/index.html")
            .exists()
    );
}

#[test]
fn help_and_removed_commands_never_start_a_build() {
    let fixture = Fixture::new();
    let output = success(
        fixture
            .cli(&["--help", "--manifest-path", "missing.toml"])
            .output()
            .unwrap(),
    );
    let help = String::from_utf8(output.stdout).unwrap();
    for text in [
        "cargo textus <COMMAND>",
        "languages",
        "기본 문서부터",
        "렌더링 설정",
        "doctest",
    ] {
        assert!(help.contains(text), "{text}");
    }
    for args in [
        &["i18n", "open", "--lang", "ko"][..],
        &["render", "build"][..],
        &["build", "--open"][..],
        &["list"][..],
    ] {
        failure(fixture.cli(args).output().unwrap(), "cargo textus");
    }
    assert!(!fixture.root.join("target").exists());
}
