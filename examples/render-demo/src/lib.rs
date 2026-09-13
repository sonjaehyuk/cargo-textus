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

/// > [!TIP]
/// > Function-page alert.
///
/// 함수 페이지에서도 $a + b$를 렌더링합니다.
///
/// ```mermaid
/// sequenceDiagram
///     Alice->>Bob: Hello
/// ```
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
    pub struct Example {
        /// 필드의 수식 $n + 1$도 같은 페이지에서 처리합니다.
        pub value: usize,
    }
}
