"""Persistent newline-delimited JSON protocol worker; stdout is protocol-only."""

from __future__ import annotations

import json
import os
from pathlib import Path
import sys
from typing import Any

from .devices import inspect_device
from .preprocessing import ImageInputError, inspect_image

PROTOCOL_VERSION = 1
MAX_REQUEST_BYTES = 1024 * 1024
PACKAGE_ROOT = Path(__file__).resolve().parents[3]
MODEL_DIR = Path(os.environ.get("RETINACARE_MODEL_DIR", PACKAGE_ROOT / "models"))


def discover_models() -> list[dict[str, Any]]:
    candidates: list[dict[str, Any]] = []
    if MODEL_DIR.is_dir():
        for path in sorted(MODEL_DIR.rglob("*")):
            if path.is_file() and path.suffix.lower() in {".pt", ".pth"}:
                candidates.append({
                    "name": path.name,
                    "path": str(path.resolve()),
                    "loadable": False,
                    "status": "adapter_not_implemented",
                })
    return candidates


def handle(request: dict[str, Any]) -> dict[str, Any]:
    request_id = request.get("request_id")
    command = request.get("command")
    base = {
        "protocol_version": PROTOCOL_VERSION,
        "request_id": request_id,
        "command": command,
    }
    if request.get("protocol_version") != PROTOCOL_VERSION:
        return {**base, "status": "error", "error": {"code": "protocol_version_unsupported"}}
    if not isinstance(request_id, str) or not request_id or not isinstance(command, str):
        return {**base, "status": "error", "error": {"code": "invalid_request"}}
    payload = request.get("payload", {})
    if not isinstance(payload, dict):
        return {**base, "status": "error", "error": {"code": "invalid_payload"}}

    if command == "hello":
        return {**base, "status": "ok", "data": {"worker": "retinacare-ai", "ready": True}}
    if command == "health":
        device, elapsed_ms = inspect_device()
        models = discover_models()
        return {
            **base,
            "status": "ok",
            "data": {
                "worker": "retinacare-ai",
                "python_version": sys.version.split()[0],
                "device": device,
                "models": models,
                "model_state": "available_but_no_adapter" if models else "no_trained_weights",
            },
            "timings_ms": {"device_probe": elapsed_ms},
        }
    if command == "list_models":
        models = discover_models()
        return {**base, "status": "ok", "data": {"models": models, "model_state": "available_but_no_adapter" if models else "no_trained_weights"}}
    if command == "analyze_image":
        image_path = payload.get("image_path")
        if not isinstance(image_path, str) or not image_path:
            return {**base, "status": "error", "error": {"code": "image_path_required"}}
        try:
            image, preprocess_ms = inspect_image(image_path)
        except (OSError, ImageInputError) as exc:
            return {**base, "status": "error", "error": {"code": str(exc) if isinstance(exc, ImageInputError) else "image_file_unavailable"}}
        models = discover_models()
        code = "inference_adapter_unavailable" if models else "no_trained_weights"
        return {
            **base,
            "status": "error",
            "error": {"code": code, "message": "No clinical result was produced."},
            "data": {"image": image, "findings": [], "artifacts": []},
            "timings_ms": {"image_validation": preprocess_ms},
        }
    if command == "shutdown":
        return {**base, "status": "ok", "data": {"stopping": True}}
    return {**base, "status": "error", "error": {"code": "unknown_command"}}


def run() -> int:
    while True:
        line = sys.stdin.buffer.readline(MAX_REQUEST_BYTES + 1)
        if not line:
            break
        if len(line) > MAX_REQUEST_BYTES:
            while line and not line.endswith(b"\n"):
                line = sys.stdin.buffer.readline(MAX_REQUEST_BYTES + 1)
            response = {
                "protocol_version": PROTOCOL_VERSION,
                "request_id": None,
                "status": "error",
                "error": {"code": "request_too_large"},
            }
        else:
            try:
                request = json.loads(line)
                if not isinstance(request, dict):
                    raise ValueError("request_must_be_object")
                response = handle(request)
            except (json.JSONDecodeError, ValueError):
                response = {
                    "protocol_version": PROTOCOL_VERSION,
                    "request_id": None,
                    "status": "error",
                    "error": {"code": "invalid_json"},
                }
        sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
        sys.stdout.flush()
        if response.get("command") == "shutdown" and response.get("status") == "ok":
            return 0
    return 0


if __name__ == "__main__":
    raise SystemExit(run())
