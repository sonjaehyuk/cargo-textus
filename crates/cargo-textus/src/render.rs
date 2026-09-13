//! rustdoc의 전체 API 페이지에 공통 브라우저 자산을 주입하는 기능 계층.
//! 원문은 rustdoc에 전달하고 다이어그램과 수식 해석은 브라우저 라이브러리에 위임한다.

use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::{
    cargo,
    cli::{Action, Options},
    config,
};

#[path = "render_assets.rs"]
mod assets;

/// 패키지 전체의 렌더링 설정. 생략하면 Mermaid·수식·알림을 활성화한다.
/// 사용자 자산은 패키지 기준 파일로 명시하며 개별 문서 매크로에 의존하지 않는다.
#[derive(Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
struct RenderConfig {
    /// Mermaid 코드 블록을 브라우저에서 SVG로 변환할지 결정한다.
    mermaid: bool,
    /// 문서 영역의 달러 구분 수식을 KaTeX로 표시할지 결정한다.
    math: bool,
    /// 최상위 blockquote의 GitHub 알림 마커를 제목·아이콘·강조 스타일로 표시한다.
    alerts: bool,
    /// 선언 순서대로 읽을 CSS 파일. 페이지 전체에 스타일이 적용된다.
    css: Vec<String>,
    /// 내장 렌더링 처리 후 선언 순서대로 실행할 JavaScript 파일.
    js: Vec<String>,
}

impl Default for RenderConfig {
    /// 설정 없이도 일반 주석의 그림과 수식을 처리하는 초기값.
    fn default() -> Self {
        Self {
            mermaid: true,
            math: true,
            alerts: true,
            css: Vec::new(),
            js: Vec::new(),
        }
    }
}

/// 대상 라이브러리의 전체 API 페이지를 공통 헤더와 함께 생성한다.
/// 언어를 지정할 때만 i18n 설정을 검증하며 일반 render는 언어 등록을 요구하지 않는다.
pub fn run(options: Options) -> Result<()> {
    let project = config::load_package(&options)?;
    if let Some(language) = &options.language {
        project.i18n_config()?.require_language(language)?;
    }
    let mut config: RenderConfig = project
        .metadata
        .pointer("/textus/render")
        .map(|value| serde_json::from_value(value.clone()))
        .transpose()
        .context("invalid [package.metadata.textus.render] configuration")?
        .unwrap_or_default();
    let target = project
        .target_directory
        .join("textus/render")
        .join(&project.name)
        .join(options.language.as_deref().unwrap_or("default"));
    let staging = target.join("textus-assets");
    fs::create_dir_all(&staging).context("could not prepare rendering assets")?;
    for (name, bytes) in assets::FILES {
        let path = staging.join(name);
        fs::create_dir_all(path.parent().unwrap())?;
        fs::write(path, bytes)?;
    }
    if config.alerts {
        fs::write(
            staging.join("alerts.css"),
            include_str!("render/alerts.css"),
        )?;
    }
    let package_root = project
        .manifest_path
        .parent()
        .context("manifest has no parent")?;
    stage_custom(&mut config.css, "css", package_root, &staging)?;
    stage_custom(&mut config.js, "js", package_root, &staging)?;
    let runtime = include_str!("render/runtime.js")
        .replace("__TEXTUS_CONFIG__", &serde_json::to_string(&config)?)
        .replace("__TEXTUS_ALERTS__", include_str!("render/alerts.js"));
    let header = target.join("textus-header.html");
    fs::write(
        &header,
        format!("<script data-textus-render>\n{runtime}\n</script>\n"),
    )?;

    // 삭제된 포함 파일과 설정 변경도 재검사하도록 전용 문서 산출물만 정리한다.
    let status = cargo::command(&options)
        .args(["clean", "--doc", "--manifest-path"])
        .arg(&project.manifest_path)
        .arg("--target-dir")
        .arg(&target)
        .status()?;
    if !status.success() {
        bail!("could not clean rendering documentation ({status})");
    }
    let mut command = cargo::command(&options);
    command
        .args(["rustdoc", "--lib", "--manifest-path"])
        .arg(&project.manifest_path)
        .arg("--package")
        .arg(&project.name)
        .arg("--target-dir")
        .arg(&target);
    if let Some(language) = &options.language {
        command.env("TEXTUS_LANG", language);
    } else {
        command.env_remove("TEXTUS_LANG");
    }
    // 기존 Cargo rustdoc 플래그는 보존하고 추가 인자로 헤더만 전달한다.
    command.arg("--").arg("--html-in-header").arg(&header);
    let status = command
        .status()
        .context("could not run rendering rustdoc")?;
    if !status.success() {
        bail!("rendering rustdoc failed ({status})");
    }
    if install_assets(&target, &staging, 0)? == 0 {
        bail!("could not locate rustdoc output for rendering assets");
    }
    if options.open || options.action == Action::Open {
        // 자산 배치 후 같은 빌드를 --open으로 호출해 브라우저 선택은 Cargo에 맡긴다.
        let mut open = cargo::command(&options);
        open.args(["rustdoc", "--lib", "--open", "--manifest-path"])
            .arg(&project.manifest_path)
            .arg("--package")
            .arg(&project.name)
            .arg("--target-dir")
            .arg(&target);
        if let Some(language) = &options.language {
            open.env("TEXTUS_LANG", language);
        } else {
            open.env_remove("TEXTUS_LANG");
        }
        let status = open
            .arg("--")
            .arg("--html-in-header")
            .arg(&header)
            .status()?;
        if !status.success() {
            bail!("could not open rendering documentation ({status})");
        }
    }
    Ok(())
}

