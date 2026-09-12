#!/usr/bin/env bash
# Keep credentials out of command tracing, arguments, and credential files.
set +x
set -euo pipefail

usage() {
    cat <<'USAGE'
Usage: bash scripts/publish-crates.sh [--dry-run | --publish] [--package NAME]

Default: --dry-run --package all (no upload and no token required).
NAME: all, textus-core, textus, or cargo-textus.
--publish requires CARGO_REGISTRY_TOKEN in the environment.
Requires Cargo >= 1.90. Run fmt, clippy, and tests first; the Forgejo workflow
does this automatically. Cargo verifies the packaged sources in both modes.
USAGE
}

mode=--dry-run
package=all
mode_set=false
package_set=false
while (($#)); do
    case "$1" in
        --dry-run|--publish)
            if [[ "$mode_set" == true ]]; then
                echo 'Specify only one of --dry-run or --publish.' >&2
                exit 2
            fi
            mode=$1
            mode_set=true
            shift
            ;;
        --package)
            if [[ "$package_set" == true || $# -lt 2 ]]; then
                echo '--package requires exactly one package selection.' >&2
                exit 2
            fi
            package=$2
            package_set=true
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            echo 'Unknown argument; see --help.' >&2
            exit 2
            ;;
    esac
done

case "$package" in
    all) packages=(textus-core textus cargo-textus) ;;
    textus-core|textus|cargo-textus) packages=("$package") ;;
    *)
        echo 'Package must be all, textus-core, textus, or cargo-textus.' >&2
        exit 2
        ;;
esac

if [[ "$mode" == --publish && -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
    echo 'CARGO_REGISTRY_TOKEN is required for publication; configure it in Forgejo Actions Secrets.' >&2
    exit 2
fi

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
cd -- "$script_dir/.."

# The macro's language selector must not affect release builds.
unset TEXTUS_LANG

cargo_version=$(cargo --version)
if [[ ! "$cargo_version" =~ ^cargo\ ([0-9]+)\.([0-9]+)\. ]]; then
    echo 'Could not determine the Cargo version.' >&2
    exit 2
fi
major=${BASH_REMATCH[1]}
minor=${BASH_REMATCH[2]}
if ((major < 1 || (major == 1 && minor < 90))); then
    echo 'Cargo 1.90 or newer is required for multi-package publishing and dry-runs.' >&2
    exit 2
fi

args=(publish --registry crates-io --locked)
if [[ "$mode" == --dry-run ]]; then
    args+=(--dry-run)
fi
for name in "${packages[@]}"; do
    args+=(--package "$name")
done

# Cargo handles dependency order, archive verification, and index propagation.
# Explicit package selection excludes examples and future unrelated members.
exec cargo "${args[@]}"
