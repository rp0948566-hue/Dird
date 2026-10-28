# M5A — Stage I → II → III (fold 0) cascade: contract, routing, tests, measurements (2026-10-28)

**Status: COMPLETE for the M5A scope** (one real validated end-to-end technical trace plus the deterministic tests). Results are a **SINGLE-FOLD TECHNICAL TEST**, `research_only`, `validation_status = technical_only`. Not a five-fold ensemble, not CUDA, not licensed-image acceptance, not a diagnosis. Phase 3 stays PARTIAL.

Audit and design: [m5a-source-audit-and-plan.md](m5a-source-audit-and-plan.md).

## API (backend only; `retinacare-app`)
- `ModelService::analyze(AnalysisRequest, CascadeModelPaths) -> Result<(), AnalysisSubmitError>` (`QueueFull`, `Stopped`, `DuplicateRequest`), `cancel_analysis(request_id) -> bool`, `try_next_analysis() -> Result<AnalysisEvent, TryRecvError>`. Existing `ModelEvent` and `ModelSubmitError` are unchanged (the GUI matches on them).
- `AnalysisRequest { request_id, examination_id, patient_id: Option, eye: Option<EyeLaterality>, image_path, policy: TechnicalFold0 }`.
- Events: `Started`, `StageStarted`, `StageCompleted(summary)`, `StageSkipped(reason)`, exactly one `Finished(AnalysisReport)`.
- `AnalysisReport`: state (`Graded`, `Ungradable`, `NegativeRoute`, `Cancelled`, `Failed`), `cascade_grade` (`Grade0FromStage1Gate`, `Severity(1..=4)`, `Ungradable`), source SHA-256 and frame size, Stage I output, the five typed masks, fold-0 `Stage3GradeResult`, typed failure (stage + kind + fixed message), `cancelled_before`, `evidence_is_diagnostic_only`, per-stage wall time, total time, peak RSS, and the `research_only` / `origin` / `validation_status` labels.
- Runner seam: `CascadeRunner` trait (`screening`, `segment`, `grade`); `ModelRuntimeRunner` loads each model on the CPU provider, runs it and unloads it (a model already loaded by the caller is reused and left loaded). Panics inside a stage are caught and reported as a typed failure.

## Routing (pinned upstream `cascade.py`)
| Situation | Behaviour |
|---|---|
| Stage I score < 0.2391716 | `NegativeRoute`; five segmentation stages and grading emit `StageSkipped`; cascade grade 0 comes from the gate; no Stage III result |
| Stage I score ≥ threshold | five masks in order MA, HE, EX, CWS, VESSEL, then fold 0 |
| Stage I error, non-finite or missing output | `Failed`; never grade 0 (upstream has no "uncertain" branch) |
| P(gradable) < 0.10 at Stage III | `Ungradable`; no severity |
| Stage II or III failure, missing model | `Failed`; earlier outputs kept as diagnostic evidence only; no grade; no zero-filled mask |
| Cancel | checked before the request starts and before each stage; a running ONNX call is not interrupted |
| Mask from another image, examination, eye or frame size | `Failed` (`IdentityMismatch`) before grading |

## Verification
| Item | Evidence |
|---|---|
| T01-T05, T07, T09, T14, T25 | deterministic state-machine tests with a stand-in runner (pass) |
| T06, T17-T19 | `ModelService` tests: two requests run sequentially with no mixed results; queue holds 2, 4th submission `QueueFull`; duplicate id rejected; id reusable after finishing (pass) |
| T13, T20 | real runner with missing manifests → typed `ModelUnavailable`, worker keeps serving the next request (pass) |
| T15, T16 | cancel while queued never reaches a model; cancel at a stage boundary stops the next stage (pass); in-model interruption is not possible and is reported as such |
| T08, T10 | assembler and grading unit tests in `retinacare-ai-bridge` (missing/duplicate/non-binary/size mismatch masks; NaN logits) |
| T11, T12, corrupt SHA | covered by existing bridge preprocessing/manifest tests; not re-exercised at cascade level |
| T21 | heavy MA, HE, EX, CWS, VESSEL ai-bridge tests and the six app-service heavy tests incl. the five-mask run (all pass after the shared CLAHE change) |
| T22, T23 | heavy fold-0 PyTorch/ONNX/Rust test (logits within 5.7e-6) and eight-channel tensor parity (max abs 0) pass |
| T24 | real cascade on the demo image: Stage I score 0.9983 (continue), five masks equal the upstream reference masks, fold 0 grade 3 (technical, single fold), terminal `Graded` |
| T26 | report records provider `cpu`; CUDA not exercised |
| T27 | one measured run, not three: see below |
| T29 | no GUI file touched; the GUI repo has its own uncommitted Antigravity changes |
| T28 | not run (GUI idle recovery is Antigravity-owned) |
| T30 | fmt, strict Clippy, release build, `git diff --check` pass; `cargo test --workspace` 90 passed, 0 failed, 17 ignored |

## Measurements (single full run, CPU, this host, demo image 640×640)
Wall time per stage includes manifest verification (SHA-256 of ~0.8 GB), ONNX session creation, inference and unload: Stage I 43.5 s (inference 6.9 s), MA 33.6 s, HE 29.2 s, EX 38.9 s, CWS 39.2 s, VESSEL 19.8 s, fold 0 27.9 s (inference 1.5 s). Total 232 s. Peak RSS 2.2 GB (`VmHWM`). Model loading, not inference, dominates; caching verified models is a possible later optimisation and was not done. No SLA is claimed. GPU memory was not measured.

## Not done / limits
- Fold 0 only. Folds 1-4 need separate approval; no `FullEnsemble` label exists.
- Demo image provenance is undocumented: technical only; licensed-image acceptance BLOCKED. CUDA and Windows unverified.
- Cascade-level corrupt-image/oversized-image cases rely on the existing bridge tests.
- The image file is re-read by each stage (seven reads); every stage must agree on the first stage's SHA-256 and frame size.
- No Dice/IoU/QWK; no clinical claim.
