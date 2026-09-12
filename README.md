# cargo-textus

For Rustaceans who are truly serious about writing.

사용자가 직접 작성한 언어별 문서를 Rust API 문서에 포함하는 Cargo 도구의 초안입니다.
자동 번역을 수행하지 않습니다. Stable Rust에서 동작하며 Rust 1.95.0으로 검증했습니다.

## 빠른 시작

저장소 루트에서 CLI를 설치합니다. 아래는 소스 checkout에서 설치하는 방법입니다.

```bash
cargo install --path crates/cargo-textus
cargo textus i18n open --lang ko --manifest-path examples/demo/Cargo.toml
```

설치 없이 예제를 빌드할 수도 있습니다.

```bash
cargo run -p cargo-textus -- i18n build --lang ko -p textus-demo
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
`cargo_textus::include_doc!`로 변경하세요. CLI 명령 `cargo textus`, 설정의
`package.metadata.textus.i18n`, 문서 파일명 규칙은 동일합니다. 별도 `textus`
패키지는 더 이상 배포하지 않습니다.

## 문서 작성

```rust
#[doc = cargo_textus::include_doc!("docs/guide.md")]
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
#![doc = cargo_textus::include_doc!("docs/overview.md")]

#[doc = cargo_textus::include_doc!("docs/greet.md")]
pub fn greet() -> &'static str {
    "Hello!"
}
```

## CLI

```bash
cargo textus i18n list
cargo textus i18n check
cargo textus i18n check --lang ko
cargo textus i18n build --lang ko
cargo textus i18n build --lang ko --open
cargo textus i18n open --lang ko
```

| 명령 | 동작 |
| --- | --- |
| `list` | 패키지 설정에 등록된 언어 출력. 파일 존재 여부는 검사하지 않음 |
| `check` | 모든 등록 언어로 실제 rustdoc 빌드 수행 |
| `check --lang ko` | 한국어 rustdoc 빌드로 파일 포함과 매크로 확장 검사 |
| `build --lang ko` | 한국어 API 문서 생성 |
| `open --lang ko` | 빌드 성공 후 Cargo의 `--open`으로 문서 열기 |

`check`도 산출물을 생성하며 doctest를 실행하거나 번역의 완전성을 판단하지 않습니다.
활성화된 Rust 항목의 매크로가 참조하는 파일을 검사합니다. 비활성 `cfg` 항목이나
선택되지 않은 Cargo feature의 문서는 검사 대상이 아닙니다.

공통 옵션은 `--manifest-path PATH`, `--package NAME` (`-p NAME`), `--offline`,
`--locked`, `--help`입니다. 옵션 값은 `--lang ko`처럼 공백으로 구분합니다.
가상 워크스페이스에 패키지가 여러 개 있으면 `-p`로 하나를 선택합니다.
일반 패키지 디렉터리에서는 해당 패키지를 자동 선택합니다.

브라우저는 Cargo의 `BROWSER` 또는 `doc.browser` 설정을 따릅니다.
브라우저 실행 실패 처리도 Cargo의 동작을 따릅니다.

## 언어와 산출물

- 언어 코드는 실제 배정된 소문자 ISO 639-1 코드만 허용합니다. 예: `ko`, `en`, `ja`.
- `zz`, `kor`, `ko-KR`, `KO`는 거부합니다. 유효해도 설정에 없는 언어는 별도 오류입니다.
- `languages`는 비어 있거나 중복될 수 없습니다. 각 패키지에 명시적으로 등록합니다.
- 설정은 지원 의사 표시입니다. 모든 매크로 호출에 해당 언어 파일을 작성해야 합니다.
- 기본 문서는 Cargo의 기본 `target/doc`에, 언어별 문서는
  `target/textus/ko/<패키지명>/doc`처럼 패키지·언어별로 분리합니다. 실제 기준 디렉터리는 `cargo metadata`의
  `target_directory`이므로 사용자 지정 target 디렉터리도 반영합니다.
- Cargo에 빌드 타깃이 설정되어 있으면 타깃 triple 등의 하위 디렉터리가 추가될 수 있습니다.

## 내부 동작과 초안 범위

1. CLI가 `cargo metadata --format-version 1 --no-deps`로 패키지와 설정을 읽습니다.
2. 선택한 언어를 검증하고, 자식 Cargo 프로세스에만 `TEXTUS_LANG=ko`를 전달합니다.
3. 전용 target 디렉터리에서 `cargo clean --doc` 후 `cargo doc --no-deps`를 실행합니다.
4. `cargo_textus::include_doc!`가 `.ko` 파일명을 생성하고 기본 `include_str!`로 확장됩니다.
5. rustdoc이 사용자가 작성한 설명과 Rust API 정보를 합쳐 HTML을 생성합니다.

언어는 매크로 **크레이트 컴파일 시점**의 `option_env!("TEXTUS_LANG")`로 읽습니다.
Cargo가 환경변수 변경을 추적하므로 같은 target 디렉터리에서 언어를 바꾸거나
기본 문서로 돌아가도 매크로와 소비 크레이트를 다시 빌드합니다. 파일 내용 변경은
생성된 `include_str!`가 rustc에 전달합니다. 다만 Cargo의 rustdoc 캐시가 삭제된
문서 파일을 놓치는 사례를 통합 테스트에서 확인했으므로, CLI는 실행마다 해당
패키지·언어의 **생성된 문서만** 비우고 다시 생성합니다. Rust 의존성 빌드 캐시와
기본 문서, 다른 패키지·언어의 산출물은 유지합니다. 확장 시점의 추적되지 않는 환경변수 읽기나
nightly 전용 API에 의존하지 않습니다.

`TEXTUS_LANG`는 내부 전달 규약입니다. 직접 설정하면 일반 Cargo 빌드에도 영향을
주므로 기본 문서로 돌아갈 때는 해제해야 합니다. CLI는 부모 프로세스 환경이나
사용자 소스를 수정하지 않습니다.

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
Unix에서는 테스트용 브라우저로 `open`과 `build --open`의 전달 경로도 검사합니다.
매크로 전용 테스트는 `--no-default-features`에서도 실행됩니다. 절차적 매크로
라이브러리는 일반 공개 함수·타입 API를 내보내지 않으며 공통 로직은 `textus-core`에 둡니다.

| 디렉터리 | 책임 |
| --- | --- |
| `crates/cargo-textus` | CLI 바이너리와 `cargo_textus::include_doc!` 절차적 매크로 라이브러리 |
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
