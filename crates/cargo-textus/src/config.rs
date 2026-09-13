//! Cargo metadata에서 공통 대상 패키지를 선택하고 필요할 때 i18n 지원 선언을 검증한다.
//! Cargo가 해석한 경로와 워크스페이스 정보를 사용해 TOML 해석을 중복 구현하지 않는다.

use std::{collections::BTreeSet, path::PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{cargo, cli::Options};

/// Cargo metadata 응답에서 패키지 선택과 산출물 위치 결정에 필요한 정보만 읽는다.
///
/// 전체 응답 스키마에 결합하지 않도록 사용하지 않는 Cargo 필드는 역직렬화 시 무시한다.
#[derive(Deserialize)]
struct Metadata {
    /// 응답에 포함된 패키지 목록. 실제 선택 전 워크스페이스 구성원으로 범위를 좁힌다.
    packages: Vec<Package>,
    /// 외부 의존성을 대상에서 제외할 때 비교하는 Cargo 패키지 식별자 목록.
    workspace_members: Vec<String>,
    /// Cargo 설정이 반영된 산출물 기준 경로. 저장소의 `target`으로 고정하지 않는다.
    target_directory: PathBuf,
}

/// 대상 패키지를 식별하고 사용자 metadata를 가져오기 위한 Cargo 응답 항목.
#[derive(Deserialize)]
struct Package {
    /// Cargo가 제공한 고유 식별자. 워크스페이스 소속 여부를 이름 대신 이 값으로 비교한다.
    id: String,
    /// 사용자가 `--package`로 지정할 이름이며 생성 문서의 패키지별 분리에 사용한다.
    name: String,
    /// 대상 패키지의 매니페스트 위치. 현재 디렉토리와 무관하게 후속 명령을 실행하는 기준이다.
    manifest_path: PathBuf,
    /// 다른 도구의 설정도 포함하는 원본 값. `/textus/i18n` 부분만 별도로 검증한다.
    metadata: serde_json::Value,
}

/// 패키지가 지원하겠다고 선언한 문서 언어 설정.
///
/// 알 수 없는 필드를 거부해 설정 오타가 묵인되지 않게 한다.
/// 이 선언은 지원 의사 표시이며 매크로별 파일 경로나 번역의 완전성을 보장하지 않는다.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct I18nConfig {
    /// 중복 없는 ISO 639-1 코드 목록. 선언 순서를 유지해 목록 출력과 전체 검사에 사용한다.
    pub languages: Vec<String>,
}

/// 설정 로딩과 패키지 선택을 마친 실행 대상.
///
/// 빌드 단계가 Cargo metadata를 다시 해석하지 않도록 확정된 경로와 설정을 함께 전달한다.
pub struct Project {
    /// 선택된 패키지 이름. Cargo의 패키지 선택과 산출물 격리에 함께 사용한다.
    pub name: String,
    /// 선택된 패키지의 매니페스트. 원래 CLI에 지정한 워크스페이스 매니페스트와 다를 수 있다.
    pub manifest_path: PathBuf,
    /// Cargo가 결정한 공통 산출물 경로. 아래에 textus의 언어별·패키지별 경로를 만든다.
    pub target_directory: PathBuf,
    /// 지원 언어의 형식·중복·빈 목록 검증을 통과한 설정.
    pub config: I18nConfig,
}

impl I18nConfig {
    /// 지원 언어 목록이 비어 있지 않고 실제 배정된 코드로 중복 없이 구성됐는지 검사한다.
    ///
    /// 설정 전체를 한 번 검증해 이후 목록 출력과 빌드에서 같은 전제를 사용하게 한다.
    /// 파일 시스템에는 접근하지 않으며 오류가 있으면 첫 번째 원인을 반환한다.
    fn validate(&self) -> Result<()> {
        if self.languages.is_empty() {
            bail!("textus i18n languages must not be empty");
        }
        let mut seen = BTreeSet::new();
        for language in &self.languages {
            textus_core::i18n::validate_language(language).map_err(anyhow::Error::msg)?;
            if !seen.insert(language) {
                bail!("duplicate language {language:?} in textus i18n configuration");
            }
        }
        Ok(())
    }

    /// 요청 언어가 프로젝트의 지원 목록에 있는지 확인하고 없으면 가능한 언어를 안내한다.
    ///
    /// 코드 자체의 ISO 639-1 유효성 검증과는 별개다. CLI 파싱에서 코드 검증을 마친 뒤
    /// 호출하므로 유효하지만 등록되지 않은 언어를 구분해서 보고할 수 있다.
    pub fn require_language(&self, language: &str) -> Result<()> {
        if !self.languages.iter().any(|entry| entry == language) {
            bail!(
                "language {language:?} is not registered; available languages: {}",
                self.languages.join(", ")
            );
        }
        Ok(())
    }
}

