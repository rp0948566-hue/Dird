"""Local image validation and preprocessing metadata; no clinical inference."""

from __future__ import annotations

from pathlib import Path
import time
from typing import Any

MAX_IMAGE_BYTES = 100 * 1024 * 1024
MAX_IMAGE_PIXELS = 40_000_000
IMAGE_SUFFIXES = {".jpg", ".jpeg", ".png", ".tif", ".tiff", ".bmp"}


class ImageInputError(ValueError):
    pass


def inspect_image(image_path: str) -> tuple[dict[str, Any], float]:
    started = time.perf_counter()
    path = Path(image_path).expanduser().resolve(strict=True)
    if not path.is_file() or path.suffix.lower() not in IMAGE_SUFFIXES:
        raise ImageInputError("unsupported_image_path_or_format")
    size = path.stat().st_size
    if size <= 0 or size > MAX_IMAGE_BYTES:
        raise ImageInputError("image_size_out_of_range")
    try:
        from PIL import Image  # type: ignore[import-not-found]
    except ImportError as exc:
        raise ImageInputError("pillow_unavailable") from exc
    Image.MAX_IMAGE_PIXELS = MAX_IMAGE_PIXELS
    try:
        with Image.open(path) as image:
            image.verify()
        with Image.open(path) as image:
            width, height = image.size
            if width <= 0 or height <= 0:
                raise ImageInputError("invalid_image_dimensions")
            if width * height > MAX_IMAGE_PIXELS:
                raise ImageInputError("image_pixel_limit_exceeded")
            image.convert("RGB")
    except ImageInputError:
        raise
    except Exception as exc:
        raise ImageInputError("image_decode_failed") from exc
    return {
        "format": path.suffix.lower().lstrip("."),
        "width": width,
        "height": height,
        "bytes": size,
        "preprocessed": False,
    }, round((time.perf_counter() - started) * 1000, 3)
