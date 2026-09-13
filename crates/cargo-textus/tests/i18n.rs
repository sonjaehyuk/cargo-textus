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
        command
            .args(["textus", "i18n"])
            .args(arguments)
            .arg("--offline");
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
                .join(language)
                .join("fixture"),
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

    let list = success(fixture.cli(&["list"]).output().unwrap());
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
    success(fixture.cli(&["check"]).output().unwrap());
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
    for args in [
        &["open", "--lang", "ko"][..],
        &["build", "--lang", "ko", "--open"][..],
    ] {
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
            opened.contains("/textus/ko/fixture/doc/fixture/index.html"),
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
            .args(["i18n", "list", "--offline"])
            .args(args)
            .output()
            .unwrap()
    };
    failure(command(&[]), "select one workspace package");
    failure(command(&["-p", "does-not-exist"]), "not found");
    failure(
        command(&["-p", "cargo-textus"]),
        "missing [package.metadata.textus.i18n]",
    );
    let result = success(command(&["-p", "textus-demo"]));
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "en\nko\n");
    let result = success(command(&["--manifest-path", "examples/demo/Cargo.toml"]));
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "en\nko\n");
}