/// Cargo metadata를 실행해 대상 패키지와 기능별 설정의 원본을 반환한다.
///
/// CLI의 오프라인·잠금 파일 옵션을 조회에도 적용한다. 워크스페이스 구성원만 선택
/// 대상에 둔다. 기능별 설정 검증은 호출 계층에서 수행한다.
/// Cargo 실행 실패와 응답 해석 실패에는 처리 단계의 맥락을 붙인다.
pub fn load_package(options: &Options) -> Result<PackageProject> {
    let mut command = cargo::command(options);
    command.args(["metadata", "--format-version", "1", "--no-deps"]);
    if let Some(path) = &options.manifest_path {
        command.arg("--manifest-path").arg(path);
    }
    let output = command.output().context("could not run cargo metadata")?;
    if !output.status.success() {
        bail!(
            "cargo metadata failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .context("could not decode cargo metadata --format-version 1")?;
    let members: Vec<_> = metadata
        .packages
        .into_iter()
        .filter(|package| metadata.workspace_members.contains(&package.id))
        .collect();
    let package = select_package(&members, options)?;
    Ok(PackageProject {
        name: package.name.clone(),
        manifest_path: package.manifest_path.clone(),
        target_directory: metadata.target_directory,
        metadata: package.metadata.clone(),
    })
}

/// 언어 설정이 필요 없는 기능도 사용할 수 있는 패키지 선택 결과.
/// Cargo metadata를 한 번 해석한 뒤 기능별 설정만 별도로 읽는다.
pub struct PackageProject {
    /// Cargo에 전달할 패키지 이름. 산출물도 이 이름으로 격리한다.
    pub name: String,
    /// 선택 패키지의 매니페스트. 자산 상대 경로의 기준이다.
    pub manifest_path: PathBuf,
    /// 사용자 Cargo 설정이 반영된 공통 산출물 경로.
    pub target_directory: PathBuf,
    /// 기능별 설정을 추출할 원본 metadata. 다른 도구의 설정은 해석하지 않는다.
    pub metadata: serde_json::Value,
}

/// 기존 i18n 실행에 필요한 언어 설정을 추가로 검증한다.
/// 일반 패키지 선택과 분리하여 render는 i18n 설정 없이도 실행할 수 있다.
pub fn load(options: &Options) -> Result<Project> {
    let package = load_package(options)?;
    let config = package.i18n_config()?;
    Ok(Project {
        name: package.name,
        manifest_path: package.manifest_path,
        target_directory: package.target_directory,
        config,
    })
}

impl PackageProject {
    /// i18n 기능 또는 render의 명시적인 언어 선택에 필요한 지원 선언을 읽고 검증한다.
    /// 설정 누락이나 잘못된 코드·중복은 문서 생성 전에 오류로 반환한다.
    pub fn i18n_config(&self) -> Result<I18nConfig> {
        let value = self
            .metadata
            .pointer("/textus/i18n")
            .context("missing [package.metadata.textus.i18n]; set languages = [\"en\", \"ko\"]")?;
        let config: I18nConfig = serde_json::from_value(value.clone())
            .context("invalid [package.metadata.textus.i18n] configuration")?;
        config.validate()?;
        Ok(config)
    }
}

/// 명시한 패키지 이름을 우선하고, 생략 시 매니페스트 위치로 대상을 결정한다.
///
/// 매니페스트 옵션도 없으면 현재 디렉토리부터 조상을 탐색한다. 정규화한 경로가
/// 구성원과 일치하지 않아도 구성원이 하나면 선택할 수 있다. 여러 구성원이 남으면
/// 임의 선택 대신 `--package`를 요구한다. 경로 확인 실패도 오류로 반환한다.
fn select_package<'a>(members: &'a [Package], options: &Options) -> Result<&'a Package> {
    if let Some(name) = &options.package {
        return members
            .iter()
            .find(|package| &package.name == name)
            .with_context(|| format!("workspace package {name:?} not found"));
    }
    let manifest = match &options.manifest_path {
        Some(path) => path.clone(),
        None => {
            let current =
                std::env::current_dir().context("could not determine current directory")?;
            current
                .ancestors()
                .map(|path| path.join("Cargo.toml"))
                .find(|path| path.is_file())
                .context("could not locate Cargo.toml")?
        }
    };
    let manifest = manifest
        .canonicalize()
        .context("could not resolve Cargo.toml path")?;
    if let Some(package) = members
        .iter()
        .find(|package| package.manifest_path.canonicalize().ok().as_ref() == Some(&manifest))
    {
        return Ok(package);
    }
    if let [package] = members {
        return Ok(package);
    }
    bail!(
        "select one workspace package with --package; available packages: {}",
        members
            .iter()
            .map(|package| package.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_rejects_invalid_duplicate_and_empty_languages() {
        for languages in [vec![], vec!["ko", "ko"], vec!["zz"], vec!["ko-KR"]] {
            let config = I18nConfig {
                languages: languages.into_iter().map(String::from).collect(),
            };
            assert!(config.validate().is_err());
        }
        let config = I18nConfig {
            languages: vec!["en".into(), "ko".into()],
        };
        config.validate().unwrap();
        config.require_language("ko").unwrap();
        assert!(config.require_language("ja").is_err());
    }
}
