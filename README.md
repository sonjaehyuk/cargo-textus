# cargo-textus

**For Rustaceans who are truly serious about writing.**

## What is this?

If you are not a native English speaker, you will likely find it inconvenient that much of the Reference material, including docs.rs, is only available in English. This is because most of the Reference is generated through comments (`///`, `//!` in Rust), and providing i18n in such an environment is practically challenging.

**cargo-textus** is a tool for managing localized documentation and some additional documentation rendering features for Rust projects. Specifically, they are as follows:
* You can set additional languages in Cargo.toml. Then, when you call cargo-textus macros for documentation, you can use CLI commands to build documentation specifically for those additional languages (while the default documentation remains intact, of course).
* You can render **Mermaid** diagrams in your documentation.
* You can render **mathematical formulas** in your documentation.
* You can use the **blockquote highlights** provided like GitHub in your documentation.


## Quickstart

저장소 루트에서 CLI를 설치합니다. 아래는 소스 checkout에서 설치하는 방법입니다.

```bash
cargo install --path crates/cargo-textus
cargo textus open --lang ko --manifest-path examples/demo/Cargo.toml
```

설치 없이 예제를 빌드할 수도 있습니다.

```bash
cargo run -p cargo-textus -- build --lang ko -p textus-demo
```

다른 Rust 프로젝트에서는 `cargo-textus`를 **일반 의존성**으로 등록하고,
지원하는 언어를 선언합니다. 아래 경로를 실제 checkout 경로로 바꾸세요.

```toml
[dependencies]
cargo-textus = { path = "/path/to/cargo-textus/crates/cargo-textus", default-features = false }

[package.metadata.textus.i18n]
languages = ["en", "ko"]
```

`cargo-textus` 패키지는 `cargo-textus` 바이너리와 `cargo_textus` 절차적 매크로
라이브러리를 함께 제공합니다. 기본 활성화되는 `cli` feature가 바이너리와 CLI 전용
의존성을 포함하므로 `cargo install`은 추가 옵션 없이 동작합니다. 매크로만 사용할
때는 위 예시처럼 `default-features = false`로 CLI 전용 의존성을 제외할 수 있습니다.

기존 `textus` 의존성은 `cargo-textus`로, `textus::include_doc!` 호출은
`cargo_textus::include_str!`로 변경하세요. CLI 명령 `cargo textus`, 설정의
`package.metadata.textus.i18n`, 문서 파일명 규칙은 동일합니다. 별도 `textus`
패키지는 더 이상 배포하지 않습니다.

## 문서 작성

```rust
#[doc = cargo_textus::include_str!("docs/guide.md")]
pub fn example() {}
```

```text
my-crate/
├── Cargo.toml
├── src/lib.rs
└── docs/
    ├── guide.md       # 일반 cargo build / doc / test에서 사용
    ├── guide.en.md    # --lang en
    └── guide.ko.md    # --lang ko
```

일반 Cargo 실행은 `docs/guide.md`를, 한국어를 선택한 실행은
`docs/guide.ko.md`를 사용합니다. 마지막 확장자 앞에 언어 코드를 삽입하므로
`api.guide.md`는 `api.guide.ko.md`가 됩니다. 기본 문서의 언어는 강제하지 않으며,
`--lang en`을 요청하면 기본 파일과 별개로 `.en.md` 파일이 필요합니다.

파일 경로는 **매크로를 호출하는 패키지의 `Cargo.toml` 기준**입니다.
일반 `include_str!`의 소스 파일 기준 경로와 다릅니다. 중첩 모듈에서도 같은
패키지 기준을 사용합니다. 문자열 리터럴 하나만 받으며, `/`로 구분한 상대 경로와
파일 확장자가 필요합니다. 절대 경로와 `..`는 허용하지 않습니다.

매크로는 선택한 경로를 기본 `include_str!`에 전달합니다. `doc` 속성, 일반 문서
주석, 기본 매크로를 변경하지 않습니다. 선택한 파일이 없으면 컴파일 오류가 나며
다른 언어로 자동 대체하지 않습니다. 파일 내용은 rustdoc의 Markdown으로 해석됩니다.

크레이트 설명, 함수, 타입, 필드 등 기존 `doc` 속성이 가능한 위치에서 사용합니다.

```rust
#![doc = cargo_textus::include_str!("docs/overview.md")]

#[doc = cargo_textus::include_str!("docs/greet.md")]
pub fn greet() -> &'static str {
    "Hello!"
}
```

