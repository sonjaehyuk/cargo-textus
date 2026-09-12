# cargo-textus

For Rustaceans who are truly serious about writing.

사용자가 직접 작성한 언어별 문서를 Rust API 문서에 포함하는 Cargo 도구의 초안입니다.
자동 번역을 수행하지 않습니다.

```rust,ignore
#[doc = textus::include_doc!("docs/guide.md")]
pub fn example() {}
```

일반 Cargo 실행은 `docs/guide.md`를, 한국어를 선택한 실행은
`docs/guide.ko.md`를 사용합니다. 파일 경로는 매크로를 호출하는 패키지의
`Cargo.toml` 기준입니다. 매크로는 선택한 경로를 기본 `include_str!`에
전달하며 `doc` 속성, 일반 문서 주석, 기본 매크로를 변경하지 않습니다.
선택한 파일이 없으면 컴파일 오류가 발생합니다.

실행 가능한 예제는 `examples/demo`에 있습니다.
