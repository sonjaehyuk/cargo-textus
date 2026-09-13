//! 언어 식별자와 문서 경로를 검증하는 순수 함수 모음.
//!
//! 파일명 접미사 방식과 명시적 디렉토리 방식이 같은 언어 표를 사용한다.
//! 경로 검증은 문자열 규칙 검사이며 파일 존재 여부나 심볼릭 링크 대상은 검사하지 않는다.

/// 실제 배정된 ISO 639-1 코드를 공백으로 구분해 정렬한 검증용 표.
///
/// `iso-codes`의 `iso_639-2.json`에서 `alpha_2` 필드를 추출한 데이터를 포함한다.
/// 형식만 맞는 미배정 코드를 허용하지 않기 위해 외부 조회 없이 이 표와 비교한다.
/// 출처는 미국 의회도서관 ISO 639-2 등록 기관의 두 글자 코드 대응표다.
/// <https://www.loc.gov/standards/iso639-2/php/code_list.php>
/// 갱신 시 데이터의 출처와 변경 내용을 함께 검토해야 한다.
const LANGUAGE_CODES: &str = include_str!("iso-639-1.txt");

/// 입력이 실제 배정된 소문자 ISO 639-1 코드인지 검사한다.
///
/// `ko`와 `en`은 허용하지만 `KO`, `kor`, `ko-KR`, 미배정 코드 `zz`는 거부한다.
/// 대소문자 변환이나 지역 태그 축약을 하지 않아 사용자 입력의 의미를 조용히 바꾸지 않는다.
/// 프로젝트의 지원 언어 등록 여부는 별도 검사이며 여기서는 언어 표의 유효성만 판단한다.
///
/// # 오류
///
/// 표에 없는 값이면 잘못된 코드와 허용 형식의 예시를 포함한 메시지를 반환한다.
pub fn validate_language(language: &str) -> Result<(), String> {
    if LANGUAGE_CODES
        .split_whitespace()
        .any(|code| code == language)
    {
        Ok(())
    } else {
        Err(format!(
            "invalid ISO 639-1 language code {language:?}; use a lowercase code such as ko or en"
        ))
    }
}

/// 기본 파일의 마지막 확장자 앞에 언어 코드를 삽입한 경로를 반환한다.
///
/// `docs/api.guide.md`와 `Some("ko")`는 `docs/api.guide.ko.md`가 된다.
/// `None`이면 검증한 기본 경로를 그대로 반환한다. 파일을 읽지 않으므로 이 결과만으로
/// 문서 존재 여부는 알 수 없다. 호출자는 패키지 매니페스트 위치를 기준으로 해석한다.
///
/// # 오류
///
/// 빈 경로, 절대 경로, 역슬래시·콜론, 빈 경로 구간, `..`, 파일명 또는 확장자 누락을
/// 거부한다. 언어가 주어지면 코드도 검증한다. 유효한 파일이 있는 다른 언어를 탐색하거나
/// 자동 대체하는 정책은 포함하지 않는다.
pub fn document_path(path: &str, language: Option<&str>) -> Result<String, String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains(['\\', ':'])
        || path.split('/').any(|part| part.is_empty() || part == "..")
    {
        return Err(
            "document path must be a nonempty package-relative path using /, without ..".into(),
        );
    }
    let filename = path.rsplit('/').next().unwrap_or_default();
    let (stem, extension) = filename
        .rsplit_once('.')
        .filter(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
        .ok_or("document path must have a filename and extension, for example docs/guide.md")?;
    match language {
        None => Ok(path.to_owned()),
        Some(language) => {
            validate_language(language)?;
            let directory = &path[..path.len() - filename.len()];
            Ok(format!("{directory}{stem}.{language}.{extension}"))
        }
    }
}

