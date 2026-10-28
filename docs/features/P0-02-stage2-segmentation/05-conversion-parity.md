# Conversion and parity gates

MA and HE conversion and CPU numerical parity completed on 2026-10-28. Evidence
and measured errors are recorded in [07 test results](07-test-results.md).
EX, CWS and Vessel remain unconverted pending separate checkpoint approvals.

MA/HE flow completed:

1. Verified exact expected byte size and SHA-256 from upstream manifest before
   deserialization. Save only into ignored `models/` storage. Download one
   checkpoint at a time; verify a temporary `.part` file before atomic rename.
2. Loaded only the checksum-verified upstream checkpoint with Python's safe
   `torch.load(..., weights_only=True, map_location="cpu")`. Reject unexpected
   archive/state structure. Instantiate exact v1.0 variant with
   `pretrained=False`, strict-load the EMA/model state specified by the
   official v1.0 inference notebook, and run in eval mode.
3. Exported a wrapper whose sole output is main full-resolution logits. Used
   fixed input `[1,3,1024,1024]`, opset chosen after graph/operator validation,
   constant folding, and record source commit, exporter versions, input/output
   names, shapes, checkpoint hash and ONNX hash in manifest.
4. Ran the same real-image preprocessed tensor through PyTorch and CPU ONNX Runtime.
   Compare raw logit map and sigmoid probability map elementwise; record max
   absolute error, relative error, RMSE, finite/range checks, and mask
   agreement at the upstream threshold. Tolerances must be justified from
   repeat runs/provider and dtype; choose no thresholds before observing the
   export's numerical behavior.
5. Loaded the manifest through Rust `VerifiedSession`; ran the exact same input
   on CPU; compare complete logits/probabilities and restored mask pixels to
   Python. Verify deterministic repeated output, shape, finite values,
   threshold boundary semantics and inverse crop transform.

Use `weights_only=True`; upstream's helper currently uses
`weights_only=False`, which is inappropriate for an untrusted pickle. Never
load before integrity verification. Do not publish the local demo image or use
it as license/clinical acceptance.
