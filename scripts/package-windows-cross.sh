#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
stage="$root/dist/RetinaCare"
cache="${RETINACARE_CACHE:-/tmp/retinacare-runtimes}"
runtime="$cache/python-windows.tar.gz"
wheel="$cache/wheels-windows/pillow-13.0.0-cp313-cp313-win_amd64.whl"
runtime_sha='56cde3b1a1dcc36576753cb408ee0c3568d211baafa0eee40423adc147c480f2'
wheel_sha='9dda0788a9932776a75d8bcc020076edc308ce2508b51f6739a45e3158b3fc42'

: "${RETINACARE_MINGW_BIN:?Set RETINACARE_MINGW_BIN to the MinGW bin directory}"
export PATH="$RETINACARE_MINGW_BIN:$PATH"
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar
export RANLIB_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ranlib
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
export RETINACARE_RC=x86_64-w64-mingw32-windres
printf '%s  %s\n' "$runtime_sha" "$runtime" | sha256sum --check --status
printf '%s  %s\n' "$wheel_sha" "$wheel" | sha256sum --check --status

cargo build --release --manifest-path "$root/native/Cargo.toml" -p retinacare-ui --target x86_64-pc-windows-gnu
rm -rf "$stage"
mkdir -p "$stage/runtime" "$stage/resources/ai_engine" "$stage/resources/models"
cp "$root/native/target/x86_64-pc-windows-gnu/release/RetinaCare.exe" "$stage/RetinaCare.exe"
tar -xzf "$runtime" -C "$stage/runtime" --strip-components=1
mkdir -p "$stage/runtime/Lib/site-packages"
python3 -m zipfile -e "$wheel" "$stage/runtime/Lib/site-packages"
cp -R "$root/ai_engine/src" "$stage/resources/ai_engine/src"
cp "$root/models/README.md" "$root/models/manifest.example.json" "$stage/resources/models/"
cp "$root/LICENSE" "$root/README.md" "$stage/"
cp "$root/scripts/test-windows.ps1" "$stage/"
(cd "$root/dist" && sha256sum RetinaCare/RetinaCare.exe > RetinaCare/SHA256SUMS && zip -qr RetinaCare-Windows-x86_64.zip RetinaCare)
sha256sum "$root/dist/RetinaCare-Windows-x86_64.zip" > "$root/dist/RetinaCare-Windows-x86_64.zip.sha256"
printf 'Cross-built %s; execution requires Windows.\n' "$stage"
