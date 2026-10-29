//! Phase 4: occlusion sensitivity through the real pinned LG-DRG ONNX sessions.
//!
//! The perturbed input is the validated 8x512x512 tensor with the RGB patch replaced; the five mask
//! channels are held fixed. For the five-fold ensemble each fold is run over the same variants one
//! fold at a time (one ~0.8 GB session alive). Every Stage III target is linear in the 5-way vector,
//! so the mean of the per-fold deltas equals the delta of the mean-probability target exactly.
use crate::{
    attribution::{
        ALGORITHM_VERSION, AttributionArtifact, AttributionError, AttributionMeta,
        AttributionMethod, AttributionProvenance, AttributionScope, AttributionTarget,
        GeometryTransform, MODEL_INPUT, OcclusionParams, OcclusionRaw, run_occlusion,
        stage3_target_score, validate_combination,
    },
    model_runtime::{ModelRuntime, verify_stage3_fold_model},
    stage2_contract::Stage2MaskResult,
    stage2_preprocessing::read_stage2_source,
    stage3_grading::{FOLD_CHECKPOINT_SHA256, Stage3InferenceError, grade_single_fold},
    stage3_input::{Stage3Input, assemble_stage3_input},
};
use std::{path::Path, time::Instant};

/// One fold's occlusion run plus the identity of the session that produced it.
#[derive(Debug, Clone)]
pub struct FoldOcclusion {
    pub fold: u8,
    pub model_id: String,
    pub model_sha256: String,
    pub checkpoint_sha256: String,
    pub provider: String,
    pub raw: OcclusionRaw,
}

/// Who the result is for. Copied into the provenance so a foreign result is rejected later.
#[derive(Debug, Clone)]
pub struct ExplainContext {
    pub patient_id: i64,
    pub examination_id: i64,
    pub run_id: String,
    pub eye: String,
    pub generated_unix: i64,
}

/// Build the 8-channel tensor for explanation, with the same checks as grading (masks are required).
pub fn prepare_stage3_input(
    image_path: &Path,
    examination_id: i64,
    masks: &[Stage2MaskResult],
) -> Result<Stage3Input, Stage3InferenceError> {
    let encoded = read_stage2_source(image_path)?;
    Ok(assemble_stage3_input(&encoded, examination_id, masks)?)
}

impl ModelRuntime {
    /// Occlusion of one loaded fold. The model must be a pinned LG-DRG fold; the tensor is restored
    /// after every patch. `progress(done, total)` counts this fold's forward calls.
    pub fn explain_stage3_fold_occlusion(
        &mut self,
        model_id: &str,
        input: &mut Stage3Input,
        target: AttributionTarget,
        params: &OcclusionParams,
        cancelled: impl FnMut() -> bool,
        progress: impl FnMut(u32, u32),
    ) -> Result<FoldOcclusion, AttributionError> {
        let session = self
            .session_mut(model_id)
            .map_err(|error| AttributionError::Scoring(error.to_string()))?;
        let fold = verify_stage3_fold_model(session.model())
            .map_err(|error| AttributionError::Scoring(error.to_string()))?;
        validate_combination(
            AttributionMethod::OcclusionSensitivity,
            target,
            AttributionScope::Stage3SingleFold { fold: fold as u8 },
        )
        .or_else(|error| match target {
            // Grade probability is defined for the mean vector; a single fold scores it the same way.
            AttributionTarget::Stage3GradeProbability { .. } => Ok(()),
            _ => Err(error),
        })?;
        let model_sha256 = session.model().sha256.clone();
        let provider = format!("{:?}", session.actual_provider());
        let raw = run_occlusion(
            &mut input.tensor,
            8,
            MODEL_INPUT,
            params,
            |tensor| -> Result<f64, Stage3InferenceError> {
                let logits = session.run(tensor)?;
                let graded = grade_single_fold(&logits)?;
                stage3_target_score(target, &[graded.full])
                    .map_err(|_| Stage3InferenceError::InvalidRequest("target score"))
            },
            cancelled,
            progress,
        )?;
        Ok(FoldOcclusion {
            fold: fold as u8,
            model_id: model_id.to_owned(),
            model_sha256,
            checkpoint_sha256: FOLD_CHECKPOINT_SHA256[fold].to_owned(),
            provider,
            raw,
        })
    }
}