## 디렉토리별 문서 작성

기존 `include_str!`는 파일명에 언어 코드를 삽입하는 기본 방식으로 유지합니다.
디렉토리를 나누려면 새 매크로에서 언어와 디렉토리를 명시적으로 연결합니다.

```rust
#[doc = cargo_textus::include_str_from_dir!(
    "docs/greet.md",
    ko = "docs/ko/",
    en = "translations/english",
)]
pub fn greet() {}
```

| 언어 선택 | 포함되는 파일 |
| --- | --- |
| 없음 | `docs/greet.md` |
| `--lang ko` | `docs/ko/greet.md` |
| `--lang en` | `translations/english/greet.md` |

기본 경로와 디렉토리는 모두 호출 패키지의 `Cargo.toml` 기준입니다.
디렉토리에 기본 파일의 **파일명 전체**를 붙입니다. 예를 들어 기본 경로가
`docs/api/greet.md`여도 `ko = "docs/ko"`는 `docs/ko/greet.md`를 선택합니다.
확장자 앞에 언어 코드를 추가하지 않습니다. 디렉토리 끝의 `/` 하나는 허용합니다.
절대 경로, 빈 디렉토리, `..`, 역슬래시, 빈 경로 구간은 허용하지 않습니다.
기본 파일 경로에는 기존 매크로와 동일하게 파일명과 확장자가 필요합니다.

최소 하나의 매핑이 필요하며, 키는 실제 배정된 소문자 ISO 639-1 코드입니다.
중복 키와 잘못된 코드·경로는 해당 언어를 선택하지 않아도 컴파일 오류입니다.
선택한 언어의 매핑이 없거나 선택한 파일이 없으면 오류를 내며 자동 대체하지 않습니다.
언어 미선택 시에는 기본 파일만 포함합니다. 선택하지 않은 파일의 존재 여부는 검사하지 않습니다.

이 문법은 이번 구현에서 채택한 설계입니다. 언어 코드와 디렉토리 이름을 분리해
`ko = "translations/korean"`처럼 기존 문서 구조를 사용할 수 있게 했습니다.
매핑은 각 호출에 명시하며 metadata의 `languages`는 계속 CLI의 지원 언어 선언으로
사용합니다. CLI로 빌드하려면 해당 언어를 metadata에도 등록해야 합니다.
`examples/demo`에서 두 매크로를 함께 사용하는 예제를 확인할 수 있습니다.
두 방식 모두 rustdoc API 설명을 포함하며 별도 안내 문서를 여는 기능은 아닙니다.

## 전체 문서 렌더링

Mermaid와 `$수식$`을 별도 매크로 없이 전체 rustdoc API 페이지에서 렌더링할 수 있습니다.

```bash
cargo textus build
cargo textus open
cargo textus build --lang ko
# 저장소의 독립 예제
cargo run -p cargo-textus -- open -p textus-render-demo
```

일반 문서에 `mermaid` 코드 블록과 `$E = mc^2$`, `$$x^2$$`를 작성합니다.
textus는 공통 JS/CSS를 주입하고, 구문 해석은 브라우저의 Mermaid·KaTeX가 담당합니다.
라이브러리·폰트는 함께 배포하므로 생성된 문서를 오프라인에서도 열 수 있습니다.

```toml
[package.metadata.textus.render]
mermaid = true
math = true
alerts = true
css = ["docs/custom.css"]
js = ["docs/custom.js"]
```

설정은 선택 사항이며 기본적으로 Mermaid·수식·알림을 활성화합니다. 사용자 CSS/JS는 패키지
기준 파일이며 전체 페이지에 적용합니다. `--lang`을 지정할 때만 기존 i18n 설정이
필요합니다. 첫 구현은 한 패키지의 라이브러리 타깃을 지원합니다. Mermaid Tiny의
일부 다이어그램 종류는 지원하지 않으며 Markdown 처리 중 변형된 TeX는 복원하지 않습니다.
GitHub 형식의 `> [!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]`, `[!CAUTION]`
알림도 지원합니다. 인용문의 첫 줄에 마커만 쓰고 다음 줄부터 내용을 작성하세요.
`alerts = false`로 끌 수 있으며, 일반 인용문과 코드 속 마커는 보존합니다.

[사용법·설계·제한 및 검증 방법](docs/rustdoc-rendering-plan.md)을 확인하세요.

## CLI

