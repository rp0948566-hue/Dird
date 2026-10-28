# Documentation map

- `architecture.md`: native crate boundaries and IPC design; read for code structure changes.
- `migration-audit.md`: verified legacy inventory and risks; read before migration work.
- `migration-progress.md`: current phase status and exact blockers; read at session start.
- `code-change-report.md`: file-by-file reasons for current implementation changes; read for change review.
- `code-quality-audit.md`: Phase 1.5 native/Python findings, fixes, and remaining risks.
- `exe-verification.md`: verified build evidence and Ubuntu/Windows execution status.
- `model-integration.md`: checkpoint contract and missing-model behavior; read for model work.
- `datasets.md`: dataset licensing and acquisition; read before accessing data.
- `windows-build.md`: native Windows build instructions; read for packaging work.
- `security.md`: patient-data and worker security controls; read for persistence/IPC changes.
- `browser-unlock-fix-plan.md`: legacy localhost unlock diagnosis and change; read only when editing that flow.
- `features/P0-01-hierarchiretina-model-loading/`: P0.1 model-loading audit, research, contracts, implementation, test evidence and handoff; read before model work.
- `features/P0-02-stage2-segmentation/`: P0.2 five-model Stage II contracts, upstream preprocessing, checkpoint inventory and implementation status.
- `features/P0-01-hierarchiretina-model-loading/stage1-rust-preprocessing-and-inference.md`: exact Stage I preprocessing contract, parity evidence and open acceptance gates; read before changing Stage I tensors or outputs.