/// Mean of per-fold occlusion runs (exactly folds 0-4 once each) as the ensemble result.
fn mean_runs(runs: &[FoldOcclusion]) -> Result<OcclusionRaw, AttributionError> {
    let first = &runs[0].raw;
    let n = runs.len() as f64;
    let mut cells = vec![0.0f32; first.cells.len()];
    let mut perturbed = vec![0.0f64; first.perturbed_scores.len()];
    let mut baseline = 0.0f64;
    for run in runs {
        if run.raw.cells.len() != cells.len() || run.raw.grid != first.grid {
            return Err(AttributionError::Dimensions);
        }
        baseline += run.raw.baseline_score / n;
        for (slot, value) in cells.iter_mut().zip(&run.raw.cells) {
            *slot += value / n as f32;
        }
        for (slot, value) in perturbed.iter_mut().zip(&run.raw.perturbed_scores) {
            *slot += value / n;
        }
    }
    Ok(OcclusionRaw {
        baseline_score: baseline,
        cells,
        perturbed_scores: perturbed,
        grid: first.grid,
        forward_calls: runs.iter().map(|run| run.raw.forward_calls).sum(),
    })
}

/// Assemble the stored artifact from completed fold runs. One run = single-fold scope; five runs
/// = five-fold ensemble. Any other set (missing, duplicate, mixed providers) is rejected.
pub fn build_stage3_occlusion_artifact(
    runs: &[FoldOcclusion],
    input: &Stage3Input,
    target: AttributionTarget,
    params: &OcclusionParams,
    context: &ExplainContext,
    elapsed_ms: u64,
) -> Result<AttributionArtifact, AttributionError> {
    let scope = match runs {
        [one] => AttributionScope::Stage3SingleFold { fold: one.fold },
        many if many.len() == 5 => {
            let mut seen = [false; 5];
            for run in many {
                match seen.get_mut(usize::from(run.fold)) {
                    Some(slot) if !*slot => *slot = true,
                    _ => return Err(AttributionError::Fold),
                }
            }
            AttributionScope::Stage3FiveFoldEnsemble
        }
        _ => {
            return Err(AttributionError::Unsupported(
                "needs exactly one fold or all five",
            ));
        }
    };
    validate_combination(AttributionMethod::OcclusionSensitivity, target, scope)?;
    let mut sorted: Vec<&FoldOcclusion> = runs.iter().collect();
    sorted.sort_by_key(|run| run.fold);
    let sorted: Vec<FoldOcclusion> = sorted.into_iter().cloned().collect();
    if sorted.iter().any(|run| run.provider != sorted[0].provider) {
        return Err(AttributionError::Unsupported(
            "folds ran on different providers",
        ));
    }
    let raw = mean_runs(&sorted)?;
    let mut parameters = params.to_map();
    parameters.insert(
        "target_score".into(),
        "mean 5-way probability of the folds in scope".into(),
    );
    parameters.insert(
        "perturbed_score_min".into(),
        raw.perturbed_scores
            .iter()
            .copied()
            .fold(f64::MAX, f64::min)
            .to_string(),
    );
    parameters.insert(
        "perturbed_score_max".into(),
        raw.perturbed_scores
            .iter()
            .copied()
            .fold(f64::MIN, f64::max)
            .to_string(),
    );
    parameters.insert("masks".into(), "held fixed (RGB influence only)".into());
    let image_sha256 = input.image_sha256.clone();
    let id = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(image_sha256.as_bytes());
        hasher.update(context.run_id.as_bytes());
        hasher.update(format!("{target:?}{scope:?}{}", context.generated_unix).as_bytes());
        hex::encode(&hasher.finalize()[..16])
    };
    let artifact = AttributionArtifact {
        meta: AttributionMeta {
            id,
            method: AttributionMethod::OcclusionSensitivity,
            target,
            scope,
            provenance: AttributionProvenance {
                patient_id: context.patient_id,
                examination_id: context.examination_id,
                run_id: context.run_id.clone(),
                eye: context.eye.clone(),
                image_sha256,
                model_ids: sorted.iter().map(|run| run.model_id.clone()).collect(),
                model_sha256: sorted.iter().map(|run| run.model_sha256.clone()).collect(),
                checkpoint_sha256: sorted
                    .iter()
                    .map(|run| run.checkpoint_sha256.clone())
                    .collect(),
                folds: sorted.iter().map(|run| run.fold).collect(),
                provider: sorted[0].provider.clone(),
                algorithm_version: ALGORITHM_VERSION.into(),
                parameters,
                generated_unix: context.generated_unix,
                research_only: true,
            },
            baseline_score: raw.baseline_score,
            native_width: raw.grid as u32,
            native_height: raw.grid as u32,
            transform: GeometryTransform {
                original_width: input.original_width,
                original_height: input.original_height,
                crop_x: input.fundus_crop.x,
                crop_y: input.fundus_crop.y,
                crop_width: input.fundus_crop.width,
                crop_height: input.fundus_crop.height,
                resized_width: input.resized_width,
                resized_height: input.resized_height,
                offset_x: input.offset_x,
                offset_y: input.offset_y,
                model_size: MODEL_INPUT as u32,
            },
            feature_layer: String::new(),
            forward_calls: raw.forward_calls,
            elapsed_ms,
        },
        raw_signed: raw.cells,
    };
    artifact.validate()?;
    Ok(artifact)
}

