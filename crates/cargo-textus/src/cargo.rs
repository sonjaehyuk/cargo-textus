//! CLI 전용 Cargo 프로세스 생성 규칙을 모은다.
//! metadata 조회와 문서 빌드가 동일한 실행 파일 및 공통 옵션을 사용하도록 한다.

use std::process::Command;

use crate::cli::Options;

/// `CARGO` 환경변수의 실행 파일을 우선하고 없으면 PATH의 `cargo`를 사용한다.
///
/// 오프라인·잠금 파일 옵션만 공통으로 붙인 아직 실행하지 않은 명령을 반환한다.
/// 호출자가 하위 명령, 대상 경로, 언어 환경변수와 실행 방식을 결정한다.
pub fn command(options: &Options) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    if options.offline {
        command.arg("--offline");
    }
    if options.locked {
        command.arg("--locked");
    }
    command
}
