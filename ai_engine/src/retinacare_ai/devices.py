"""Optional PyTorch/CUDA discovery. Missing packages are normal CPU-only state."""

from __future__ import annotations

import importlib.metadata
import time
from typing import Any


def inspect_device() -> tuple[dict[str, Any], float]:
    started = time.perf_counter()
    device: dict[str, Any] = {
        "torch_available": False,
        "torch_version": None,
        "cuda_available": False,
        "device_name": None,
        "cuda_version": None,
        "memory_bytes": None,
        "error": None,
    }
    try:
        import torch  # type: ignore[import-not-found]

        device["torch_available"] = True
        device["torch_version"] = torch.__version__
        device["cuda_version"] = torch.version.cuda
        device["cuda_available"] = bool(torch.cuda.is_available())
        if device["cuda_available"]:
            index = torch.cuda.current_device()
            device["device_name"] = torch.cuda.get_device_name(index)
            props = torch.cuda.get_device_properties(index)
            device["memory_bytes"] = {
                "total": int(props.total_memory),
                "allocated": int(torch.cuda.memory_allocated(index)),
                "reserved": int(torch.cuda.memory_reserved(index)),
            }
    except ImportError:
        device["error"] = "pytorch_unavailable"
        try:
            device["torch_version"] = importlib.metadata.version("torch")
        except importlib.metadata.PackageNotFoundError:
            pass
    except Exception as exc:  # Device discovery must not kill the JSONL worker.
        device["error"] = f"device_probe_failed: {type(exc).__name__}"
    return device, round((time.perf_counter() - started) * 1000, 3)
