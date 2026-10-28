#!/usr/bin/env bash
# Real Stage I CUDA proof (P1-M3). Needs: NVIDIA GPU visible to `nvidia-smi`, and the project-local CUDA 13 libs
# listed in $CUDA_LIB_PATH_FILE (default below). The session is built with CPU fallback disabled, so a pass means
# the CUDA execution provider actually ran. Prints a CUDA-PROFILE line with timings, GPU memory and logits.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
nvidia-smi -L >/dev/null || { echo "nvidia-smi sees no GPU; CUDA cannot run (reboot/reload the driver)"; exit 2; }
libs="$(cat "${CUDA_LIB_PATH_FILE:-$HOME/Desktop/retinacare-cuda/libpath.txt}")"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/Desktop/retinacare-cuda/target}"
LD_LIBRARY_PATH="$libs" \
RETINACARE_STAGE1_MANIFEST="$root/models/stage1_convnextv2l_768/manifest.json" \
RETINACARE_STAGE1_IMAGE="${RETINACARE_STAGE1_IMAGE:-$root/dist/demo-images/session-1-final/OD-1.png}" \
cargo test --manifest-path "$root/native/Cargo.toml" -p retinacare-ai-bridge --features cuda --release \
  local_stage1_cuda -- --ignored --nocapture
