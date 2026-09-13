# 전체 rustdoc 렌더링

상태: 작업 중심 CLI로 통합한 전체 페이지 렌더링 구현. 작업 브랜치는 `rustdoc-rendering`이다.
이전 계획의 `doc_assets!`·`render_doc!` 개별 문서 매크로는 채택하지 않았다.

## 책임 분리

사용자의 방향에 따라 textus는 Markdown이나 TeX를 직접 파싱하지 않는다.
Cargo metadata로 패키지를 선택하고 공통 헤더를 rustdoc에 전달하며 브라우저 자산을
배치한다. rustdoc이 생성한 각 API 페이지에서 Mermaid와 KaTeX가 내용을 렌더링한다.

Mermaid는 원래 `.mermaid` 요소를 인식한다. rustdoc의 `mermaid` 코드 블록은
`pre.language-mermaid`로 생성되므로 초기화 코드가 클래스와 코드 컨테이너만 연결한다.
다이어그램 구문 해석과 SVG 생성은 `mermaid.run`이 수행한다. 수식 탐색은
KaTeX의 `renderMathInElement`가 수행하며 자체 수식 파서는 두지 않는다.

참고: [rustdoc HTML 주입 옵션](https://doc.rust-lang.org/rustdoc/command-line-arguments.html#--html-in-header-include-more-html-in-head),
[Mermaid 사용법과 run API](https://mermaid.js.org/config/usage.html),
[KaTeX 자동 렌더 설정](https://katex.org/docs/autorender.html).

## 사용법

```bash
cargo textus build
cargo textus open
cargo textus check
cargo textus languages
cargo textus build --lang ko
# 저장소 예제: 별도 매크로나 cargo-textus 의존성이 없는 라이브러리
cargo run -p cargo-textus -- open -p textus-render-demo
```

첫 구현은 한 패키지의 **라이브러리 타깃 전체 API 페이지**를 생성한다.
크레이트·모듈·함수·타입·필드 문서에 공통 초기화 코드가 들어간다. 의존성의 API 문서,
바이너리만 있는 패키지, 워크스페이스 전체 일괄 생성은 현재 범위가 아니다.
`--manifest-path`, `--package`/`-p`, `--offline`, `--locked`를 지원한다.
생성 작업은 `build`·`open`·`check`로 통일했다. 일반 `cargo doc`에는 자동 적용되지 않는다.

일반 주석 또는 기존에 포함하던 Markdown에 다음처럼 작성한다.

````rust
/// 흐름은 다음과 같다.
/// ```mermaid
/// flowchart LR
///     A --> B
/// ```
/// 인라인 수식은 $E = mc^2$이다.
///
/// $$x^2 + y^2 = z^2$$
pub fn example() {}
````

기본적으로 Mermaid·수식·알림이 활성화되며 설정은 선택 사항이다.

```toml
[package.metadata.textus.render]
mermaid = true
math = true
alerts = true
css = ["docs/custom.css"]
js = ["docs/custom.js"]
```

CSS/JS 경로는 호출 패키지의 `Cargo.toml` 기준이다. 절대 경로, `..`, 빈 구간,
역슬래시·콜론을 거부하며 파일이 없으면 빌드 전에 오류를 낸다.
파일은 생성 디렉토리의 고정 이름으로 복사한다. 사용자 CSS 내부의 상대 `url()`이나
JS의 상대 import 및 추가 파일은 자동 수집·복사하지 않는다. 첫 버전에서는 독립적인
CSS와 classic JS 파일을 사용한다. CSS는 페이지 전체에 적용되며, JS는 DOM 준비 및
내장 렌더링 시도 후 지정 순서대로 실행된다. 사용자 JS는 페이지 권한으로 실행된다.
알 수 없는 설정 키와 잘못된 값 타입은 오류다.

## GitHub 형식 알림

`alerts = true`가 기본값이며 `false`로 설정하면 원래 인용문을 그대로 표시한다.
CSS/JS를 추가로 작성할 필요 없이 전체 API 페이지의 문서 영역에 적용한다.

```markdown
> [!NOTE]
> 참고 정보입니다.

> [!TIP]
> 작업에 도움이 되는 팁입니다.

> [!IMPORTANT]
> 꼭 알아야 할 정보입니다.

> [!WARNING]
> 즉시 주의해야 하는 내용입니다.

> [!CAUTION]
> 위험이나 부정적인 결과를 안내합니다.
```

구현은 rustdoc이 생성한 최상위 `blockquote`의 첫 문단을 확인한다. 대문자 마커만
첫 줄에 독립적으로 있을 때 변환하고 제목(Note·Tip·Important·Warning·Caution),
자체 SVG 아이콘, 종류별 색상의 테두리를 표시한다. 아이콘은 스크린 리더에서 제외하고
텍스트 제목으로 종류를 전달한다. 정적 문서이므로 실시간 알림 역할(`role="alert"`)은 붙이지 않는다.
밝은 테마와 rustdoc의 dark·ayu 테마에 맞춰 색상을 변경하며 사용자 CSS가 재정의할 수 있다.

본문의 링크·강조·목록·코드·추가 문단은 원래 DOM을 유지한다. 코드로 감싼 마커,
지원하지 않는 종류, 소문자 마커, 같은 줄에 본문을 덧붙인 마커, 중첩 인용문이나 목록
내부의 알림은 변환하지 않는다. Markdown escape는 rustdoc 단계에서 사라질 수 있으므로
마커를 문자 그대로 보이려면 인라인 코드로 감싼다. GitHub의 서버 렌더러를 그대로
가져온 것은 아니며 [GitHub 알림 문법](https://docs.github.com/en/enterprise-cloud%40latest/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax#alerts)을
브라우저 DOM에서 처리하는 방식이다. JS를 끄면 원래 마커와 인용문이 보인다.

처리 순서는 알림 DOM 변환 → Mermaid·KaTeX → 사용자 JS이다. 알림 안의 그림과
수식에도 기존 렌더러가 적용된다. CSS는 사용자 CSS보다 먼저 로드한다. 별도의 외부 자산이나
추가 의존성 없이 작동한다. 브라우저 테스트에서 다섯 종류, 일반 인용문 보존, 중첩 페이지,
서식 보존, 토글 비활성화와 테마 전환을 검증한다.

## i18n 조합과 빌드

`build --lang ko`는 기존 언어 코드 및 지원 언어 등록을 검증한 다음,
자식 Cargo에 `TEXTUS_LANG=ko`를 전달한다. 기존 i18n 매크로가 문서 파일을 선택하고
생성된 모든 API 페이지에 같은 렌더링 헤더가 적용된다. 언어 미지정 시 i18n metadata가
필요 없으며 부모 환경의 `TEXTUS_LANG`도 자식에서 제거해 기본 문서를 사용한다.
기존 i18n 매크로 동작은 유지한다. 이전 i18n/render 명령 계층은 제거했다.
`check`는 기본 문서와 모든 등록 언어를, `check --lang ko`는 한국어만 같은 경로로 검사한다.
`languages`는 추가 언어 목록만 출력하며 설정이 없으면 빈 결과로 성공한다.

산출물은 `<Cargo target>/textus/<패키지>/<언어 또는 default>/doc`에 생성한다.
Cargo 빌드 타깃 설정에 따라 `doc` 앞에 triple 경로가 추가될 수 있다.
공통 자산은 각 문서 루트의 `textus-assets/`에 한 번만 복사하며 각 페이지는
rustdoc의 `data-root-path`를 이용해 중첩 깊이에 맞는 경로로 로드한다.
HTML 파일을 공유할 때는 `doc` 디렉토리 전체를 함께 전달해야 한다.

`cargo rustdoc --lib -- --html-in-header ...`로 선택 패키지에만 헤더를 적용한다.
사용자의 기존 rustdoc 환경 플래그는 덮어쓰지 않는다. 공백이 있는 경로도 인자로
전달한다. 삭제된 포함 파일과 설정 변경을 재검사하려고 전용 문서 산출물만 정리한다.
`open`은 첫 빌드와 자산 배치 후 같은 Cargo 명령을 `--open`으로 다시 호출해
브라우저 선택을 Cargo에 맡긴다. 원본 문서나 Cargo 설정 파일은 수정하지 않는다.

## 브라우저 동작과 제한

- 페이지의 `.docblock` 전체가 대상이다. 검색 UI와 코드 예제는 수식 탐색 대상이 아니다.
- KaTeX 자동 렌더에 `$$...$$`, `$...$` 순서로 구분자를 설정했다. 코드·pre 등은
  라이브러리의 기본 제외 규칙을 따르며 `.textus-no-math`도 제외한다.
- 일반 달러 표기와 수식 구분은 KaTeX 규칙을 따른다. 통화 표기처럼 달러가 반복되는
  문장은 코드나 `<span class="textus-no-math">...</span>`로 감쌀 수 있다.
- Markdown 처리 후의 텍스트를 읽으므로 역슬래시·강조 구문으로 변형된 TeX를
  복원하지 않는다. 필요한 경우 Markdown escape나 원문을 보존하는 HTML을 사용하고
  실제 생성 결과를 확인해야 한다. 모든 LaTeX 구문 지원을 약속하지 않는다.
- Mermaid는 `strict`, KaTeX는 `trust: false`로 초기화한다. 수식 오류는 KaTeX의 오류
  표시를 사용한다. Mermaid 및 자산 로딩 오류는 브라우저 콘솔에서 확인한다.
- JS는 정적으로 생성된 문서에 한 번 적용한다. 사용자 JS로 나중에 추가한 콘텐츠의
  자동 재처리와 테마 전환 시 다이어그램 재생성은 아직 제공하지 않는다.
- Mermaid Tiny 12.0.0을 사용한다. 일반 흐름도와 시퀀스 등을 지원하지만 mindmap,
  architecture, Mermaid 내부 KaTeX, lazy loading과 ELK는 제공하지 않는다.
  본문 수식은 별도의 KaTeX 0.18.7이 처리한다.
- 로컬 자산만 사용하므로 CDN 연결이 필요 없다. Chromium 140과 153에서 `file://`와 HTTP를
  검증했다. 다른 브라우저 및 docs.rs/CSP 제한 환경은 아직 검증하지 않았다.

## 자산 출처와 갱신

`assets/render`의 JS/CSS/폰트는 npm `@mermaid-js/tiny@12.0.0`과 `katex@0.18.7`의
배포 파일을 수정 없이 복사한 것이다. 각 패키지의 MIT 라이선스와 번들 내부 저작권
표기를 보존한다. 해당 파일은 프로젝트의 한국어 문서 주석 규칙으로 재작성하지 않는다.
`SHA256SUMS`는 복사된 파일의 무결성 비교에 사용한다.

```bash
npm ci --prefix scripts/render-assets --ignore-scripts
python3 scripts/render-assets/vendor.py
cargo fmt --all
```

버전 갱신 시 package.json과 lockfile, 생성된 Rust 자산 목록, 실제 자산을 함께 갱신한다.
새 버전이 파일을 제거한 경우 남은 이전 파일도 검토한다. 지원 구문·라이선스·브라우저
호환성을 재검증한다. Cargo 사용자에게 Node나 npm 설치를 요구하지 않는다.

## 검증과 다음 범위

Rust 테스트는 CLI 구분, 설정 검증, 기존 i18n 회귀, 전체 페이지 헤더 주입,
기존 rustdoc 플래그 보존, 공백 경로와 사용자 자산 누락 오류를 검사한다.
브라우저 테스트는 매크로 없는 예제의 크레이트·함수·중첩 타입 페이지에서 실제 SVG,
수식 DOM, 사용자 CSS/JS, 코드 영역 보존 및 외부 요청 없음을 확인한다.

```bash
cargo test --workspace
cargo test -p cargo-textus --no-default-features
cargo run -p cargo-textus -- build -p textus-render-demo --offline
npm ci --prefix scripts/render-browser
npx --prefix scripts/render-browser playwright install chromium
node scripts/render-browser/test.cjs
```

다음 확장 후보는 전체 Mermaid 배포 지원, 런타임 설정 노출, 타깃 선택과 테마 변경
대응이다. 어느 경우에도 개별 문서를 textus에서 파싱하는 구조로 되돌리지 않는다.
