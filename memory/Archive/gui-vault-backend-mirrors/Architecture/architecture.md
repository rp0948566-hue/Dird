---
title: RetinaCare code architecture map
type: architecture
domain: engineering
status: VERIFIED
verified_on: 2026-10-28
source_refs: ["../../../../docs/architecture.md", "../../../../native/Cargo.toml"]
owner: Codex/Claude backend
tags: [architecture, source-map]
---
<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Architecture/architecture.md (sha256 1b3e4d0ef9e2); verify current state in [[active-phase]] -->
# Architecture (RetinaCare)

This is the code-level crate and protocol map. High-level dataflow:
[[Archive/gui-vault-backend-mirrors/Architecture/architecture-overview]]. Canonical docs:
[repository architecture](../../../../docs/architecture.md), [[Archive/gui-vault-backend-mirrors/Architecture/source-map]],
[[Archive/gui-vault-backend-mirrors/Handoffs/agent-ownership]].

```
retinacare-ui (eframe shell; frontend-owned)
      │ state/commands
      ▼
retinacare-app ─┬─► retinacare-persistence (SQLCipher, atomic .dird import)
  ModelService  ├─► retinacare-domain    (typed clinical entities)
                ├─► retinacare-reports   (report structures)
                └─► retinacare-ai-bridge (manifest → registry → runtime)
                         ├─► ONNX Runtime (CPU default; opt-in CUDA)
                         └─► WorkerClient (versioned JSONL, Python utility worker)
                                ▼
                 ai_engine/retinacare_ai/worker.py (persistent process)
                        │ health · device probe · image inspection ·
                        │ model discovery (lists .pt/.pth, never loads them)
                        ▼
                 models/ (Stage I ONNX/checkpoint present)
                 /tmp/retinacare-models/ (MA/HE Stage II; outside Git)
```

The Antigravity GUI is a second eframe application in its own Git repository;
it path-depends on Dird AI/app crates. Do not assume it is the same binary as
Dird native/crates/retinacare-ui. Native target avoids browser patient storage;
legacy web code still contains browser persistence.

Trust boundary: native commands expose typed operations, not raw SQL. Worker
validates request shape and local paths. Production inference requires
manifest-verified ONNX; Rust does not deserialize PyTorch weights.

IPC constants (`native/crates/retinacare-ai-bridge/src/lib.rs`):
`PROTOCOL_VERSION = 1`, request ≤ 4096 bytes, response ≤ 1 MiB, timeout kills
and reaps the child, `request_id` and protocol version must be echoed.

Paths: `retinacare-app::resources::ResourcePaths` resolves `python`,
`worker_source`, `models`, `data`; `RETINACARE_DEV_ROOT` / `RETINACARE_PYTHON`
override discovery in debug builds.
