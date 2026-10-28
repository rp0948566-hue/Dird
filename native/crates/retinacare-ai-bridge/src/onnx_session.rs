//! P0.1 session path: verified ONNX artifacts only; PyTorch checkpoints are
//! never deserialized by the production Rust runtime.
use crate::{
    device::{ActualProvider, ProviderRequest},
    model_manifest::{ArtifactFormat, ManifestError, VerifiedModel, verify_manifest},
};
use ort::{
    environment::Environment,
    session::Session,
    value::{Tensor, TensorElementType, ValueType},
};
use std::{path::Path, sync::OnceLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    /// Invalid manifest, missing artifact or checksum mismatch: ONNX Runtime
    /// is never reached.
    #[error("manifest gate rejected the model: {0}")]
    Manifest(#[from] ManifestError),
    #[error("onnx runtime environment unavailable: {0}")]
    Runtime(String),
    /// The file verified as bytes but is not a loadable ONNX graph.
    #[error("onnx session could not be created: {0}")]
    Build(String),
    #[error("PyTorch checkpoint requires ONNX conversion before production inference")]
    ConversionRequired,
    #[error("requested CUDA provider is not compiled into this build")]
    CudaNotCompiled,
    #[error("CUDA device id {0} is outside the supported range")]
    InvalidCudaDevice(u32),
    #[error("input has {actual} values; model requires {expected}")]
    InputSize { expected: usize, actual: usize },
    #[error("input contains a non-finite value")]
    NonFiniteInput,
    /// The graph's declared tensors disagree with the manifest.
    #[error("model metadata disagrees with the manifest: {0}")]
    InputMismatch(String),
    #[error("inference failed: {0}")]
    Run(String),
}

/// ONNX Runtime allows a single environment per process, so it is created once
/// and any failure is cached rather than retried on every call.
static ENVIRONMENT: OnceLock<Result<Environment, String>> = OnceLock::new();

fn environment() -> Result<&'static Environment, SessionError> {
    match ENVIRONMENT.get_or_init(|| ort::init().build().map_err(|error| error.to_string())) {
        Ok(environment) => Ok(environment),
        Err(error) => Err(SessionError::Runtime(error.clone())),
    }
}

/// A manifest that passed verification, bound to a live ONNX session whose
/// graph metadata was checked against that manifest.
pub struct VerifiedSession {
    model: VerifiedModel,
    session: Session,
    input_name: String,
    output_name: String,
    output_shape: Vec<i64>,
    actual_provider: ActualProvider,
}

impl VerifiedSession {
    /// Load a verified ONNX model on CPU.
    pub fn open(manifest_path: &Path) -> Result<Self, SessionError> {
        Self::open_with_provider(manifest_path, ProviderRequest::Cpu)
    }

    /// Load a verified model using an explicit execution provider request.
    pub fn open_with_provider(
        manifest_path: &Path,
        provider: ProviderRequest,
    ) -> Result<Self, SessionError> {
        #[cfg(not(feature = "cuda"))]
        if matches!(provider, ProviderRequest::Cuda { .. }) {
            return Err(SessionError::CudaNotCompiled);
        }
        let model = verify_manifest(manifest_path)?;
        if model.format != ArtifactFormat::Onnx {
            return Err(SessionError::ConversionRequired);
        }
        let builder = Session::builder(environment()?)
            .map_err(|error| SessionError::Build(error.to_string()))?;
        let (mut builder, actual_provider) = match provider {
            ProviderRequest::Cpu => (
                builder
                    .with_execution_providers([ort::ep::CPU::default().build().error_on_failure()])
                    .map_err(|error| SessionError::Build(error.to_string()))?,
                ActualProvider::Cpu,
            ),
            ProviderRequest::Cuda { device_id } => {
                #[cfg(not(feature = "cuda"))]
                {
                    let _ = device_id;
                    return Err(SessionError::CudaNotCompiled);
                }
                #[cfg(feature = "cuda")]
                {
                    let device = i32::try_from(device_id)
                        .map_err(|_| SessionError::InvalidCudaDevice(device_id))?;
                    let builder = builder
                        .with_disable_cpu_fallback()
                        .map_err(|error| SessionError::Build(error.to_string()))?
                        .with_execution_providers([ort::ep::CUDA::default()
                            .with_device_id(device)
                            .build()
                            .error_on_failure()])
                        .map_err(|error| SessionError::Build(error.to_string()))?;
                    (builder, ActualProvider::Cuda { device_id })
                }
            }
        };
        let session = builder
            .commit_from_file(&model.checkpoint)
            .map_err(|error| SessionError::Build(error.to_string()))?;
        Self::inspect(session, model, actual_provider)
    }

