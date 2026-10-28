# Local AI worker

Run from the repository root with `PYTHONPATH=ai_engine/src python -m retinacare_ai.worker` (Windows PowerShell: `$env:PYTHONPATH='ai_engine/src'; python -m retinacare_ai.worker`). Send UTF-8 JSON Lines to stdin and read one response line per request from stdout.

Protocol version 1 supports `hello`, `health`, `list_models`, `analyze_image`, and `shutdown`. Stderr is for diagnostics. No network listener, automatic downloads, or bundled checkpoints. Install optional extras only in a project-controlled environment: `pip install -e 'ai_engine[images,gpu]'`.

The current phase provides device discovery, image validation, and explicit missing-checkpoint/adapter errors. It does not produce clinical inference.
