#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stage="$root/dist/RetinaCare-Ubuntu"
cache="${RETINACARE_CACHE:-/tmp/retinacare-runtimes}"
runtime="$cache/python-linux.tar.gz"
wheel="$cache/wheels-linux/pillow-13.0.0-cp313-cp313-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl"
runtime_url='https://github.com/astral-sh/python-build-standalone/releases/download/20261021/cpython-3.13.17%2B20261021-x86_64-unknown-linux-gnu-install_only_stripped.tar.gz'
runtime_sha='f829adc9d888e76c8801b4799ed444188d35239fcae3a9808a99ec5cb0d8d92c'
wheel_sha='5f56499c1a74d2b862beb0c33b8016f99d095858455df9d21583bbbc1c15b582'

mkdir -p "$cache" "$(dirname "$wheel")"
test -f "$runtime" || curl -fL "$runtime_url" -o "$runtime"
test -f "$wheel" || python3 -m pip download --dest "$(dirname "$wheel")" --only-binary=:all: --platform manylinux_2_27_x86_64 --implementation cp --python-version 3.13 --abi cp313 Pillow==13.0.0
printf '%s  %s\n' "$runtime_sha" "$runtime" | sha256sum --check --status
printf '%s  %s\n' "$wheel_sha" "$wheel" | sha256sum --check --status

cargo build --release --manifest-path "$root/native/Cargo.toml" -p retinacare-ui
rm -rf "$stage"
mkdir -p "$stage/runtime" "$stage/resources/ai_engine" "$stage/resources/models"
cp "$root/native/target/release/RetinaCare" "$stage/RetinaCare"
tar -xzf "$runtime" -C "$stage/runtime" --strip-components=1
"$stage/runtime/bin/python3" -m pip install --no-index --no-deps --target "$stage/runtime/lib/python3.13/site-packages" "$wheel"
cp -R "$root/ai_engine/src" "$stage/resources/ai_engine/src"
cp "$root/models/README.md" "$root/models/manifest.example.json" "$stage/resources/models/"
cp "$root/LICENSE" "$root/README.md" "$stage/"

XDG_DATA_HOME="${XDG_DATA_HOME:-/tmp/retinacare-smoke-data}" "$stage/RetinaCare" --smoke-test "$stage/smoke-test.json"
(cd "$root/dist" && sha256sum RetinaCare-Ubuntu/RetinaCare > RetinaCare-Ubuntu/SHA256SUMS && tar -czf RetinaCare-Ubuntu-x86_64.tar.gz RetinaCare-Ubuntu)
sha256sum "$root/dist/RetinaCare-Ubuntu-x86_64.tar.gz" > "$root/dist/RetinaCare-Ubuntu-x86_64.tar.gz.sha256"
printf 'Built %s\n' "$stage"
