"""npm ci로 받은 고정 버전 자산을 Rust 바이너리에 포함할 형태로 복사한다.

배포용 원본과 라이선스는 수정하지 않는다. 버전 변경 시 기능·라이선스를 검토하고
브라우저 테스트까지 실행한 뒤 생성 결과와 잠금 파일을 함께 커밋한다.
"""

from pathlib import Path
import hashlib
import shutil

ROOT = Path(__file__).resolve().parents[2]
MODULES = Path(__file__).resolve().parent / "node_modules"
OUTPUT = ROOT / "crates/cargo-textus/assets/render"
FILES = {
    "@mermaid-js/tiny/dist/mermaid.tiny.js": "mermaid.js",
    "@mermaid-js/tiny/LICENSE": "MERMAID-LICENSE",
    "katex/dist/katex.min.js": "katex.js",
    "katex/dist/contrib/auto-render.min.js": "auto-render.js",
    "katex/dist/katex.min.css": "katex.css",
    "katex/LICENSE": "KATEX-LICENSE",
}

OUTPUT.mkdir(parents=True, exist_ok=True)
for source, destination in FILES.items():
    shutil.copyfile(MODULES / source, OUTPUT / destination)
for font in sorted((MODULES / "katex/dist/fonts").iterdir()):
    destination = "fonts/" + font.name
    (OUTPUT / "fonts").mkdir(exist_ok=True)
    shutil.copyfile(font, OUTPUT / destination)
    FILES[str(font.relative_to(MODULES))] = destination
names = sorted(FILES.values())
(OUTPUT / "SHA256SUMS").write_text("".join(
    hashlib.sha256((OUTPUT / name).read_bytes()).hexdigest() + "  " + name + "\n"
    for name in names
))
(ROOT / "crates/cargo-textus/src/render_assets.rs").write_text(
    "//! 고정 버전 브라우저 자산을 바이너리에 포함해 설치 후 네트워크 없이 배치한다.\n"
    "//! scripts/render-assets/vendor.py에서 생성한다.\n\n"
    "/// 문서 루트 아래에 복사할 상대 경로와 원본 바이트. 폰트 상대 경로도 유지한다.\n"
    "pub const FILES: &[(&str, &[u8])] = &[\n" + "".join(
        f'    ("{name}", include_bytes!("../assets/render/{name}")),\n'
        for name in names
    ) + "];\n"
)
