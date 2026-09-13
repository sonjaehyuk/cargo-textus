//! ISO 639-1 validation and localized document filenames.

/// Assigned ISO 639-1 codes, stored as a sorted, whitespace-separated table.
/// Source: iso-codes `iso_639-2.json` (alpha_2 fields), maintained from the
/// Library of Congress ISO 639-2 registration authority's alpha-2 mappings.
/// <https://www.loc.gov/standards/iso639-2/php/code_list.php>
const LANGUAGE_CODES: &str = include_str!("iso-639-1.txt");

/// Accept only assigned, lowercase ISO 639-1 codes (not regional tags).
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

/// Select `guide.ko.md` for `guide.md` and `ko`, or preserve the default path.
/// Paths are portable, package-relative paths using `/` separators.
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

/// Select the default file, or its basename within an explicitly mapped directory.
/// All paths are package-relative; mappings are validated even when unselected.
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
        // A single trailing slash is accepted for directory notation.
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