    fn inspect(
        session: Session,
        model: VerifiedModel,
        actual_provider: ActualProvider,
    ) -> Result<Self, SessionError> {
        // ponytail: single input/output graph only; a multi-input stage would
        // need an explicit input-mapping field in the manifest schema first.
        if session.inputs().len() != 1 || session.outputs().len() != 1 {
            return Err(SessionError::InputMismatch(format!(
                "expected one input and one output, found {} and {}",
                session.inputs().len(),
                session.outputs().len()
            )));
        }
        let declared: Vec<i64> = model
            .input_shape
            .iter()
            .map(|&dimension| dimension as i64)
            .collect();
        let (input_name, input_type, input_shape) = match session.inputs()[0].dtype() {
            ValueType::Tensor { ty, shape, .. } => {
                (session.inputs()[0].name().to_string(), *ty, shape.to_vec())
            }
            _ => return Err(SessionError::InputMismatch("input is not a tensor".into())),
        };
        let (output_name, output_type, output_shape) = match session.outputs()[0].dtype() {
            ValueType::Tensor { ty, shape, .. } => {
                (session.outputs()[0].name().to_string(), *ty, shape.to_vec())
            }
            _ => return Err(SessionError::InputMismatch("output is not a tensor".into())),
        };
        if input_name != model.input_name {
            return Err(SessionError::InputMismatch(format!(
                "input name `{input_name}` does not match manifest `{}`",
                model.input_name
            )));
        }
        if input_type != TensorElementType::Float32 || model.input_data_type != "float32" {
            return Err(SessionError::InputMismatch(format!(
                "input `{input_name}` is {input_type:?}, manifest declares float32"
            )));
        }
        if input_shape != declared {
            return Err(SessionError::InputMismatch(format!(
                "input `{input_name}` is {input_shape:?}, manifest declares {declared:?}"
            )));
        }
        if output_name != model.output_name
            || output_type != TensorElementType::Float32
            || model.output_data_type != "float32"
            || output_shape != model.output_shape
        {
            return Err(SessionError::InputMismatch(format!(
                "output `{output_name}` metadata ({output_type:?}, {output_shape:?}) differs from manifest ({}, {}, {:?})",
                model.output_name, model.output_data_type, model.output_shape
            )));
        }
        Ok(Self {
            model,
            session,
            input_name,
            output_name,
            output_shape,
            actual_provider,
        })
    }