/// 기본 파일 또는 언어에 명시적으로 연결된 디렉토리의 동일 파일명을 선택한다.
///
/// `directories`는 `(언어 코드, 패키지 기준 디렉토리)` 쌍의 목록이다.
/// `docs/api/guide.md`와 `("ko", "translations/korean/")`을 사용하면 한국어
/// 선택 결과는 `translations/korean/guide.md`다. 기본 경로의 상위 디렉토리는
/// 복사하지 않으며 디렉토리 끝의 `/` 하나를 제거한 뒤 파일명 전체를 붙인다.
///
/// 언어가 없으면 기본 파일을 선택하지만 모든 매핑의 유효성을 먼저 확인한다.
/// 이렇게 하면 언어를 전환해야만 설정 오타가 드러나는 일을 줄일 수 있다.
/// 선택하지 않은 파일의 존재 여부를 포함해 파일 시스템은 검사하지 않는다.
///
/// # 오류
///
/// 기본 경로는 [`document_path`]와 같은 규칙을 따른다. 빈 매핑 목록, 미배정 코드,
/// 중복 키, 빈 디렉토리, 절대 경로, 역슬래시·콜론, 빈 구간과 `..`를 거부한다.
/// 요청 언어의 유효성과 매핑 존재 여부는 별개로 검사하며, 유효하지만 매핑이 없는
/// 언어에는 등록 누락 오류를 반환한다. 기본 파일이나 다른 언어로 대체하지 않는다.
pub fn directory_document_path(
    path: &str,
    directories: &[(&str, &str)],
    language: Option<&str>,
) -> Result<String, String> {
    document_path(path, None)?;
    if directories.is_empty() {
        return Err("at least one language directory mapping is required".into());
    }
    for (index, &(code, directory)) in directories.iter().enumerate() {
        validate_language(code)?;
        if directories[..index]
            .iter()
            .any(|&(previous, _)| previous == code)
        {
            return Err(format!("duplicate language directory mapping for {code:?}"));
        }
        // 디렉토리임을 표현하는 마지막 슬래시 하나만 허용하고 중복 슬래시는 거부한다.
        let directory = directory.strip_suffix('/').unwrap_or(directory);
        if directory.is_empty()
            || directory.starts_with('/')
            || directory.contains(['\\', ':'])
            || directory
                .split('/')
                .any(|part| part.is_empty() || part == "..")
        {
            return Err(
                "language directory must be a nonempty package-relative path using /, without .."
                    .into(),
            );
        }
    }
    let Some(language) = language else {
        return Ok(path.to_owned());
    };
    validate_language(language)?;
    let directory = directories
        .iter()
        .find_map(|&(code, directory)| (code == language).then_some(directory))
        .ok_or_else(|| format!("no document directory registered for language {language:?}"))?;
    let filename = path.rsplit('/').next().unwrap();
    Ok(format!(
        "{}/{filename}",
        directory.strip_suffix('/').unwrap_or(directory)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_assigned_codes_only() {
        for code in ["ko", "en", "ja", "zh", "zu"] {
            assert!(validate_language(code).is_ok());
        }
        for code in ["", "zz", "KO", "kor", "ko-KR", "../ko"] {
            assert!(validate_language(code).is_err(), "{code}");
        }
    }

    #[test]
    fn table_is_sorted_unique_and_has_two_letter_codes() {
        let codes: Vec<_> = LANGUAGE_CODES.split_whitespace().collect();
        assert!(codes.len() > 180);
        assert!(codes.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            codes
                .iter()
                .all(|code| code.len() == 2 && code.bytes().all(|byte| byte.is_ascii_lowercase()))
        );
    }

    #[test]
    fn selects_only_the_filename_suffix() {
        assert_eq!(
            document_path("docs/guide.md", None).unwrap(),
            "docs/guide.md"
        );
        assert_eq!(
            document_path("docs.v1/api.guide.md", Some("ko")).unwrap(),
            "docs.v1/api.guide.ko.md"
        );
        assert_eq!(
            document_path("docs/안내.md", Some("ja")).unwrap(),
            "docs/안내.ja.md"
        );
    }

    #[test]
    fn rejects_ambiguous_or_nonportable_paths() {
        for path in [
            "",
            "/a.md",
            "../a.md",
            "docs/../a.md",
            "a",
            ".md",
            "a.",
            "a//b.md",
            "C:/a.md",
            "docs\\a.md",
        ] {
            assert!(document_path(path, Some("ko")).is_err(), "{path}");
        }
        assert!(document_path("a.md", Some("zz")).is_err());
    }
}

#[cfg(test)]
mod directory_tests {
    use super::directory_document_path as select;

    #[test]
    fn selects_basename_and_explicit_directory() {
        let mappings = [("ko", "docs/ko/"), ("en", "translations/english")];
        assert_eq!(
            select("docs/nested/api.guide.md", &mappings, None).unwrap(),
            "docs/nested/api.guide.md"
        );
        assert_eq!(
            select("docs/nested/api.guide.md", &mappings, Some("ko")).unwrap(),
            "docs/ko/api.guide.md"
        );
        assert_eq!(
            select("docs/안내.md", &mappings, Some("en")).unwrap(),
            "translations/english/안내.md"
        );
        assert!(
            select("a.md", &mappings, Some("ja"))
                .unwrap_err()
                .contains("no document directory registered")
        );
        assert!(
            select("a.md", &mappings, Some("zz"))
                .unwrap_err()
                .contains("invalid ISO 639-1")
        );
    }

    #[test]
    fn validates_all_mappings_even_without_language_selection() {
        assert!(select("a.md", &[], None).is_err());
        assert!(select("a.md", &[("ko", "ko"), ("ko", "other")], None).is_err());
        for code in ["zz", "KO", "kor", "ko-KR"] {
            assert!(select("a.md", &[(code, "ko")], None).is_err());
        }
        for directory in [
            "",
            "/",
            "/ko",
            "../ko",
            "docs/../ko",
            "docs//ko",
            "ko//",
            "C:/ko",
            "docs\\ko",
        ] {
            assert!(
                select("a.md", &[("ko", directory)], None).is_err(),
                "{directory}"
            );
        }
        assert!(select("../a.md", &[("ko", "ko")], None).is_err());
    }
}
