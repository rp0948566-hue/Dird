//! Loaded-session lifecycle for verified, single-input/single-output ONNX models.
use crate::{
    device::ProviderRequest,
    onnx_session::{SessionError, VerifiedSession},
};
use std::time::Instant;
use std::{collections::HashMap, path::Path};
use thiserror::Error;

use crate::preprocessing::{
    PREPROCESSING_VERSION, PreprocessError, PreprocessedStage1Image, preprocess_stage1_image,
};
use crate::stage1::{DecodedStage1Gate, Stage1DecodeError, decode_stage1_gate};
use crate::{
    stage2_contract::{
        MaskPixelData, Stage2InferenceError, Stage2MaskResult, Stage2ModelKind, Stage2Request,
        ThresholdMetadata,
    },
    stage2_preprocessing::{preprocess_stage2_lesion_image, restore_stage2_lesion_mask},
};

#[derive(Debug, Error)]
pub enum Stage1InferenceError {
    #[error(transparent)]
    Runtime(#[from] ModelRuntimeError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("loaded model `{0}` does not satisfy the Stage I single-logit gate contract")]
    IncompatibleModel(String),
    #[error(transparent)]
    Decode(#[from] Stage1DecodeError),
    #[error(transparent)]
    Preprocessing(#[from] PreprocessError),
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stage1GateOutput {
    pub model_id: String,
    pub version: String,
    pub sha256: String,
    pub provider: crate::device::ActualProvider,
    pub preprocessing_version: String,
    pub image: Option<Stage1ImageMetadata>,
    pub preprocessing_microseconds: Option<u128>,
    pub elapsed_microseconds: u128,
    #[serde(flatten)]
    pub gate: DecodedStage1Gate,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Stage1ImageMetadata {
    pub sha256: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Error)]
pub enum ModelRuntimeError {
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error("model `{0}` is already loaded")]
    AlreadyLoaded(String),
    #[error("model `{0}` is not loaded")]
    NotLoaded(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedModel {
    pub model_id: String,
    pub version: String,
    pub sha256: String,
    pub input_name: String,
    pub input_shape: Vec<usize>,
    pub output_name: String,
    pub output_shape: Vec<i64>,
    pub provider: crate::device::ActualProvider,
}

#[derive(Default)]
pub struct ModelRuntime {
    sessions: HashMap<String, VerifiedSession>,
}

impl ModelRuntime {
    pub fn load(
        &mut self,
        manifest_path: &Path,
        provider: ProviderRequest,
    ) -> Result<LoadedModel, ModelRuntimeError> {
        let session = VerifiedSession::open_with_provider(manifest_path, provider)?;
        let model_id = session.model().model_id.clone();
        if self.sessions.contains_key(&model_id) {
            return Err(ModelRuntimeError::AlreadyLoaded(model_id));
        }
        let loaded = LoadedModel {
            model_id: model_id.clone(),
            version: session.model().version.clone(),
            sha256: session.model().sha256.clone(),
            input_name: session.input_name().to_owned(),
            input_shape: session.model().input_shape.clone(),
            output_name: session.output_name().to_owned(),
            output_shape: session.output_shape().to_vec(),
            provider: session.actual_provider(),
        };
        self.sessions.insert(model_id, session);
        Ok(loaded)
    }

    pub fn infer(&mut self, model_id: &str, input: &[f32]) -> Result<Vec<f32>, ModelRuntimeError> {
        self.sessions
            .get_mut(model_id)
            .ok_or_else(|| ModelRuntimeError::NotLoaded(model_id.to_owned()))?
            .run(input)
            .map_err(Into::into)
    }

    pub fn infer_stage1_gate(
        &mut self,
        model_id: &str,
        input: &[f32],
    ) -> Result<Stage1GateOutput, Stage1InferenceError> {
        let session = self
            .sessions
            .get_mut(model_id)
            .ok_or_else(|| ModelRuntimeError::NotLoaded(model_id.to_owned()))?;
        let model = session.model();
        if model.task != "dr_screening_gate"
            || model.input_shape != [1, 3, 768, 768]
            || model.output_schema != "single_logit"
            || model.output_shape != [1, 1]
        {
            return Err(Stage1InferenceError::IncompatibleModel(model_id.to_owned()));
        }
        let identity = (
            model.model_id.clone(),
            model.version.clone(),
            session.model().sha256.clone(),
            session.actual_provider(),
        );
        let started = Instant::now();
        let logits = session.run(input)?;
        let gate = decode_stage1_gate(&logits)?;
        Ok(Stage1GateOutput {
            model_id: identity.0,
            version: identity.1,
            sha256: identity.2,
            provider: identity.3,
            preprocessing_version:
                "hierarchiretina-v1.0-preprocess_gate (caller supplied; parity unverified)".into(),
            image: None,
            preprocessing_microseconds: None,
            elapsed_microseconds: started.elapsed().as_micros(),
            gate,
        })
    }

    pub fn infer_stage1_image(
        &mut self,
        model_id: &str,
        image_path: &Path,
    ) -> Result<Stage1GateOutput, Stage1InferenceError> {
        if !self.sessions.contains_key(model_id) {
            return Err(ModelRuntimeError::NotLoaded(model_id.to_owned()).into());
        }
        let started = Instant::now();
        let prepared = preprocess_stage1_image(image_path)?;
        let mut result = self.infer_stage1_gate(model_id, &prepared.tensor)?;
        attach_image_metadata(&mut result, &prepared);
        let total_microseconds = started.elapsed().as_micros();
        result.preprocessing_microseconds =
            Some(total_microseconds.saturating_sub(result.elapsed_microseconds));
        result.elapsed_microseconds = total_microseconds;
        Ok(result)
    }

    /// Run the verified MA ONNX session and return its registered full-frame mask.
    pub fn infer_stage2_image(
        &mut self,
        request: &Stage2Request,
    ) -> Result<Stage2MaskResult, Stage2InferenceError> {
        request.validate()?;
        let session = self
            .sessions
            .get_mut(&request.model_id)
            .ok_or_else(|| ModelRuntimeError::NotLoaded(request.model_id.clone()))?;
        let model = session.model();
        let (expected_model_id, expected_architecture, expected_task, expected_checkpoint_hash) =
            match request.model_kind {
                Stage2ModelKind::Ma => (
                    "hierarchiretina-stage2-ma-hsmoe-aunet",
                    "HSMoE-AUNet-MA",
                    "microaneurysm_segmentation",
                    "42c776e9b1bc04f6fb267a3e325a0188a1fad54097293961f3e6be1200b092f8",
                ),
                Stage2ModelKind::He => (
                    "hierarchiretina-stage2-he-hsmoe-aunet",
                    "HSMoE-AUNet-HE",
                    "hemorrhage_segmentation",
                    "c8fc44d5b14f54cb26c8418d6e40c61c283bf7daae7a1b3fd2ff31923371d94a",
                ),
                _ => return Err(Stage2InferenceError::IncompatibleModel(request.model_kind)),
            };
        let checkpoint_sha256 = model
            .validation
            .as_ref()
            .and_then(|value| value.get("checkpoint_sha256"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned);
        let source_commit = model
            .validation
            .as_ref()
            .and_then(|value| value.get("source_commit"))
            .and_then(serde_json::Value::as_str);
        if model.model_id != expected_model_id
            || model.version != "1.0.0"
            || model.architecture != expected_architecture
            || model.task != expected_task
            || model.input_shape != [1, 3, 1024, 1024]
            || model.output_schema != "main_logits"
            || model.output_shape != [1, 1, 1024, 1024]
            || checkpoint_sha256.as_deref() != Some(expected_checkpoint_hash)
            || source_commit != Some("bf84b1d3c7409d2575dff84c7575c67577493a30")
        {
            return Err(Stage2InferenceError::IncompatibleModel(request.model_kind));
        }
        let model_version = model.version.clone();
        let model_sha256 = session.model().sha256.clone();
        let provider = session.actual_provider();
        let started = Instant::now();
        let prepared = preprocess_stage2_lesion_image(&request.image_path, request.model_kind)?;
        let logits = session.run(&prepared.tensor)?;
        let mask = restore_stage2_lesion_mask(
            &logits,
            &prepared.transform,
            prepared.original_width,
            prepared.original_height,
            request.model_kind.threshold(),
        )?;
        let result = Stage2MaskResult {
            processing_status: crate::stage2_contract::Stage2ProcessingStatus::Complete,
            error: None,
            examination_id: request.examination_id,
            source_image_sha256: prepared.image_sha256,
            laterality: request.laterality,
            model_kind: request.model_kind,
            model_version,
            checkpoint_sha256: checkpoint_sha256
                .ok_or(Stage2InferenceError::IncompatibleModel(request.model_kind))?,
            model_sha256,
            original_width: prepared.original_width,
            original_height: prepared.original_height,
            mask_width: prepared.original_width,
            mask_height: prepared.original_height,
            transform: prepared.transform,
            preprocessing_version: prepared.preprocessing_version.into(),
            threshold: ThresholdMetadata {
                value: request.model_kind.threshold(),
                operator: request.model_kind.threshold_operator(),
            },
            provider,
            elapsed_microseconds: started.elapsed().as_micros(),
            data: MaskPixelData::Binary(mask),
        };
        result.validate()?;
        Ok(result)
    }

    /// Drop the session and release its runtime-owned resources.
    pub fn unload(&mut self, model_id: &str) -> bool {
        self.sessions.remove(model_id).is_some()
    }

    pub fn loaded_models(&self) -> Vec<String> {
        let mut models: Vec<_> = self.sessions.keys().cloned().collect();
        models.sort();
        models
    }
}

fn attach_image_metadata(result: &mut Stage1GateOutput, prepared: &PreprocessedStage1Image) {
    result.preprocessing_version = PREPROCESSING_VERSION.to_owned();
    result.image = Some(Stage1ImageMetadata {
        sha256: prepared.image_sha256.clone(),
        width: prepared.original_width,
        height: prepared.original_height,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_id_returns_a_typed_lifecycle_error() {
        let mut runtime = ModelRuntime::default();
        assert!(matches!(
            runtime.infer("missing", &[]),
            Err(ModelRuntimeError::NotLoaded(id)) if id == "missing"
        ));
        assert!(!runtime.unload("missing"));
        assert!(runtime.loaded_models().is_empty());
    }

    #[test]
    fn missing_model_rejects_image_inference_before_reading_the_image() {
        let mut runtime = ModelRuntime::default();
        let result = runtime.infer_stage1_image("missing", Path::new("does-not-exist.png"));
        assert!(matches!(
            result,
            Err(Stage1InferenceError::Runtime(ModelRuntimeError::NotLoaded(id))) if id == "missing"
        ));
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE2_MA_MANIFEST and RETINACARE_STAGE2_MA_IMAGE"]
    fn local_ma_checkpoint_runs_to_registered_original_frame_mask() {
        verify_local_lesion_runtime(Stage2ModelKind::Ma, "MA");
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE2_HE_MANIFEST and RETINACARE_STAGE2_HE_IMAGE"]
    fn local_he_checkpoint_runs_to_registered_original_frame_mask() {
        verify_local_lesion_runtime(Stage2ModelKind::He, "HE");
    }

    fn verify_local_lesion_runtime(model_kind: Stage2ModelKind, model_code: &str) {
        let prefix = model_code.to_ascii_lowercase();
        let parity_dir = std::path::PathBuf::from(
            std::env::var(format!("RETINACARE_STAGE2_{model_code}_PARITY_DIR"))
                .expect("set the temporary Python parity-fixture directory"),
        );
        let manifest = Path::new(
            &std::env::var(format!("RETINACARE_STAGE2_{model_code}_MANIFEST"))
                .expect("set path to the verified temporary lesion manifest"),
        )
        .to_path_buf();
        let image_path = Path::new(
            &std::env::var(format!("RETINACARE_STAGE2_{model_code}_IMAGE"))
                .expect("set path to the local-only retinal technical test image"),
        )
        .to_path_buf();
        let mut runtime = ModelRuntime::default();
        let loaded = runtime.load(&manifest, ProviderRequest::Cpu).unwrap();
        let prepared =
            crate::stage2_preprocessing::preprocess_stage2_lesion_image(&image_path, model_kind)
                .unwrap();
        let reference_input =
            std::fs::read(parity_dir.join(format!("{prefix}_input.f32"))).unwrap();
        assert_eq!(reference_input.len(), prepared.tensor.len() * 4);
        let max_input_diff = prepared
            .tensor
            .iter()
            .zip(reference_input.chunks(4))
            .map(|(rust, bytes)| {
                let python = f32::from_le_bytes(bytes.try_into().unwrap());
                (rust - python).abs()
            })
            .fold(0.0f32, f32::max);
        assert!(max_input_diff <= 1e-5, "tensor max diff {max_input_diff}");
        let reference_logits =
            std::fs::read(parity_dir.join(format!("{prefix}_pytorch_logits.f32"))).unwrap();
        let rust_logits = runtime.infer(&loaded.model_id, &prepared.tensor).unwrap();
        assert_eq!(reference_logits.len(), rust_logits.len() * 4);
        let max_logit_diff = rust_logits
            .iter()
            .zip(reference_logits.chunks(4))
            .map(|(rust, bytes)| {
                let python = f32::from_le_bytes(bytes.try_into().unwrap());
                (rust - python).abs()
            })
            .fold(0.0f32, f32::max);
        assert!(max_logit_diff <= 1e-3, "logit max diff {max_logit_diff}");
        let logit_squared_error_sum = rust_logits
            .iter()
            .zip(reference_logits.chunks(4))
            .map(|(rust, bytes)| {
                let python = f32::from_le_bytes(bytes.try_into().unwrap());
                f64::from(rust - python).powi(2)
            })
            .sum::<f64>();
        let logit_rmse = (logit_squared_error_sum / rust_logits.len() as f64).sqrt();
        eprintln!(
            "Stage II {model_code} parity: tensor max_abs={max_input_diff:.9}, logits max_abs={max_logit_diff:.9}, logits RMSE={logit_rmse:.9}"
        );
        let result = runtime
            .infer_stage2_image(&Stage2Request {
                examination_id: 7,
                image_path,
                laterality: None,
                model_kind,
                model_id: loaded.model_id,
            })
            .unwrap();
        result.validate().unwrap();
        assert_eq!(result.model_kind, model_kind);
        assert_eq!(result.mask_width, result.original_width);
        assert_eq!(result.mask_height, result.original_height);
        assert!(matches!(
            result.transform,
            crate::stage2_contract::Stage2SpatialTransform::LesionCircleStretch { .. }
        ));
        let reference_mask =
            std::fs::read(parity_dir.join(format!("{prefix}_pytorch_mask.u8"))).unwrap();
        let crate::stage2_contract::MaskPixelData::Binary(rust_mask) = result.data else {
            panic!("expected binary lesion mask");
        };
        eprintln!(
            "Stage II {model_code} Rust result: source_sha256={}, model_sha256={}, frame={}x{}, foreground_pixels={}, total_us={}",
            result.source_image_sha256,
            result.model_sha256,
            result.original_width,
            result.original_height,
            rust_mask.iter().filter(|&&pixel| pixel == 255).count(),
            result.elapsed_microseconds,
        );
        assert_eq!(
            rust_mask, reference_mask,
            "Rust/PyTorch restored mask mismatch for {model_code}"
        );
    }
}