명령은 **할 일**, `--lang`은 **문서 선택**, `metadata.textus.render`는 **표시 방식**입니다.
i18n과 렌더링을 별도로 조합할 필요 없이 모든 생성 명령이 같은 설정을 적용합니다.

```bash
cargo textus build
cargo textus open
cargo textus check
cargo textus languages
cargo textus open --lang ko
cargo textus check --lang ko
```

| 명령 | 동작 |
| --- | --- |
| `build` | 기본 문서를 렌더링 설정과 함께 생성 |
| `open` | 기본 문서를 생성하고 자산 배치 후 브라우저로 열기 |
| `check` | 기본 문서부터 모든 등록 언어를 순서대로 빌드 검사 |
| `languages` | 등록된 추가 언어만 선언 순서대로 출력. 설정이 없으면 빈 결과로 성공 |
| `build/open/check --lang ko` | 한국어 문서만 대상으로 수행 |

언어 설정이 없는 프로젝트도 기본 문서를 생성·검사할 수 있습니다. 설정을 작성했다면
잘못된 코드·중복·빈 목록은 오류입니다. `--lang`은 등록된 언어만 허용하며
`languages`에는 사용할 수 없습니다. 기본 문서의 언어를 영어 등으로 추정하지 않습니다.

`check`도 산출물을 생성하며 첫 실패에서 멈춥니다. doctest, 번역 완전성 또는 브라우저
JavaScript의 렌더링 성공을 검사하지 않습니다. 활성 Rust 항목이 참조하는 문서를
검사하므로 비활성 `cfg`와 선택하지 않은 feature의 문서는 검사 대상이 아닙니다.

공통 옵션은 `--manifest-path PATH`, `--package NAME` (`-p NAME`), `--offline`,
`--locked`, `--help`입니다. 이름과 값은 공백으로 구분합니다. 가상 워크스페이스에
여러 패키지가 있으면 `-p`로 하나를 선택합니다. 브라우저는 Cargo의 `BROWSER` 또는
`doc.browser` 설정을 따릅니다. `--help`는 설정 로딩 없이 의도와 동작 범위를 설명합니다.

이전 명령은 별칭으로 실행하지 않습니다. 호출 스크립트도 함께 바꾸세요.

| 이전 | 현재 |
| --- | --- |
| `cargo textus i18n build --lang ko` | `cargo textus build --lang ko` |
| `cargo textus render open --lang ko` | `cargo textus open --lang ko` |
| `cargo textus i18n check` | `cargo textus check` — 이제 기본 문서도 검사 |
| `cargo textus i18n list` | `cargo textus languages` |
| `cargo textus build --open` | `cargo textus open` |

## 언어와 산출물

- 언어 코드는 실제 배정된 소문자 ISO 639-1 코드만 허용합니다. 예: `ko`, `en`, `ja`.
- `zz`, `kor`, `ko-KR`, `KO`는 거부합니다. 유효해도 설정에 없는 언어는 별도 오류입니다.
- 언어 설정은 선택 사항입니다. 작성한 `languages` 목록은 비어 있거나 중복될 수 없습니다.
- 설정은 지원 의사 표시입니다. 모든 매크로 호출에 해당 언어 파일을 작성해야 합니다.
- textus의 기본 문서는 `target/textus/<패키지명>/default/doc`에, 언어별 문서는
  `target/textus/<패키지명>/ko/doc`에 분리합니다. 일반 `cargo doc`의 `target/doc`은 유지합니다. 실제 기준 디렉터리는 `cargo metadata`의
  `target_directory`이므로 사용자 지정 target 디렉터리도 반영합니다.
- Cargo에 빌드 타깃이 설정되어 있으면 타깃 triple 등의 하위 디렉터리가 추가될 수 있습니다.

## 내부 동작과 초안 범위

1. CLI가 `cargo metadata --format-version 1 --no-deps`로 패키지와 설정을 읽습니다.
2. 선택한 언어를 검증하고, 자식 Cargo 프로세스에만 `TEXTUS_LANG=ko`를 전달합니다.
3. 전용 target 디렉터리에서 `cargo clean --doc` 후 공통 헤더를 지정한 `cargo rustdoc --lib`를 실행합니다.
4. 매크로가 파일명 접미사 또는 명시한 언어별 디렉토리로 경로를 선택하고 기본 `include_str!`로 확장됩니다.
5. rustdoc이 HTML을 생성하면 공통 자산을 배치합니다. 브라우저에서 Mermaid·KaTeX·알림을 처리합니다.