/// Local PyTorch worker for Grad-CAM (trusted pinned upstream clone; no network, bounded output).
#[derive(Debug, Clone)]
pub struct GradCamTools {
    pub python: std::path::PathBuf,
    pub script: std::path::PathBuf,
    /// Verified `stage3_lgdrg_best_fold{n}.pt` of the fold being explained.
    pub checkpoint: std::path::PathBuf,
    pub timeout: std::time::Duration,
}

#[derive(serde::Deserialize)]
struct GradCamWorkerOutput {
    fold: u8,
    checkpoint_sha256: String,
    score: f64,
    layer: String,
    native_width: u32,
    native_height: u32,
    relu_map: Vec<f32>,
}

fn target_arg(target: AttributionTarget) -> Result<String, AttributionError> {
    match target {
        AttributionTarget::Stage3Severity { grade } if (1..=4).contains(&grade) => {
            Ok(format!("severity:{grade}"))
        }
        // Per-fold probability of the grade in the five-way vector (the ensemble target is their mean).
        AttributionTarget::Stage3GradeProbability { grade } if (1..=5).contains(&grade) => {
            Ok(format!("grade:{grade}"))
        }
        AttributionTarget::Stage3Gradability => Ok("gradability".into()),
        _ => Err(AttributionError::Unsupported(
            "Grad-CAM target not supported",
        )),
    }
}

fn transform_of(input: &Stage3Input) -> GeometryTransform {
    GeometryTransform {
        original_width: input.original_width,
        original_height: input.original_height,
        crop_x: input.fundus_crop.x,
        crop_y: input.fundus_crop.y,
        crop_width: input.fundus_crop.width,
        crop_height: input.fundus_crop.height,
        resized_width: input.resized_width,
        resized_height: input.resized_height,
        offset_x: input.offset_x,
        offset_y: input.offset_y,
        model_size: MODEL_INPUT as u32,
    }
}

fn run_fold_worker(
    tools: &GradCamTools,
    fold: u8,
    input: &Stage3Input,
    target_text: &str,
    cancelled: impl FnMut() -> bool,
) -> Result<GradCamWorkerOutput, AttributionError> {
    let args = [
        "--fold".into(),
        fold.to_string().into(),
        "--target".into(),
        target_text.into(),
        "--checkpoint".into(),
        tools.checkpoint.clone().into_os_string(),
    ];
    let bytes = crate::gradcam_worker::run_worker(
        &tools.python,
        &tools.script,
        &args,
        &input.tensor,
        tools.timeout,
        cancelled,
    )?;
    let out: GradCamWorkerOutput = serde_json::from_slice(&bytes)
        .map_err(|_| AttributionError::Scoring("Grad-CAM worker output invalid".into()))?;
    if out.fold != fold || out.checkpoint_sha256 != FOLD_CHECKPOINT_SHA256[usize::from(fold)] {
        return Err(AttributionError::WrongModels);
    }
    if out.native_width as usize * out.native_height as usize != out.relu_map.len() {
        return Err(AttributionError::Dimensions);
    }
    Ok(out)
}

