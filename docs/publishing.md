# crates.io 수동 배포

`.forgejo/workflows/publish.yml`은 `workflow_dispatch`로만 실행합니다.
push, 태그 생성, 릴리즈 생성으로 자동 배포하지 않습니다.

## Forgejo 설정

1. 저장소의 Actions를 활성화하고 `intensive-distro` 라벨의 러너를 연결합니다.
2. 러너에 Bash, Git, Python 3(배포 스크립트 테스트용), Rust/Cargo **1.90 이상**, rustfmt, Clippy, 링커 등 Rust 빌드
   도구를 준비합니다. 현재 프로젝트는 Rust 1.95.0으로 검증했습니다.
   checkout 액션은 Node.js 20 런타임을 사용하므로 러너 실행 환경에서 지원해야 합니다.
3. crates.io에서 배포 권한이 있는 API 토큰을 발급합니다. 필요한 크레이트는
   `textus-core`, `textus`, `cargo-textus`입니다. 최초 배포 시에는 새 크레이트를
   생성할 수 있는 권한이 필요하고, 이미 존재한다면 해당 크레이트 소유자의 권한이 필요합니다.
4. Forgejo 저장소 **Settings → Actions → Secrets**에
   **`CARGO_REGISTRY_TOKEN`**이라는 이름으로 토큰을 저장합니다.
   위치는 `/{owner}/{repo}/settings/actions/secrets`입니다.
5. 워크플로 파일이 Forgejo 저장소에 반영되면 Actions에서 **Publish to crates.io**를
   선택하고 수동 실행합니다. 실제 Forgejo 인스턴스에서의 등록·실행은 별도 단계입니다.

토큰은 업로드 step의 환경변수로만 전달합니다. 테스트·dry-run step에는 등록한
Secret을 주입하지 않습니다. `cargo login`이나 `--token`을 사용하지 않으며,
토큰을 로그나 credentials 파일에 기록하지 않습니다. checkout 인증도
`persist-credentials: false`로 작업 이후에 남기지 않습니다.

러너는 checkout 액션의 `github.com`과 Cargo 의존성 및 crates.io 업로드·인덱스에
접근할 수 있어야 합니다. 도구 체인을 자동 설치하거나 업데이트하지 않습니다.

## 실행 옵션

| 입력 | 기본값 | 의미 |
| --- | --- | --- |
| `mode` | `dry-run` | 패키징과 빌드 검증만 수행. `publish`를 선택하면 검증 후 업로드 |
| `package` | `all` | 세 크레이트 전체 선택. 개별 이름을 선택하면 해당 크레이트만 처리 |

배포할 커밋이 있는 브랜치를 선택하고 `mode=publish`, `package=all`로 실행합니다.
선택한 실행의 SHA를 checkout하며 버전은 해당 커밋의 `Cargo.toml`을 사용합니다.
버전을 자동 변경하거나 Git 태그·릴리즈를 생성하지 않습니다.

워크플로는 다음 순서로 동작합니다.

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`
3. `cargo test --workspace --locked`
4. 선택한 크레이트들을 `cargo publish --dry-run --registry crates-io --locked`로 검증
5. `mode=publish`인 경우 같은 선택으로 `cargo publish` 수행

Cargo 1.90 이상의 다중 패키지 배포 기능이 내부 의존성을 고려합니다.
먼저 `textus-core`가 배포되어야 `textus`와 `cargo-textus`를 배포할 수 있습니다.
최초 배포의 dry-run도 아직 등록하지 않은 워크스페이스 의존성을 함께 검증합니다.
`textus-demo`는 `publish = false`이며 스크립트의 배포 목록에도 포함하지 않습니다.

## 버전 갱신 및 재실행

2026-09-12의 실제 dry-run에서 **`textus 0.1.0`이 이미 crates.io 인덱스에 존재함**을
확인했습니다. dry-run은 이를 경고로 표시하고 패키징을 검증하지만, 실제 배포는
거부합니다. 본인 소유 크레이트라면 새 버전으로, 본인 소유가 아니라면 사용 가능한
패키지 이름으로 조정해야 합니다. 소유권은 이번 작업에서 확인하지 않았습니다.
이름을 변경하면 Cargo 의존성과 스크립트·워크플로의 배포 목록도 함께 갱신해야 합니다.

- 다음 배포에서는 루트 `Cargo.toml`의 `[workspace.package].version`과
  `[workspace.dependencies].textus-core.version`을 함께 검토·갱신합니다.
  `Cargo.lock`도 갱신하고 검증한 변경을 커밋한 뒤 실행합니다.
- crates.io의 동일 이름·버전은 다시 업로드할 수 없습니다. 크레이트 이름이 이미
  다른 소유자에게 등록되어 있으면 이름을 변경하거나 배포 권한을 받아야 합니다.
- 여러 크레이트의 배포는 원자적이지 않습니다. 실패 시 로그와 crates.io에서 실제
  등록된 이름·버전을 먼저 확인하고, 아직 배포하지 않은 크레이트를 `package`로
  선택하여 수동 실행합니다. 개별 dry-run에는 선행 의존성이 crates.io에 있어야 합니다.
- 인덱스 반영 대기 중 실패한 경우에도 업로드 자체는 완료되었을 수 있습니다.
  무조건 자동 재시도하거나 기존 버전을 자동 건너뛰지 않습니다.
- 같은 저장소의 배포 실행은 `crates-io-publish` concurrency 그룹으로 대기시키며,
  진행 중인 배포를 새 실행이 취소하지 않도록 설정했습니다.

## 로컬 검증

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s scripts/tests
bash scripts/publish-crates.sh --dry-run
```

dry-run에는 토큰이 필요 없지만 crates.io 네트워크 접근은 필요합니다.
스크립트 기본 동작도 dry-run입니다. `--allow-dirty`, `--no-verify` 등 검증을
생략하는 옵션을 전달하지 않으므로 배포 대상의 변경은 먼저 커밋해야 합니다.
로컬에서 실제 업로드하려면 토큰을 환경에 제공한 뒤 `--publish`를 명시합니다.
스크립트 자체는 Cargo 패키징·검증·배포를 담당하고, fmt·clippy·test는 위 명령이나
Forgejo 워크플로에서 별도로 실행합니다.

```bash
bash scripts/publish-crates.sh --dry-run --package textus-core
# CARGO_REGISTRY_TOKEN을 안전하게 환경에 제공한 경우에만:
bash scripts/publish-crates.sh --publish --package textus-core
```

참고: [Forgejo 수동 실행·Secrets·concurrency](https://forgejo.org/docs/latest/user/actions/reference/),
[Cargo 다중 패키지 배포](https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/),
[Cargo publish](https://doc.rust-lang.org/cargo/commands/cargo-publish.html).
