//! Loaded-session lifecycle for verified, single-input/single-output ONNX models.
use crate::stage2_preprocessing::read_stage2_source;
use crate::stage2_vessel::{
    VESSEL_PREPROCESSING_VERSION, preprocess_stage2_vessel_image, restore_stage2_vessel_mask,
    vessel_tta_probabilities,
};
use crate::stage3_grading::{
    CORN_THRESHOLDS, FOLD_CHECKPOINT_SHA256, GRADABILITY_THRESHOLD, Stage3GradeResult,
    Stage3InferenceError, Stage3Request, Stage3Scope, grade_single_fold,
};
use crate::stage3_input::assemble_stage3_input;
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
    stage2_preprocessing::{
        Stage2MaskPostprocess, flip_planes, preprocess_stage2_lesion_image,
        restore_stage2_probability_mask_with, stage2_probabilities_from_logits,
    },
};

/// Pin an LG-DRG session to its fold id, checkpoint hash, upstream commit, graph and I/O shapes.
pub(crate) fn verify_stage3_fold_model(
    model: &crate::model_manifest::VerifiedModel,
) -> Result<usize, Stage3InferenceError> {
    let fold = (0..FOLD_CHECKPOINT_SHA256.len())
        .find(|fold| model.model_id == format!("hierarchiretina-stage3-lgdrg-fold{fold}"))
        .ok_or(Stage3InferenceError::IncompatibleModel)?;
    let checkpoint_sha256 = model
        .validation
        .as_ref()
        .and_then(|value| value.get("checkpoint_sha256"))
        .and_then(serde_json::Value::as_str);
    let source_commit = model
        .validation
        .as_ref()
        .and_then(|value| value.get("source_commit"))
        .and_then(serde_json::Value::as_str);
    if model.version != "1.0.0"
        || model.architecture != "LG-DRG-ConvNeXtV2-L"
        || model.task != "lg_drg_grading"
        || model.input_shape != [1, 8, 512, 512]
        || model.output_schema != "lgdrg_logits"
        || model.output_shape != [1, 4]
        || checkpoint_sha256 != Some(FOLD_CHECKPOINT_SHA256[fold])
        || source_commit != Some("bf84b1d3c7409d2575dff84c7575c67577493a30")
    {
        return Err(Stage3InferenceError::IncompatibleModel);
    }
    Ok(fold)
}

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
        let side = request.model_kind.input_size() as usize;
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
                Stage2ModelKind::Cws => (
                    "hierarchiretina-stage2-cws-hsmoe-aunet",
                    "HSMoE-AUNet-CWS",
                    "cotton_wool_spot_segmentation",
                    "c42c9b7754c71c474d9f58587d3fbe63e17deb5611a3f5656e76f0415bd5bba2",
                ),
                Stage2ModelKind::Ex => (
                    "hierarchiretina-stage2-ex-hsmoe-aunet",
                    "HSMoE-AUNet-EX",
                    "hard_exudate_segmentation",
                    "6a4bd1b40749475dbb36a1812e08d5caf3a4fce5d2c90892c0f53ab55e64453e",
                ),
                Stage2ModelKind::Vessel => (
                    "hierarchiretina-stage2-vessel-swinhrunetpp",
                    "SwinHRUNet++-vessel",
                    "vessel_segmentation",
                    "32bad65f3a1cf099e847fd292ff5ff84199ed0882112c0a22499e423e481422c",
                ),
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
            || model.input_shape != [1, 3, side, side]
            || model.output_schema != "main_logits"
            || model.output_shape != [1, 1, side as i64, side as i64]
            || checkpoint_sha256.as_deref() != Some(expected_checkpoint_hash)
            || source_commit != Some("bf84b1d3c7409d2575dff84c7575c67577493a30")
        {
            return Err(Stage2InferenceError::IncompatibleModel(request.model_kind));
        }
        let model_version = model.version.clone();
        let model_sha256 = session.model().sha256.clone();
        let provider = session.actual_provider();
        let started = Instant::now();
        let (image_sha256, original_width, original_height, transform, preprocessing_version, mask) =
            if request.model_kind == Stage2ModelKind::Vessel {
                let prepared = preprocess_stage2_vessel_image(&request.image_path)?;
                let probabilities = vessel_tta_probabilities::<Stage2InferenceError>(
                    |input| Ok(session.run(input)?),
                    &prepared.tensor,
                )?;
                let mask = restore_stage2_vessel_mask(
                    &probabilities,
                    &prepared.model_fov,
                    &prepared.transform,
                    &prepared.frame_fov,
                    prepared.original_width,
                    prepared.original_height,
                    request.model_kind.threshold(),
                )?;
                (
                    prepared.image_sha256,
                    prepared.original_width,
                    prepared.original_height,
                    prepared.transform,
                    VESSEL_PREPROCESSING_VERSION,
                    mask,
                )
            } else {
                let prepared =
                    preprocess_stage2_lesion_image(&request.image_path, request.model_kind)?;
                let probabilities = if request.model_kind.uses_flip_tta() {
                    // Upstream EX and CWS: mean of identity, horizontal-flip and vertical-flip sigmoid maps.
                    let identity =
                        stage2_probabilities_from_logits(&session.run(&prepared.tensor)?)?;
                    let horizontal = stage2_probabilities_from_logits(&flip_planes(
                        &session.run(&flip_planes(&prepared.tensor, 3, true))?,
                        1,
                        true,
                    ))?;
                    let vertical = stage2_probabilities_from_logits(&flip_planes(
                        &session.run(&flip_planes(&prepared.tensor, 3, false))?,
                        1,
                        false,
                    ))?;
                    identity
                        .iter()
                        .zip(horizontal.iter().zip(vertical.iter()))
                        .map(|(a, (b, c))| (a + b + c) / 3.0)
                        .collect()
                } else {
                    stage2_probabilities_from_logits(&session.run(&prepared.tensor)?)?
                };
                let postprocess = if request.model_kind == Stage2ModelKind::Cws {
                    Stage2MaskPostprocess::CwsComponents
                } else {
                    Stage2MaskPostprocess::Threshold
                };
                let mask = restore_stage2_probability_mask_with(
                    &probabilities,
                    &prepared.transform,
                    prepared.original_width,
                    prepared.original_height,
                    request.model_kind.threshold(),
                    postprocess,
                )?;
                (
                    prepared.image_sha256,
                    prepared.original_width,
                    prepared.original_height,
                    prepared.transform,
                    prepared.preprocessing_version,
                    mask,
                )
            };
        let result = Stage2MaskResult {
            processing_status: crate::stage2_contract::Stage2ProcessingStatus::Complete,
            error: None,
            examination_id: request.examination_id,
            source_image_sha256: image_sha256,
            laterality: request.laterality,
            model_kind: request.model_kind,
            model_version,
            checkpoint_sha256: checkpoint_sha256
                .ok_or(Stage2InferenceError::IncompatibleModel(request.model_kind))?,
            model_sha256,
            original_width,
            original_height,
            mask_width: original_width,
            mask_height: original_height,
            transform,
            preprocessing_version: preprocessing_version.into(),
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

    /// Run one authentic LG-DRG fold on the eight-channel input assembled from verified Stage II masks.
    /// The result is a SINGLE-FOLD TECHNICAL TEST, not the final five-fold ensemble.
    pub fn infer_stage3_fold(
        &mut self,
        request: &Stage3Request,
    ) -> Result<Stage3GradeResult, Stage3InferenceError> {
        if request.examination_id <= 0 {
            return Err(Stage3InferenceError::InvalidRequest(
                "examination id must be positive",
            ));
        }
        let session = self
            .sessions
            .get_mut(&request.model_id)
            .ok_or_else(|| ModelRuntimeError::NotLoaded(request.model_id.clone()))?;
        let model = session.model();
        let fold = verify_stage3_fold_model(model)?;
        let model_version = model.version.clone();
        let model_sha256 = model.sha256.clone();
        let provider = session.actual_provider();
        let started = Instant::now();
        let encoded = read_stage2_source(&request.image_path)?;
        let input = assemble_stage3_input(&encoded, request.examination_id, &request.masks)?;
        let logits = session.run(&input.tensor)?;
        let decoded = grade_single_fold(&logits)?;
        Ok(Stage3GradeResult {
            examination_id: request.examination_id,
            source_image_sha256: input.image_sha256,
            scope: Stage3Scope::SingleFoldTechnicalTest { fold: fold as u8 },
            model_version,
            checkpoint_sha256: vec![FOLD_CHECKPOINT_SHA256[fold].to_owned()],
            model_sha256: vec![model_sha256],
            severity_logits: Some(decoded.severity_logits),
            gradability_logit: Some(decoded.gradability_logit),
            full_probabilities: decoded.full,
            ordinal_probabilities: [
                decoded.probabilities[0],
                decoded.probabilities[1],
                decoded.probabilities[2],
            ],
            gradable_probability: decoded.probabilities[3],
            corn_thresholds: CORN_THRESHOLDS,
            gradability_threshold: GRADABILITY_THRESHOLD,
            grade: decoded.grade,
            ungradable: decoded.grade == 5,
            provider,
            elapsed_microseconds: started.elapsed().as_micros(),
            research_only: true,
        })
    }

    pub(crate) fn session_mut(
        &mut self,
        model_id: &str,
    ) -> Result<&mut VerifiedSession, ModelRuntimeError> {
        self.sessions
            .get_mut(model_id)
            .ok_or_else(|| ModelRuntimeError::NotLoaded(model_id.to_owned()))
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

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE2_EX_MANIFEST, RETINACARE_STAGE2_EX_IMAGE and RETINACARE_STAGE2_EX_PARITY_DIR"]
    fn local_ex_checkpoint_runs_to_registered_letterboxed_mask() {
        verify_local_rect_lesion_runtime(Stage2ModelKind::Ex, "EX");
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE2_CWS_MANIFEST, RETINACARE_STAGE2_CWS_IMAGE and RETINACARE_STAGE2_CWS_PARITY_DIR"]
    fn local_cws_checkpoint_runs_to_postprocessed_letterboxed_mask() {
        verify_local_rect_lesion_runtime(Stage2ModelKind::Cws, "CWS");
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE2_VESSEL_MANIFEST, RETINACARE_STAGE2_VESSEL_IMAGE and RETINACARE_STAGE2_VESSEL_PARITY_DIR"]
    fn local_vessel_checkpoint_runs_to_registered_fov_mask() {
        let parity_dir = std::path::PathBuf::from(
            std::env::var("RETINACARE_STAGE2_VESSEL_PARITY_DIR")
                .expect("set the temporary Python vessel parity-fixture directory"),
        );
        let manifest = Path::new(
            &std::env::var("RETINACARE_STAGE2_VESSEL_MANIFEST")
                .expect("set path to the verified temporary vessel manifest"),
        )
        .to_path_buf();
        let image_path = Path::new(
            &std::env::var("RETINACARE_STAGE2_VESSEL_IMAGE")
                .expect("set path to the local-only retinal technical test image"),
        )
        .to_path_buf();
        let mut runtime = ModelRuntime::default();
        let loaded = runtime.load(&manifest, ProviderRequest::Cpu).unwrap();
        let prepared = crate::stage2_vessel::preprocess_stage2_vessel_image(&image_path).unwrap();
        let reference_input = std::fs::read(parity_dir.join("vessel_input.f32")).unwrap();
        assert_eq!(reference_input.len(), prepared.tensor.len() * 4);
        let max_input_diff = prepared
            .tensor
            .iter()
            .zip(reference_input.chunks(4))
            .map(|(rust, bytes)| (rust - f32::from_le_bytes(bytes.try_into().unwrap())).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_input_diff <= 1e-5,
            "vessel tensor max diff {max_input_diff}"
        );
        let result = runtime
            .infer_stage2_image(&Stage2Request {
                examination_id: 13,
                image_path,
                laterality: None,
                model_kind: Stage2ModelKind::Vessel,
                model_id: loaded.model_id,
            })
            .unwrap();
        result.validate().unwrap();
        assert!(matches!(
            result.transform,
            crate::stage2_contract::Stage2SpatialTransform::VesselFovSquare { .. }
        ));
        let reference_mask = std::fs::read(parity_dir.join("vessel_pytorch_mask.u8")).unwrap();
        let crate::stage2_contract::MaskPixelData::Binary(rust_mask) = result.data else {
            panic!("expected binary vessel mask");
        };
        let disagreements = rust_mask
            .iter()
            .zip(reference_mask.iter())
            .filter(|(rust, python)| rust != python)
            .count();
        eprintln!(
            "Stage II VESSEL: tensor max_abs={max_input_diff:.9}, mask disagreement px={disagreements}, foreground_pixels={}, frame={}x{}, total_us={}",
            rust_mask.iter().filter(|&&pixel| pixel == 255).count(),
            result.original_width,
            result.original_height,
            result.elapsed_microseconds,
        );
        assert_eq!(disagreements, 0, "Rust/PyTorch vessel mask mismatch");
    }

    #[test]
    fn stage3_rejects_bad_examination_and_unloaded_model_before_reading_the_image() {
        let mut runtime = ModelRuntime::default();
        let request = |examination_id| Stage3Request {
            examination_id,
            image_path: Path::new("does-not-exist.png").into(),
            masks: Vec::new(),
            model_id: "missing".into(),
        };
        assert!(matches!(
            runtime.infer_stage3_fold(&request(0)),
            Err(Stage3InferenceError::InvalidRequest(_))
        ));
        assert!(matches!(
            runtime.infer_stage3_fold(&request(5)),
            Err(Stage3InferenceError::Runtime(ModelRuntimeError::NotLoaded(id))) if id == "missing"
        ));
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE3_MANIFEST, RETINACARE_STAGE3_IMAGE and RETINACARE_STAGE3_MASK_DIR"]
    fn local_stage3_single_fold_grades_demo_image_and_matches_pytorch_logits() {
        use crate::stage3_input::tests::mask_result;
        use sha2::{Digest, Sha256};
        let manifest = std::path::PathBuf::from(
            std::env::var("RETINACARE_STAGE3_MANIFEST").expect("set the LG-DRG fold manifest path"),
        );
        let image_path = std::path::PathBuf::from(
            std::env::var("RETINACARE_STAGE3_IMAGE").expect("set the local technical test image"),
        );
        let mask_dir = std::path::PathBuf::from(
            std::env::var("RETINACARE_STAGE3_MASK_DIR")
                .expect("set the raw Stage II mask directory"),
        );
        let encoded = std::fs::read(&image_path).unwrap();
        let sha = hex::encode(Sha256::digest(&encoded));
        let side = 640u32; // the technical demo frame is square
        let masks: Vec<_> = crate::stage3_input::STAGE3_MASK_ORDER
            .iter()
            .zip(["ma", "he", "ex", "cws", "vessel"])
            .map(|(kind, name)| {
                let pixels =
                    std::fs::read(mask_dir.join(format!("{name}_pytorch_mask.u8"))).unwrap();
                mask_result(*kind, 21, &sha, side, pixels)
            })
            .collect();
        let mut runtime = ModelRuntime::default();
        let loaded = runtime.load(&manifest, ProviderRequest::Cpu).unwrap();
        let result = runtime
            .infer_stage3_fold(&Stage3Request {
                examination_id: 21,
                image_path,
                masks,
                model_id: loaded.model_id,
            })
            .unwrap();
        let manifest_json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        let cases = manifest_json["validation"]["pytorch_vs_onnxruntime_cpu"]["cases"]
            .as_array()
            .unwrap();
        let case = cases
            .iter()
            .find(|case| case["input"].as_str().unwrap().starts_with("demo image"))
            .expect("manifest records the demo-image PyTorch reference");
        let reference: Vec<f64> = case["pytorch_logits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_f64().unwrap())
            .collect();
        let rust = [
            result.severity_logits.unwrap()[0],
            result.severity_logits.unwrap()[1],
            result.severity_logits.unwrap()[2],
            result.gradability_logit.unwrap(),
        ];
        let worst = rust
            .iter()
            .zip(&reference)
            .map(|(a, b)| (f64::from(*a) - b).abs())
            .fold(0.0f64, f64::max);
        eprintln!(
            "Stage III fold {:?}: logits={rust:?} max_abs_vs_pytorch={worst:.9} grade={} ungradable={} p_gradable={:.6} provider={:?} total_us={}",
            result.scope,
            result.grade,
            result.ungradable,
            result.gradable_probability,
            result.provider,
            result.elapsed_microseconds
        );
        assert!(worst <= 1e-3, "logit max diff {worst}");
        assert_eq!(
            u64::from(result.grade),
            case["single_fold_grade_pytorch"].as_u64().unwrap()
        );
        let fold = manifest_json["validation"]["fold"].as_u64().unwrap() as u8;
        assert_eq!(
            result.scope,
            crate::stage3_grading::Stage3Scope::SingleFoldTechnicalTest { fold }
        );
        assert_eq!(
            result.checkpoint_sha256,
            vec![crate::stage3_grading::FOLD_CHECKPOINT_SHA256[usize::from(fold)].to_owned()],
            "the manifest's fold must carry that fold's pinned release hash"
        );
        assert!(result.research_only);
    }

    fn verify_local_rect_lesion_runtime(model_kind: Stage2ModelKind, model_code: &str) {
        let prefix = model_code.to_ascii_lowercase();
        let parity_dir = std::path::PathBuf::from(
            std::env::var(format!("RETINACARE_STAGE2_{model_code}_PARITY_DIR"))
                .expect("set the temporary Python rectangle-lesion parity-fixture directory"),
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
        let prepared = preprocess_stage2_lesion_image(&image_path, model_kind).unwrap();
        let reference_input =
            std::fs::read(parity_dir.join(format!("{prefix}_input.f32"))).unwrap();
        assert_eq!(reference_input.len(), prepared.tensor.len() * 4);
        let max_input_diff = prepared
            .tensor
            .iter()
            .zip(reference_input.chunks(4))
            .map(|(rust, bytes)| (rust - f32::from_le_bytes(bytes.try_into().unwrap())).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_input_diff <= 1e-5,
            "{model_code} tensor max diff {max_input_diff}"
        );
        let result = runtime
            .infer_stage2_image(&Stage2Request {
                examination_id: 11,
                image_path,
                laterality: None,
                model_kind,
                model_id: loaded.model_id,
            })
            .unwrap();
        result.validate().unwrap();
        assert!(matches!(
            result.transform,
            crate::stage2_contract::Stage2SpatialTransform::LesionRectLetterbox { .. }
        ));
        let reference_mask =
            std::fs::read(parity_dir.join(format!("{prefix}_pytorch_mask.u8"))).unwrap();
        let crate::stage2_contract::MaskPixelData::Binary(rust_mask) = result.data else {
            panic!("expected binary {model_code} mask");
        };
        let disagreements = rust_mask
            .iter()
            .zip(reference_mask.iter())
            .filter(|(rust, python)| rust != python)
            .count();
        eprintln!(
            "Stage II {model_code}: tensor max_abs={max_input_diff:.9}, mask disagreement px={disagreements}, foreground_pixels={}, frame={}x{}, total_us={}",
            rust_mask.iter().filter(|&&pixel| pixel == 255).count(),
            result.original_width,
            result.original_height,
            result.elapsed_microseconds,
        );
        assert_eq!(disagreements, 0, "Rust/PyTorch {model_code} mask mismatch");
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

    fn proc_status_kib(key: &str) -> u64 {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|text| {
                text.lines()
                    .find(|line| line.starts_with(key))
                    .and_then(|line| line.split_whitespace().nth(1)?.parse().ok())
            })
            .unwrap_or(0)
    }

    /// Cold/warm Stage I profile on the real ONNX. Prints a table; asserts only determinism and the
    /// reported provider, never a speed target.
    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE1_MANIFEST and RETINACARE_STAGE1_IMAGE"]
    fn local_stage1_cold_and_warm_profile_is_deterministic() {
        use std::time::Instant;
        let manifest = std::path::PathBuf::from(
            std::env::var_os("RETINACARE_STAGE1_MANIFEST").expect("set RETINACARE_STAGE1_MANIFEST"),
        );
        let image = std::path::PathBuf::from(
            std::env::var_os("RETINACARE_STAGE1_IMAGE").expect("set RETINACARE_STAGE1_IMAGE"),
        );
        let mut logits = Vec::new();
        let cycles: usize = std::env::var("RETINACARE_PROFILE_CYCLES")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3);
        for cycle in 1..=cycles {
            let mut runtime = ModelRuntime::default();
            let rss_before = proc_status_kib("VmRSS:");
            let started = Instant::now();
            let loaded = runtime.load(&manifest, ProviderRequest::Cpu).unwrap();
            let load_ms = started.elapsed().as_millis();
            assert_eq!(loaded.provider, crate::device::ActualProvider::Cpu);
            let mut times = Vec::new();
            for _ in 0..3 {
                let out = runtime
                    .infer_stage1_image(&loaded.model_id, &image)
                    .unwrap();
                logits.push(out.gate.raw_logit);
                println!(
                    "GATE logit={} score={} decision={:?}",
                    out.gate.raw_logit, out.gate.screening_score, out.gate.decision
                );
                times.push((
                    out.elapsed_microseconds / 1000,
                    out.preprocessing_microseconds,
                ));
            }
            println!(
                "PROFILE cycle={cycle} load_ms={load_ms} infer_ms(total,pre_us)={times:?} rss_before_kib={rss_before} rss_after_kib={} hwm_kib={}",
                proc_status_kib("VmRSS:"),
                proc_status_kib("VmHWM:")
            );
            assert!(runtime.unload(&loaded.model_id));
        }
        assert!(
            logits
                .iter()
                .all(|value| value.to_bits() == logits[0].to_bits()),
            "Stage I logit must be bit-identical across cold and warm runs: {logits:?}"
        );
    }
    #[cfg(feature = "cuda")]
    fn gpu_memory_used_mib() -> Option<u64> {
        let out = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=memory.used", "--format=csv,noheader,nounits"])
            .output()
            .ok()?;
        String::from_utf8(out.stdout)
            .ok()?
            .lines()
            .next()?
            .trim()
            .parse()
            .ok()
    }

    /// Real CUDA proof for Stage I: the session is built with CPU fallback disabled, so success means the
    /// CUDA provider executed. Compares to the CPU logit and prints timings and GPU memory.
    #[cfg(feature = "cuda")]
    #[test]
    #[ignore = "local-only: needs an NVIDIA GPU, CUDA 12 + cuDNN 9 on LD_LIBRARY_PATH, RETINACARE_STAGE1_MANIFEST/IMAGE"]
    fn local_stage1_cuda_matches_cpu_and_reports_provider() {
        use crate::device::ActualProvider;
        use std::time::Instant;
        let manifest = std::path::PathBuf::from(
            std::env::var_os("RETINACARE_STAGE1_MANIFEST").expect("set RETINACARE_STAGE1_MANIFEST"),
        );
        let image = std::path::PathBuf::from(
            std::env::var_os("RETINACARE_STAGE1_IMAGE").expect("set RETINACARE_STAGE1_IMAGE"),
        );
        let mut cpu = ModelRuntime::default();
        let cpu_model = cpu.load(&manifest, ProviderRequest::Cpu).unwrap();
        let cpu_out = cpu.infer_stage1_image(&cpu_model.model_id, &image).unwrap();
        assert!(cpu.unload(&cpu_model.model_id));
        drop(cpu);

        let mib_before = gpu_memory_used_mib();
        let started = Instant::now();
        let mut gpu = ModelRuntime::default();
        let loaded = gpu
            .load(&manifest, ProviderRequest::Cuda { device_id: 0 })
            .expect("CUDA session must build with CPU fallback disabled");
        let load_ms = started.elapsed().as_millis();
        assert_eq!(loaded.provider, ActualProvider::Cuda { device_id: 0 });
        let mut runs = Vec::new();
        for _ in 0..4 {
            let out = gpu.infer_stage1_image(&loaded.model_id, &image).unwrap();
            runs.push(out);
        }
        let mib_after = gpu_memory_used_mib();
        println!(
            "CUDA-PROFILE provider={:?} load_ms={load_ms} infer_ms={:?} gpu_mem_used_mib before={mib_before:?} after={mib_after:?} cpu_logit={} gpu_logits={:?}",
            loaded.provider,
            runs.iter()
                .map(|r| r.elapsed_microseconds / 1000)
                .collect::<Vec<_>>(),
            cpu_out.gate.raw_logit,
            runs.iter().map(|r| r.gate.raw_logit).collect::<Vec<_>>()
        );
        for run in &runs {
            assert!(
                (run.gate.raw_logit - cpu_out.gate.raw_logit).abs() < 1e-2,
                "CUDA/CPU logit gap {}",
                (run.gate.raw_logit - cpu_out.gate.raw_logit).abs()
            );
            assert_eq!(run.gate.decision, cpu_out.gate.decision);
            assert_eq!(run.provider, ActualProvider::Cuda { device_id: 0 });
        }
    }
}
