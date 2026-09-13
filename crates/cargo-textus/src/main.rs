//! Cargo 확장 바이너리의 진입점.
//! 인자 전달, 도움말 출력, 오류 출력과 종료 코드만 담당하고 기능 처리는 모듈에 위임한다.

mod cargo;
mod cli;
mod config;
mod document;
mod render;

/// 운영체제 인자를 파서에 전달하고 요청 결과를 프로세스 종료 상태로 변환한다.
///
/// 오류는 원인 체인과 함께 표준 오류로 출력해 Cargo 호출에서도 실패 맥락을 보존한다.
/// 도움말은 프로젝트 설정을 읽지 않고 성공 상태로 종료한다.
fn main() -> std::process::ExitCode {
    match cli::parse(std::env::args_os().skip(1)).and_then(|invocation| match invocation {
        cli::Invocation::Help => {
            println!(include_str!("help.txt"));
            Ok(())
        }
        cli::Invocation::Run(options) => document::run(options),
    }) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
