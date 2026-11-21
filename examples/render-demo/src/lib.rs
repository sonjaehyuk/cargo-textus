//! # 전체 문서 렌더링 예제
//!
//! 별도 textus 매크로 없이 $E = mc^2$를 표현합니다.
//!
//! ```mermaid
//! flowchart LR
//!     Markdown --> rustdoc --> Browser
//! ```
//!
//! $$x^2 + y^2 = z^2$$
//!
//! 인라인 코드 `$not_math$`와 일반 코드 블록은 보존됩니다.
//! ```text
//! $not_math$
//! ```

//! > [!NOTE]
//! > Useful **information** with a [link](https://example.com).
//!
//! > [!TIP]
//! > Helpful advice with `code`.
//!
//! > [!IMPORTANT]
//! > Key information.
//! >
//! > - First item
//! > - Second item
//!
//! > [!WARNING]
//! > Urgent information.
//!
//! > [!CAUTION]
//! > Risks and negative outcomes.
//!
//! > Ordinary quote.
//!
//! > [!UNKNOWN]
//! > An unsupported marker remains visible.
//!
//! > `[!NOTE]`
//! > A code marker is not an alert.
//!
//! > [!NOTE] Same-line content is not an alert.
//!
//! > Outer quote.
//! >
//! > > [!NOTE]
//! > > Nested markers are not alerts.
//!
//! > [!NOTE]
//! >
//! > Separate paragraph.
//!
//! 각주 미리보기[^preview]와 같은 각주를 다시 가리키는 표시[^preview]입니다.
//! 긴 각주도 끝까지 스크롤할 수 있습니다[^long].
//!
//! [^preview]: Footnote **formatting**, `inline code`, and a [link](https://example.com).
//!
//!     Second paragraph with $a + b$.
//!
//!     ```mermaid
//!     flowchart LR
//!         Footnote --> Preview
//!     ```
//!
//! [^long]: A long footnote with several paragraphs.
//!
//!     1. First item.
//!     2. Second item.
//!
//!     ```text
//!     line 01
//!     line 02
//!     line 03
//!     line 04
//!     line 05
//!     line 06
//!     line 07
//!     line 08
//!     line 09
//!     line 10
//!     line 11
//!     line 12
//!     line 13
//!     line 14
//!     line 15
//!     line 16
//!     line 17
//!     line 18
//!     line 19
//!     line 20
//!     line 21
//!     line 22
//!     line 23
//!     line 24
//!     ```
//!
//!     End of the long footnote.

/// > [!TIP]
/// > Function-page alert.
///
/// 함수 페이지에서도 $a + b$를 렌더링합니다.
///
/// ```mermaid
/// sequenceDiagram
///     Alice->>Bob: Hello
/// ```
///
/// 함수 각주[^function].
///
/// [^function]: Function-page footnote.
pub fn example() {}

/// 중첩된 모듈 페이지의 상대 자산 경로도 동일하게 처리합니다.
pub mod nested {
    /// > [!WARNING]
    /// > Nested-page alert.
    ///
    /// 타입 문서의 수식 $x^2$입니다.
    ///
    /// ```mermaid
    /// flowchart TD
    ///     A --> B
    /// ```
    ///
    /// 중첩 페이지 각주[^nested].
    ///
    /// [^nested]: Nested-page footnote.
    pub struct Example {
        /// 필드의 수식 $n + 1$도 같은 페이지에서 처리합니다.
        pub value: usize,
    }
}