    /// Run the graph using the provider reported by `actual_provider`.
    pub fn run(&mut self, image: &[f32]) -> Result<Vec<f32>, SessionError> {
        let expected = self
            .model
            .input_shape
            .iter()
            .try_fold(1usize, |count, dimension| count.checked_mul(*dimension))
            .ok_or_else(|| SessionError::InputMismatch("input shape overflows usize".into()))?;
        if image.len() != expected {
            return Err(SessionError::InputSize {
                expected,
                actual: image.len(),
            });
        }
        if image.iter().any(|value| !value.is_finite()) {
            return Err(SessionError::NonFiniteInput);
        }
        let tensor = Tensor::from_array((self.model.input_shape.clone(), image.to_vec()))
            .map_err(|error| SessionError::Run(error.to_string()))?;
        let outputs = self
            .session
            .run(ort::inputs![tensor])
            .map_err(|error| SessionError::Run(error.to_string()))?;
        let (_, values) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|error| SessionError::Run(error.to_string()))?;
        Ok(values.to_vec())
    }

    pub fn model(&self) -> &VerifiedModel {
        &self.model
    }

    pub fn input_name(&self) -> &str {
        &self.input_name
    }

    pub fn output_name(&self) -> &str {
        &self.output_name
    }

    pub fn output_shape(&self) -> &[i64] {
        &self.output_shape
    }

    pub fn actual_provider(&self) -> ActualProvider {
        self.actual_provider
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use sha2::{Digest, Sha256};
    use std::{fs, path::PathBuf};

    const FIXTURE: &str = "fixtures/tiny_mul.onnx";
    /// 1 x 3 x 4 x 4 — the fixture's declared input element count.
    const ELEMENTS: usize = 48;

    fn scratch(name: &str) -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("retinacare-session-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    /// Copy the synthetic fixture next to a freshly written manifest so the
    /// checkpoint path resolves the way it does in `models/`.
    fn staged_manifest(directory: &Path) -> PathBuf {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
        fs::copy(&fixture, directory.join("tiny_mul.onnx")).unwrap();
        let sha256 = hex::encode(Sha256::digest(fs::read(&fixture).unwrap()));
        let path = directory.join("manifest.json");
        fs::write(
            &path,
            json!({
                "schema_version": 1,
                "model_id": "fixture-tiny-mul-nonmedical",
                "version": "1.0.0",
                "task": "synthetic_tensor_check",
                "architecture": "elementwise_mul_by_two",
                "checkpoint": "tiny_mul.onnx",
                "size_bytes": fs::metadata(directory.join("tiny_mul.onnx")).unwrap().len(),
                "sha256": sha256,
                "license": "CC0-1.0",
                "source_url": "https://github.com/Debaq/Dird",
                "input": {
                    "name": "image",
                    "data_type": "float32",
                    "shape": [1, 3, 4, 4],
                    "color_space": "RGB",
                    "normalization": "none"
                },
                "outputs": {
                    "schema": "float32_same_shape_as_input",
                    "name": "result",
                    "data_type": "float32",
                    "shape": [1, 3, 4, 4]
                },
                "validation": {"report": "synthetic fixture; no clinical validation"}
            })
            .to_string(),
        )
        .unwrap();
        path
    }

    #[test]
    fn loads_and_runs_the_synthetic_fixture_behind_the_gate() {
        let directory = scratch("run");
        let manifest = staged_manifest(&directory);
        let mut session = VerifiedSession::open(&manifest).unwrap();

        // metadata reported by the graph itself
        assert_eq!(session.model().model_id, "fixture-tiny-mul-nonmedical");
        assert_eq!(session.input_name(), "image");
        assert_eq!(session.output_name(), "result");
        assert_eq!(session.output_shape(), [1, 3, 4, 4]);
        assert_eq!(session.model().input_shape, vec![1, 3, 4, 4]);
        assert_eq!(session.actual_provider(), ActualProvider::Cpu);

        let image: Vec<f32> = (0..ELEMENTS).map(|index| index as f32).collect();
        let output = session.run(&image).unwrap();
        assert_eq!(output.len(), ELEMENTS);
        let expected: Vec<f32> = image.iter().map(|value| value * 2.0).collect();
        assert_eq!(output, expected);
    }

    #[test]
    fn rejects_an_invalid_manifest_before_reaching_onnx() {
        let directory = scratch("invalid-manifest");
        staged_manifest(&directory);
        let manifest = directory.join("manifest.json");
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
        value["model_id"] = json!("replace-with-verified-model-id");
        fs::write(&manifest, value.to_string()).unwrap();

        assert!(matches!(
            VerifiedSession::open(&manifest),
            Err(SessionError::Manifest(ManifestError::InvalidField(
                "model_id"
            )))
        ));
    }

    #[test]
    fn reports_a_missing_checkpoint_as_a_manifest_error() {
        let directory = scratch("missing-artifact");
        let manifest = staged_manifest(&directory);
        fs::remove_file(directory.join("tiny_mul.onnx")).unwrap();

        assert!(matches!(
            VerifiedSession::open(&manifest),
            Err(SessionError::Manifest(ManifestError::CheckpointMissing(_)))
        ));
    }

    #[test]
    fn reports_an_unsupported_model_file() {
        let directory = scratch("unsupported");
        let manifest = staged_manifest(&directory);
        let broken = directory.join("tiny_mul.onnx");
        fs::write(&broken, b"definitely not an onnx graph\n").unwrap();
        // keep the manifest honest so the failure comes from ONNX Runtime,
        // not from the hash check
        let sha256 = hex::encode(Sha256::digest(fs::read(&broken).unwrap()));
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
        value["sha256"] = json!(sha256);
        value["size_bytes"] = json!(fs::metadata(&broken).unwrap().len());
        fs::write(&manifest, value.to_string()).unwrap();

        assert!(matches!(
            VerifiedSession::open(&manifest),
            Err(SessionError::Build(_))
        ));
    }

    #[test]
    fn graph_metadata_that_differs_from_the_manifest_is_refused() {
        // A model whose real output shape, output name or input shape differs from what the manifest
        // promises must never reach inference (Stage I would otherwise decode the wrong tensor).
        for (pointer, replacement) in [
            ("/outputs/shape", json!([1, 3, 4, 5])),
            ("/outputs/name", json!("logit")),
            ("/input/shape", json!([1, 3, 8, 8])),
            ("/input/name", json!("pixels")),
        ] {
            let directory = scratch("metadata-mismatch");
            let manifest = staged_manifest(&directory);
            let mut value: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
            *value.pointer_mut(pointer).unwrap() = replacement;
            fs::write(&manifest, value.to_string()).unwrap();
            assert!(
                matches!(
                    VerifiedSession::open(&manifest),
                    Err(SessionError::InputMismatch(_))
                ),
                "{pointer}"
            );
        }
    }

    #[test]
    fn reports_a_runtime_failure_on_a_wrong_sized_input() {
        let directory = scratch("run-failure");
        let manifest = staged_manifest(&directory);
        let mut session = VerifiedSession::open(&manifest).unwrap();
        let short = vec![0.0; ELEMENTS - 1];
        assert!(matches!(
            session.run(&short),
            Err(SessionError::InputSize { expected: ELEMENTS, actual }) if actual == ELEMENTS - 1
        ));
    }

    #[test]
    fn rejects_non_finite_input_before_onnx_runtime() {
        let directory = scratch("non-finite-input");
        let manifest = staged_manifest(&directory);
        let mut session = VerifiedSession::open(&manifest).unwrap();
        let mut input = vec![0.0; ELEMENTS];
        input[17] = f32::NAN;
        assert!(matches!(
            session.run(&input),
            Err(SessionError::NonFiniteInput)
        ));
    }

    #[test]
    fn refuses_pytorch_checkpoints_without_deserializing_them() {
        let directory = scratch("pytorch-not-loaded");
        let manifest = staged_manifest(&directory);
        let checkpoint = directory.join("tiny_mul.onnx");
        let pytorch = directory.join("tiny_mul.pth");
        fs::rename(&checkpoint, &pytorch).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();
        value["checkpoint"] = json!("tiny_mul.pth");
        fs::write(&manifest, value.to_string()).unwrap();
        assert!(matches!(
            VerifiedSession::open(&manifest),
            Err(SessionError::ConversionRequired)
        ));
    }

    #[cfg(not(feature = "cuda"))]
    #[test]
    fn explicit_cuda_request_errors_when_cuda_is_not_compiled() {
        let directory = scratch("cuda-not-compiled");
        let manifest = staged_manifest(&directory);
        assert!(matches!(
            VerifiedSession::open_with_provider(&manifest, ProviderRequest::Cuda { device_id: 0 }),
            Err(SessionError::CudaNotCompiled)
        ));
    }

    /// Deterministic f32 input identical to `lcg_input()` in
    /// `scripts/stage1_export_parity.py`: u32 LCG, advance first, then
    /// `(state as f32) / 2^32 * 2 - 1`.
    fn lcg_input(seed: u32, mul: u32, add: u32) -> Vec<f32> {
        // elements of [1, 3, 768, 768]
        let total = 3 * 768 * 768;
        let mut state = seed;
        let mut values = Vec::with_capacity(total);
        for _ in 0..total {
            state = state.wrapping_mul(mul).wrapping_add(add);
            let scaled = (state as f32) / 4294967296.0;
            values.push(scaled * 2.0 - 1.0);
        }
        values
    }

    fn sigmoid(value: f32) -> f32 {
        1.0 / (1.0 + (-value).exp())
    }

    /// Authentic HierarchiRetina Stage I gate against the Python-generated golden.
    ///
    /// Opt-in: needs the approved 1.61 GB checkpoint plus
    /// `scripts/stage1_export_parity.py` (both git-ignored artifacts).
    ///
    /// Run: `cargo test -p retinacare-ai-bridge -- --ignored`
    #[test]
    #[ignore = "needs models/stage1_convnextv2l_768 + stage1_golden.json; run scripts/stage1_export_parity.py first"]
    fn authentic_stage1_matches_python_golden() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = root.join("../../../models/stage1_convnextv2l_768/manifest.json");
        let golden_file = root.join("fixtures/stage1_golden.json");
        assert!(
            manifest.is_file(),
            "manifest missing: {manifest:?} — run scripts/stage1_export_parity.py first"
        );
        let golden: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(&golden_file).expect("stage1_golden.json missing"),
        )
        .unwrap();
        let raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&manifest).unwrap()).unwrap();

        // Provenance: exact upstream checkpoint hash, read from the manifest the gate verifies.
        assert_eq!(raw["model_id"], "hierarchiretina-stage1-convnextv2l-768");
        assert_eq!(
            raw["validation"]["upstream_checkpoint_sha256"],
            "89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff"
        );

        let mut runtime = crate::model_runtime::ModelRuntime::default();
        let loaded = runtime
            .load(&manifest, ProviderRequest::Cpu)
            .expect("verified Stage I model should load");
        assert_eq!(loaded.input_shape, vec![1, 3, 768, 768]);
        assert_eq!(loaded.model_id, "hierarchiretina-stage1-convnextv2l-768");
        assert_eq!(loaded.input_name, "image");
        assert_eq!(loaded.output_name, "logit");
        assert_eq!(
            loaded.sha256,
            golden["onnx_sha256"].as_str().unwrap(),
            "golden belongs to a different ONNX export"
        );
        assert_eq!(
            golden["upstream_checkpoint_sha256"].as_str().unwrap(),
            raw["validation"]["upstream_checkpoint_sha256"]
                .as_str()
                .unwrap()
        );

        let threshold = golden["gate_threshold"].as_f64().unwrap() as f32;
        assert_eq!(threshold, 0.239_171_6_f32, "GATE_THRESHOLD from config.py");
        let tolerance = golden["parity_tolerance"].as_f64().unwrap() as f32;
        let mul = golden["lcg"]["mul"].as_u64().unwrap() as u32;
        let add = golden["lcg"]["add"].as_u64().unwrap() as u32;

        let mut lcg_cases = 0;
        for case in golden["cases"].as_array().unwrap() {
            // The OpenCV-preprocessed case has no Rust reproduction yet, so only
            // LCG cases (bit-reproducible on both sides) are checked here.
            let Some(seed) = case["input"]["seed"].as_u64() else {
                continue;
            };
            let input = lcg_input(seed as u32, mul, add);
            let output = runtime
                .infer_stage1_gate(&loaded.model_id, &input)
                .expect("Stage I typed gate inference should succeed");
            let expected = case["onnxruntime_logit"].as_f64().unwrap() as f32;
            let diff = (output.gate.raw_logit - expected).abs();
            assert!(
                diff <= tolerance,
                "{}: rust logit {} vs python {} (|Δ| = {diff} > {tolerance})",
                case["name"].as_str().unwrap(),
                output.gate.raw_logit,
                expected
            );
            let decision =
                output.gate.decision == crate::stage1::Stage1GateDecision::ContinueCascade;
            assert_eq!(output.gate.threshold, threshold);
            assert_eq!(output.gate.screening_score, sigmoid(output.gate.raw_logit));
            assert_eq!(output.provider, crate::device::ActualProvider::Cpu);
            assert_eq!(output.sha256, loaded.sha256);
            assert!(output.elapsed_microseconds > 0);
            assert_eq!(
                decision,
                case["decision_positive"].as_bool().unwrap(),
                "threshold decision for {}",
                case["name"].as_str().unwrap()
            );
            lcg_cases += 1;
        }
        assert!(lcg_cases >= 1, "golden must carry a Rust-reproducible case");
    }
}