언어는 매크로 **크레이트 컴파일 시점**의 `option_env!("TEXTUS_LANG")`로 읽습니다.
Cargo가 환경변수 변경을 추적하므로 같은 target 디렉터리에서 언어를 바꾸거나
기본 문서로 돌아가도 매크로와 소비 크레이트를 다시 빌드합니다. 파일 내용 변경은
생성된 `include_str!`가 rustc에 전달합니다. 다만 Cargo의 rustdoc 캐시가 삭제된
문서 파일을 놓치는 사례를 통합 테스트에서 확인했으므로, CLI는 실행마다 해당
패키지·언어의 **생성된 문서만** 비우고 다시 생성합니다. Rust 의존성 빌드 캐시와
일반 Cargo 문서와 다른 패키지·언어의 산출물은 유지합니다. 확장 시점의 추적되지 않는 환경변수 읽기나
nightly 전용 API에 의존하지 않습니다.

`TEXTUS_LANG`는 내부 전달 규약입니다. 직접 설정하면 일반 Cargo 빌드에도 영향을
주므로 기본 문서로 돌아갈 때는 해제해야 합니다. CLI는 부모 프로세스 환경이나
사용자 소스를 수정하지 않습니다. 언어 미지정 CLI 실행은 자식의 `TEXTUS_LANG`을 제거합니다.

첫 버전은 한 번에 한 패키지, Cargo의 기본 feature 선택을 지원합니다.
별도 HTML 문서 열기, 자동 번역, 언어별 rustdoc UI 번역, 임의의 Cargo 옵션 전달,
워크스페이스 전체 일괄 처리, 패키지별로 서로 다른 언어를 동시에 선택하는 기능은
구현하지 않았습니다. 의존성 API 문서는 생성하지 않지만, 의존성 자체에서 textus
매크로를 사용하면 그 컴파일에도 같은 언어 선택이 적용됩니다.

파일명 규칙과 metadata 스키마는 이번 초안의 선택이며 향후 조정할 수 있습니다.

## 개발과 검증

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo clippy -p cargo-textus --all-targets --no-default-features -- -D warnings
cargo test -p cargo-textus --no-default-features
python3 -m unittest discover -s scripts/tests
```

통합 테스트는 실제 Cargo/rustdoc을 실행하므로 Rust 도구 체인과 의존성 캐시가
필요합니다. 내부 Cargo 명령은 `--offline`으로 실행합니다. 한국어 생성, 일반
`include_str!` 보존, 언어 전환, 파일 수정 및 누락, CLI와 패키지 선택을 확인합니다.
Unix에서는 테스트용 브라우저로 `open`의 전달 경로와 자산 배치 순서를 검사합니다.
매크로 전용 테스트는 `--no-default-features`에서도 실행됩니다. 절차적 매크로
라이브러리는 일반 공개 함수·타입 API를 내보내지 않으며 공통 로직은 `textus-core`에 둡니다.

| 디렉터리 | 책임 |
| --- | --- |
| `crates/cargo-textus` | CLI 바이너리와 `cargo_textus::include_str!` 절차적 매크로 라이브러리 |
| `crates/textus-core` | 언어 코드 검증과 문서 경로 선택 |
| `examples/demo` | 기본·영어·한국어 문서가 있는 실행 예제 |

언어 표는 시스템 `iso-codes`의 `iso_639-2.json`에서 `alpha_2` 필드를 추출했습니다
(2026-09-12, 183개). 출처는 [ISO 639 등록 기관의 언어 코드 목록](https://www.loc.gov/standards/iso639-2/php/code_list.php)이며,
목록 갱신 시 출처와 변경 내용을 함께 검토합니다.

설계 근거: [Rust 속성 내 매크로 확장](https://doc.rust-lang.org/reference/attributes.html#meta-item-attribute-syntax),
[Cargo 외부 명령](https://doc.rust-lang.org/cargo/reference/external-tools.html#custom-subcommands).

## 배포

Forgejo Actions에서 **Publish to crates.io**를 수동 실행하여 배포할 수 있습니다.
러너 라벨은 `intensive-distro`이며, 인증은 `CARGO_REGISTRY_TOKEN` Secret을 사용합니다.
설정 방법과 dry-run, 버전 갱신, 부분 배포 후 재실행은 [배포 가이드](docs/publishing.md)를 참고하세요.