fn artifact_id(input: &Stage3Input, context: &ExplainContext, tag: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input.image_sha256.as_bytes());
    hasher.update(context.run_id.as_bytes());
    hasher.update(format!("{tag}{}", context.generated_unix).as_bytes());
    hex::encode(&hasher.finalize()[..16])
}

/// Grad-CAM of ONE fold through the PyTorch EMA weights. The tensor handed to the worker is the exact
/// one the ONNX path scores. The worker is killed on cancel or timeout; no partial result is kept.
pub fn explain_stage3_gradcam(
    tools: &GradCamTools,
    fold: u8,
    input: &Stage3Input,
    target: AttributionTarget,
    context: &ExplainContext,
    cancelled: impl FnMut() -> bool,
) -> Result<AttributionArtifact, AttributionError> {
    let scope = AttributionScope::Stage3SingleFold { fold };
    // A fold scores grade probabilities with the same vector algebra; only the severity targets are
    // single-fold labels, so grade probabilities are mapped to them for validation.
    let label_target = match target {
        AttributionTarget::Stage3GradeProbability { grade } if grade <= 4 => {
            AttributionTarget::Stage3Severity { grade }
        }
        AttributionTarget::Stage3GradeProbability { .. } => AttributionTarget::Stage3Gradability,
        other => other,
    };
    validate_combination(AttributionMethod::GradCam, label_target, scope)?;
    let target_text = target_arg(label_target)?;
    let started = Instant::now();
    let out = run_fold_worker(tools, fold, input, &target_text, cancelled)?;
    let id = artifact_id(input, context, &format!("gradcam{label_target:?}{scope:?}"));
    let parameters = std::collections::BTreeMap::from([
        ("feature_layer".to_string(), out.layer.clone()),
        (
            "explained_score".into(),
            "differentiable 5-way probability (CORN algebra incl. gradability gate)".into(),
        ),
        (
            "map".into(),
            "ReLU(sum_k alpha_k A_k), alpha_k = spatial mean gradient".into(),
        ),
        (
            "weights".into(),
            "pytorch ema checkpoint, weights_only=True".into(),
        ),
        ("worker_score".into(), out.score.to_string()),
    ]);
    let artifact = AttributionArtifact {
        meta: AttributionMeta {
            id,
            method: AttributionMethod::GradCam,
            target: label_target,
            scope,
            provenance: AttributionProvenance {
                patient_id: context.patient_id,
                examination_id: context.examination_id,
                run_id: context.run_id.clone(),
                eye: context.eye.clone(),
                image_sha256: input.image_sha256.clone(),
                model_ids: vec![format!(
                    "hierarchiretina-stage3-lgdrg-fold{fold}-pytorch-ema"
                )],
                model_sha256: vec![out.checkpoint_sha256.clone()],
                checkpoint_sha256: vec![out.checkpoint_sha256],
                folds: vec![fold],
                provider: "pytorch-cpu".into(),
                algorithm_version: ALGORITHM_VERSION.into(),
                parameters,
                generated_unix: context.generated_unix,
                research_only: true,
            },
            baseline_score: out.score,
            native_width: out.native_width,
            native_height: out.native_height,
            transform: transform_of(input),
            feature_layer: out.layer,
            forward_calls: 1,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
        raw_signed: out.relu_map,
    };
    artifact.validate()?;
    Ok(artifact)
}

/// Five-fold Grad-CAM: the equal-weight mean of the five per-fold Grad-CAM maps, each computed on its
/// own network for the SAME five-way target (see `validate_combination`). The baseline score is the mean
/// of the five fold scores, which is the ensemble target score. Folds run one after another; progress is
/// `done` of 5; a failed or cancelled fold gives no result at all.
pub fn explain_stage3_gradcam_ensemble(
    tools: &[GradCamTools; 5],
    input: &Stage3Input,
    target: AttributionTarget,
    context: &ExplainContext,
    mut cancelled: impl FnMut() -> bool,
    mut progress: impl FnMut(u32, u32),
) -> Result<AttributionArtifact, AttributionError> {
    let scope = AttributionScope::Stage3FiveFoldEnsemble;
    validate_combination(AttributionMethod::GradCam, target, scope)?;
    let target_text = target_arg(target)?;
    let started = Instant::now();
    let mut outs = Vec::with_capacity(5);
    for (fold, tool) in tools.iter().enumerate() {
        progress(fold as u32, 5);
        outs.push(run_fold_worker(
            tool,
            fold as u8,
            input,
            &target_text,
            &mut cancelled,
        )?);
    }
    progress(5, 5);
    let len = outs[0].relu_map.len();
    if outs
        .iter()
        .any(|o| o.relu_map.len() != len || o.native_width != outs[0].native_width)
    {
        return Err(AttributionError::Dimensions);
    }
    let mean: Vec<f32> = (0..len)
        .map(|i| outs.iter().map(|o| o.relu_map[i]).sum::<f32>() / 5.0)
        .collect();
    let baseline = outs.iter().map(|o| o.score).sum::<f64>() / 5.0;
    let parameters = std::collections::BTreeMap::from([
        ("feature_layer".to_string(), outs[0].layer.clone()),
        (
            "aggregation".into(),
            "equal-weight mean of five per-fold Grad-CAM maps (each fold: ReLU(sum_k alpha_k A_k) on its own last spatial map)".into(),
        ),
        (
            "explained_score".into(),
            "mean over the five folds of the fold's 5-way probability for the target".into(),
        ),
        (
            "fold_scores".into(),
            outs.iter().map(|o| format!("{:.6}", o.score)).collect::<Vec<_>>().join(","),
        ),
        ("weights".into(), "pytorch ema checkpoints, weights_only=True".into()),
    ]);
    let artifact = AttributionArtifact {
        meta: AttributionMeta {
            id: artifact_id(input, context, &format!("gradcam{target:?}{scope:?}")),
            method: AttributionMethod::GradCam,
            target,
            scope,
            provenance: AttributionProvenance {
                patient_id: context.patient_id,
                examination_id: context.examination_id,
                run_id: context.run_id.clone(),
                eye: context.eye.clone(),
                image_sha256: input.image_sha256.clone(),
                model_ids: (0..5)
                    .map(|k| format!("hierarchiretina-stage3-lgdrg-fold{k}-pytorch-ema"))
                    .collect(),
                model_sha256: outs.iter().map(|o| o.checkpoint_sha256.clone()).collect(),
                checkpoint_sha256: outs.iter().map(|o| o.checkpoint_sha256.clone()).collect(),
                folds: vec![0, 1, 2, 3, 4],
                provider: "pytorch-cpu".into(),
                algorithm_version: ALGORITHM_VERSION.into(),
                parameters,
                generated_unix: context.generated_unix,
                research_only: true,
            },
            baseline_score: baseline,
            native_width: outs[0].native_width,
            native_height: outs[0].native_height,
            transform: transform_of(input),
            feature_layer: outs[0].layer.clone(),
            forward_calls: 5,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
        raw_signed: mean,
    };
    artifact.validate()?;
    Ok(artifact)
}

/// Convenience driver used by the app: runs the listed fold manifests one after another (fold-major),
/// loading and unloading each session, and returns the finished artifact. Cancellation is polled
/// between forward calls and between folds; a partial result is never returned.
#[allow(clippy::too_many_arguments)] // one call site; a parameter struct would only rename the same eight values
pub fn explain_stage3_occlusion(
    runtime: &mut ModelRuntime,
    fold_manifests: &[&Path],
    input: &mut Stage3Input,
    target: AttributionTarget,
    params: &OcclusionParams,
    context: &ExplainContext,
    mut cancelled: impl FnMut() -> bool,
    mut progress: impl FnMut(u32, u32),
) -> Result<AttributionArtifact, AttributionError> {
    use crate::{device::ProviderRequest, model_runtime::ModelRuntimeError};
    let started = Instant::now();
    let per_fold = (params.origins(MODEL_INPUT).len().pow(2) + 1) as u32;
    let total = per_fold * fold_manifests.len() as u32;
    let mut runs = Vec::with_capacity(fold_manifests.len());
    for (index, manifest) in fold_manifests.iter().enumerate() {
        if cancelled() {
            return Err(AttributionError::Cancelled);
        }
        let (model_id, owned) =
            match runtime.load_unverified(manifest, ProviderRequest::Cpu, "explainability") {
                Ok(loaded) => (loaded.model_id, true),
                Err(ModelRuntimeError::AlreadyLoaded(id)) => (id, false),
                Err(error) => return Err(AttributionError::Scoring(error.to_string())),
            };
        let base = per_fold * index as u32;
        let outcome = runtime.explain_stage3_fold_occlusion(
            &model_id,
            input,
            target,
            params,
            &mut cancelled,
            |done, _| progress(base + done, total),
        );
        if owned {
            runtime.unload(&model_id);
        }
        runs.push(outcome?);
    }
    build_stage3_occlusion_artifact(
        &runs,
        input,
        target,
        params,
        context,
        started.elapsed().as_millis() as u64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stage3_input::{STAGE3_MASK_ORDER, assemble_stage3_input_raw};
    use std::path::PathBuf;

    fn env(name: &str) -> PathBuf {
        PathBuf::from(std::env::var(name).unwrap_or_else(|_| panic!("set {name}")))
    }

    /// Real ONNX occlusion on the technical demo image (unknown provenance, technical use only).
    /// Env: RETINACARE_CASCADE_IMAGE, RETINACARE_EXPLAIN_MASKS (dir with ma/he/ex/cws/vessel
    /// `<name>_pytorch_mask.u8`), RETINACARE_CASCADE_FOLD0_MANIFEST, optional
    /// RETINACARE_CASCADE_OTHER_FOLD_MANIFESTS (path list; all five folds when set),
    /// optional RETINACARE_EXPLAIN_OUT (directory for the raw map + metadata).
    #[test]
    #[ignore = "heavy local: real LG-DRG ONNX sessions; see doc comment for environment variables"]
    fn local_real_model_occlusion_and_deletion() {
        let image = env("RETINACARE_CASCADE_IMAGE");
        let masks_dir = env("RETINACARE_EXPLAIN_MASKS");
        let encoded = std::fs::read(&image).unwrap();
        let names = ["ma", "he", "ex", "cws", "vessel"];
        let raw: Vec<Vec<u8>> = names
            .iter()
            .map(|n| std::fs::read(masks_dir.join(format!("{n}_pytorch_mask.u8"))).unwrap())
            .collect();
        assert_eq!(STAGE3_MASK_ORDER.len(), 5);
        let mut input =
            assemble_stage3_input_raw(&encoded, [&raw[0], &raw[1], &raw[2], &raw[3], &raw[4]])
                .unwrap();
        let mut manifests = vec![env("RETINACARE_CASCADE_FOLD0_MANIFEST")];
        if let Some(list) = std::env::var_os("RETINACARE_CASCADE_OTHER_FOLD_MANIFESTS") {
            manifests.extend(std::env::split_paths(&list));
        }
        let refs: Vec<&Path> = manifests.iter().map(PathBuf::as_path).collect();
        // Target: the grade the technical demo reaches (severity 3 in the recorded runs).
        let target = if refs.len() == 5 {
            AttributionTarget::Stage3GradeProbability { grade: 3 }
        } else {
            AttributionTarget::Stage3Severity { grade: 3 }
        };
        let params = OcclusionParams::default();
        let context = ExplainContext {
            patient_id: 1,
            examination_id: 101,
            run_id: "heavy-test".into(),
            eye: "right".into(),
            generated_unix: 0,
        };
        let mut runtime = ModelRuntime::default();
        let started = Instant::now();
        let mut last = 0;
        let artifact = explain_stage3_occlusion(
            &mut runtime,
            &refs,
            &mut input,
            target,
            &params,
            &context,
            || false,
            |done, total| {
                if done != last && done % 20 == 0 {
                    eprintln!(
                        "progress {done}/{total} at {:.0}s",
                        started.elapsed().as_secs_f64()
                    );
                }
                last = done;
            },
        )
        .unwrap();
        eprintln!(
            "OCCLUSION: scope={:?} target={:?} baseline={:.6} forwards={} elapsed_ms={} cells_min={:.5} cells_max={:.5}",
            artifact.meta.scope,
            artifact.meta.target,
            artifact.meta.baseline_score,
            artifact.meta.forward_calls,
            artifact.meta.elapsed_ms,
            artifact.support().min,
            artifact.support().max
        );
        // Parity: baseline target score must equal the PyTorch reference of the same fold set.
        let mut reference = Vec::new();
        for path in &refs {
            let json: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            let case = json["validation"]["pytorch_vs_onnxruntime_cpu"]["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["input"].as_str().unwrap().starts_with("demo image"))
                .expect("demo-image PyTorch reference");
            let v: Vec<f32> = case["pytorch_logits"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap() as f32)
                .collect();
            reference.push(grade_single_fold(&v).unwrap().full);
        }
        let expected = stage3_target_score(target, &reference).unwrap();
        eprintln!(
            "PARITY: onnx_baseline={:.6} pytorch_reference={expected:.6}",
            artifact.meta.baseline_score
        );
        assert!((artifact.meta.baseline_score - expected).abs() < 1e-3);
        if let Some(out) = std::env::var_os("RETINACARE_EXPLAIN_OUT") {
            let out = PathBuf::from(out);
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(out.join("map.f32"), artifact.map_bytes()).unwrap();
            let meta = serde_json::to_vec_pretty(&artifact.meta).unwrap();
            std::fs::write(out.join("meta.json"), meta).unwrap();
            std::fs::write(out.join("map.sha256"), artifact.map_sha256()).unwrap();
        }
    }

    /// Deletion / insertion faithfulness on fold 0 (technical demo image, not clinical evidence).
    /// Ranks the 64 occlusion cells by their measured score drop, then removes (deletion) or restores
    /// (insertion) the top-k fraction at once and records the REAL fold-0 target score, next to three
    /// seeded random cell orders. Env as `local_real_model_occlusion_and_deletion` (fold 0 only).
    #[test]
    #[ignore = "heavy local: real LG-DRG ONNX session; see doc comment for environment variables"]
    fn local_deletion_insertion_faithfulness() {
        use crate::attribution::{cells_to_delete, run_occlusion};
        use crate::device::ProviderRequest;
        let encoded = std::fs::read(env("RETINACARE_CASCADE_IMAGE")).unwrap();
        let masks_dir = env("RETINACARE_EXPLAIN_MASKS");
        let raw: Vec<Vec<u8>> = ["ma", "he", "ex", "cws", "vessel"]
            .iter()
            .map(|n| std::fs::read(masks_dir.join(format!("{n}_pytorch_mask.u8"))).unwrap())
            .collect();
        let mut input =
            assemble_stage3_input_raw(&encoded, [&raw[0], &raw[1], &raw[2], &raw[3], &raw[4]])
                .unwrap();
        let mut runtime = ModelRuntime::default();
        let loaded = runtime
            .load(
                &env("RETINACARE_CASCADE_FOLD0_MANIFEST"),
                ProviderRequest::Cpu,
            )
            .unwrap();
        let target = AttributionTarget::Stage3Severity { grade: 3 };
        let params = OcclusionParams::default();
        let session = runtime.session_mut(&loaded.model_id).unwrap();
        let mut score = |tensor: &[f32]| -> Result<f64, Stage3InferenceError> {
            let logits = session.run(tensor)?;
            let graded = grade_single_fold(&logits)?;
            Ok(stage3_target_score(target, &[graded.full]).unwrap())
        };
        let base_tensor = input.tensor.clone();
        let occ = run_occlusion(
            &mut input.tensor,
            8,
            512,
            &params,
            &mut score,
            || false,
            |_, _| {},
        )
        .unwrap();
        let grid = occ.grid;
        let blank = |tensor: &mut Vec<f32>, cell: usize, restore: bool| {
            let (gy, gx) = (cell / grid, cell % grid);
            for channel in 0..3 {
                for y in gy * 64..gy * 64 + 64 {
                    let row = channel * 512 * 512 + y * 512 + gx * 64;
                    if restore {
                        tensor[row..row + 64].copy_from_slice(&base_tensor[row..row + 64]);
                    } else {
                        tensor[row..row + 64].fill(0.0);
                    }
                }
            }
        };
        let mut seed = 0x2545_f491_4f6c_dd1d_u64;
        let mut random_order = |n: usize| {
            let mut order: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                order.swap(i, (seed % (i as u64 + 1)) as usize);
            }
            order
        };
        let ranked = cells_to_delete(&occ.cells, 1.0);
        let randoms = [random_order(64), random_order(64), random_order(64)];
        eprintln!(
            "BASELINE target score {:.6}; cells {}",
            occ.baseline_score,
            occ.cells.len()
        );
        for fraction in [0.0f32, 0.1, 0.2, 0.3, 0.4, 0.5] {
            let k = (64.0 * fraction).round() as usize;
            let measure = |order: &[usize], insertion: bool, score: &mut dyn FnMut(&[f32]) -> Result<f64, Stage3InferenceError>| {
                let mut tensor = base_tensor.clone();
                if insertion {
                    for cell in 0..64 {
                        blank(&mut tensor, cell, false);
                    }
                    for cell in &order[..k] {
                        blank(&mut tensor, *cell, true);
                    }
                } else {
                    for cell in &order[..k] {
                        blank(&mut tensor, *cell, false);
                    }
                }
                score(&tensor).unwrap()
            };
            let del = measure(&ranked, false, &mut score);
            let ins = measure(&ranked, true, &mut score);
            let del_r: Vec<f64> = randoms
                .iter()
                .map(|o| measure(o, false, &mut score))
                .collect();
            let ins_r: Vec<f64> = randoms
                .iter()
                .map(|o| measure(o, true, &mut score))
                .collect();
            let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
            eprintln!(
                "FAITHFULNESS fraction={fraction:.1} k={k} deletion_ranked={del:.4} deletion_random_mean={:.4} insertion_ranked={ins:.4} insertion_random_mean={:.4}",
                mean(&del_r),
                mean(&ins_r)
            );
        }
    }

    /// Grad-CAM worker parity: the explained fold-0 score equals the ONNX fold-0 score of the same tensor,
    /// and the map is finite with positive support. Env as above plus RETINACARE_REPO (repository root).
    #[test]
    #[ignore = "heavy local: PyTorch worker (about 5 GiB RAM); set RETINACARE_REPO and the variables above"]
    fn local_gradcam_score_matches_onnx() {
        use crate::device::ProviderRequest;
        let repo = env("RETINACARE_REPO");
        let encoded = std::fs::read(env("RETINACARE_CASCADE_IMAGE")).unwrap();
        let masks_dir = env("RETINACARE_EXPLAIN_MASKS");
        let raw: Vec<Vec<u8>> = ["ma", "he", "ex", "cws", "vessel"]
            .iter()
            .map(|n| std::fs::read(masks_dir.join(format!("{n}_pytorch_mask.u8"))).unwrap())
            .collect();
        let input =
            assemble_stage3_input_raw(&encoded, [&raw[0], &raw[1], &raw[2], &raw[3], &raw[4]])
                .unwrap();
        let target = AttributionTarget::Stage3Severity { grade: 3 };
        let tools = GradCamTools {
            python: repo.join("ai_engine/.venv/bin/python"),
            script: repo.join("scripts/stage3_gradcam.py"),
            checkpoint: repo.join("models/stage3_lgdrg/stage3_lgdrg_best_fold0.pt"),
            timeout: std::time::Duration::from_secs(600),
        };
        let context = ExplainContext {
            patient_id: 1,
            examination_id: 1,
            run_id: "heavy".into(),
            eye: "right".into(),
            generated_unix: 0,
        };
        let artifact =
            explain_stage3_gradcam(&tools, 0, &input, target, &context, || false).unwrap();
        let mut runtime = ModelRuntime::default();
        let loaded = runtime
            .load(
                &env("RETINACARE_CASCADE_FOLD0_MANIFEST"),
                ProviderRequest::Cpu,
            )
            .unwrap();
        let logits = runtime
            .session_mut(&loaded.model_id)
            .unwrap()
            .run(&input.tensor)
            .unwrap();
        let onnx =
            stage3_target_score(target, &[grade_single_fold(&logits).unwrap().full]).unwrap();
        eprintln!(
            "GRADCAM: worker_score={:.6} onnx_score={onnx:.6} map_max={:.5} elapsed_ms={}",
            artifact.meta.baseline_score,
            artifact.support().max,
            artifact.meta.elapsed_ms
        );
        assert!((artifact.meta.baseline_score - onnx).abs() < 1e-3);
        assert!(artifact.support().has_positive);
        assert_eq!(artifact.meta.native_width, 16);
    }
}