/// 사용자 자산을 고정 이름으로 복사해 원본 경로가 HTML이나 JavaScript에 섞이지 않게 한다.
/// 읽기 실패와 잘못된 상대 경로는 빌드 전에 오류로 알린다. 심볼릭 링크 대상은 제한하지 않는다.
fn stage_custom(paths: &mut [String], extension: &str, root: &Path, staging: &Path) -> Result<()> {
    for (index, path) in paths.iter_mut().enumerate() {
        if path.is_empty()
            || path.starts_with('/')
            || path.contains(['\\', ':'])
            || path.split('/').any(|part| part.is_empty() || part == "..")
        {
            bail!(
                "render asset must be a nonempty package-relative path using /, without ..: {path:?}"
            );
        }
        let source = root.join(&*path);
        let name = format!("custom-{index}.{extension}");
        fs::copy(&source, staging.join(&name))
            .with_context(|| format!("could not read render asset {}", source.display()))?;
        *path = name;
    }
    Ok(())
}

/// Cargo의 기본 출력과 명시적 타깃 출력의 doc 디렉토리에 동일한 자산을 배치한다.
/// target 바로 아래와 타깃 triple 바로 아래만 살피며 컴파일러 캐시는 순회하지 않는다.
fn install_assets(target: &Path, staging: &Path, depth: usize) -> Result<usize> {
    let mut count = 0;
    for entry in fs::read_dir(target)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        if entry.file_name() == "doc" {
            copy_tree(staging, &entry.path().join("textus-assets"))?;
            count += 1;
        } else if depth == 0
            && !matches!(
                entry.file_name().to_str(),
                Some("debug" | "release" | "textus-assets")
            )
        {
            count += install_assets(&entry.path(), staging, depth + 1)?;
        }
    }
    Ok(count)
}

/// 폰트를 포함한 상대 디렉토리 구조를 보존하며 생성 자산만 복제한다.
/// 원본 문서는 이 복사 작업의 입력이나 출력으로 사용하지 않는다.
fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), destination.join(entry.file_name()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_defaults_and_errors() {
        let config: RenderConfig = serde_json::from_str("{}").unwrap();
        assert!(config.mermaid && config.math && config.alerts);
        let config: RenderConfig =
            serde_json::from_str(r#"{"math":false,"alerts":false,"js":["a.js"]}"#).unwrap();
        assert!(config.mermaid && !config.math && !config.alerts);
        assert_eq!(config.js, ["a.js"]);
        for json in [
            r#"{"math":"yes"}"#,
            r#"{"alerts":"true"}"#,
            r#"{"unknown":true}"#,
            r#"{"css":"a.css"}"#,
        ] {
            assert!(serde_json::from_str::<RenderConfig>(json).is_err());
        }
    }

    #[test]
    fn custom_assets_reject_nonportable_paths_before_reading() {
        for path in ["", "../a.js", "/a.js", "C:/a.js", "a//b.js", "a\\b.js"] {
            assert!(
                stage_custom(&mut [path.into()], "js", Path::new("."), Path::new(".")).is_err()
            );
        }
    }
}
