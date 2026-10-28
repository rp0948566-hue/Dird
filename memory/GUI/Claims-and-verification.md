<!-- migrated 2026-10-28 from /home/claude/Desktop/Dird/GUI/Hackthon dezine/memory/Claims-and-verification.md (sha256 c32d6b72f502); verify current state in [[active-phase]] -->
# Claims and verification

## Scope

This is a reconciliation note for GUI and backend memory, not a fresh code audit or test run. See [[GUI/gui-00-index]] and [[GUI/gui-01-claude-start-here]].

## Status corrections

- Backend notes report Stage I CPU pipeline work and Stage II MA/HE work. Treat results as recorded evidence; this memory consolidation did not rerun those checks. Licensed-image acceptance and RTX 5070/CUDA evidence remain pending.
- Stage III grading remains pending. GUI Stage IV clinical grading enum is inconsistent with the three HierarchiRetina AI stages and should not be presented as an implemented AI stage.
- MA and HE are the only authorized Stage II checkpoints in the prior session. Do not infer EX, CWS, or vessel model availability.
- GUI test counts are historical reports from GUI task notes, not independently rerun here. Task05 includes mock services and does not demonstrate production PDF/audit integration.
- A GUI-recorded Stage I checkpoint hash conflicts with backend model provenance; recheck against the actual model manifest before use.
- GUI references ONNX Runtime rc.9 while backend records rc.15; resolve versions against lockfiles/build evidence.
- FIPS hash algorithm use is not FIPS certification. De-identification features do not establish HIPAA compliance.
- Demo/mock clinical-looking content is not a real result and must not be used as clinical evidence.

## Source of truth

- GUI feature evidence: `../../Hackthon dezine/docs/features/`.
- Backend feature and model evidence: `../../Dird/docs/features/`.
- Current phase notes: [[active-phase]], [[GUI/gui-active-phase]].
