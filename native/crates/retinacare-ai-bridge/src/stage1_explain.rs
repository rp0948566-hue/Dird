//! Phase 5: Grad-CAM of the Stage I screening gate through the pinned PyTorch checkpoint.
//!
//! The 3x768x768 tensor handed to the worker is the exact one the ONNX gate scores. The 24x24 map is
//! bilinearly upsampled to the 768-pixel model frame and restored to the source image through the square
//! window recorded by the preprocessing (`Stage1Geometry`); pixels the model saw only as zero padding, and
//! pixels outside the retina, are zero. Nothing is shown unless the worker's score matches the checkpoint.

use crate::{
    attribution::{
        ALGORITHM_VERSION, AttributionArtifact, AttributionError, AttributionMeta,
        AttributionMethod, AttributionProvenance, AttributionScope, AttributionTarget,
        GeometryTransform, STAGE1_CHECKPOINT_SHA256, validate_combination,
    },
    gradcam_worker::run_worker,
    preprocessing::{PreprocessedStage1Image, STAGE1_IMAGE_SIZE},
    stage3_explain::{ExplainContext, GradCamTools},
};
use std::time::Instant;

#[derive(serde::Deserialize)]
struct Stage1WorkerOutput {
    checkpoint_sha256: String,
    ema_applied: bool,
    score: f64,
    logit: f64,
    layer: String,
    native_width: u32,
    native_height: u32,
    relu_map: Vec<f32>,
}

/// Transform of the Stage I window; `None` when the window does not overlap the image.
pub fn stage1_transform(pre: &PreprocessedStage1Image) -> Option<GeometryTransform> {
    let v = pre
        .geometry
        .visible(pre.original_width, pre.original_height)?;
    Some(GeometryTransform {
        original_width: pre.original_width,
        original_height: pre.original_height,
        crop_x: v.crop_x,
        crop_y: v.crop_y,
        crop_width: v.crop_width,
        crop_height: v.crop_height,
        resized_width: v.resized_width,
        resized_height: v.resized_height,
        offset_x: v.offset_x,
        offset_y: v.offset_y,
        model_size: STAGE1_IMAGE_SIZE as u32,
    })
}

pub fn explain_stage1_gradcam(
    tools: &GradCamTools,
    pre: &PreprocessedStage1Image,
    context: &ExplainContext,
    cancelled: impl FnMut() -> bool,
) -> Result<AttributionArtifact, AttributionError> {
    let scope = AttributionScope::Stage1Model;
    let target = AttributionTarget::Stage1DrScreening;
    validate_combination(AttributionMethod::GradCam, target, scope)?;
    let transform = stage1_transform(pre).ok_or(AttributionError::Dimensions)?;
    let started = Instant::now();
    let args = [
        "--checkpoint".into(),
        tools.checkpoint.clone().into_os_string(),
    ];
    let bytes = run_worker(
        &tools.python,
        &tools.script,
        &args,
        &pre.tensor,
        tools.timeout,
        cancelled,
    )?;
    let out: Stage1WorkerOutput = serde_json::from_slice(&bytes)
        .map_err(|_| AttributionError::Scoring("Grad-CAM worker output invalid".into()))?;
    if out.checkpoint_sha256 != STAGE1_CHECKPOINT_SHA256 || !out.ema_applied {
        return Err(AttributionError::WrongModels);
    }
    if out.native_width as usize * out.native_height as usize != out.relu_map.len() {
        return Err(AttributionError::Dimensions);
    }
    let id = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(pre.image_sha256.as_bytes());
        h.update(context.run_id.as_bytes());
        h.update(format!("stage1-gradcam{}", context.generated_unix).as_bytes());
        hex::encode(&h.finalize()[..16])
    };
    let parameters = std::collections::BTreeMap::from([
        ("feature_layer".to_string(), out.layer.clone()),
        (
            "explained_score".into(),
            "sigmoid(logit) of the screening gate; routing threshold 0.2391715943813324".into(),
        ),
        (
            "map".into(),
            "ReLU(sum_k alpha_k A_k), alpha_k = spatial mean gradient".into(),
        ),
        (
            "weights".into(),
            "pytorch checkpoint with EMA shadow, as the ONNX export".into(),
        ),
        ("worker_logit".into(), out.logit.to_string()),
        (
            "window".into(),
            format!(
                "x={} y={} side={} (zero padded), resized to 768",
                pre.geometry.x, pre.geometry.y, pre.geometry.side
            ),
        ),
    ]);
    let artifact = AttributionArtifact {
        meta: AttributionMeta {
            id,
            method: AttributionMethod::GradCam,
            target,
            scope,
            provenance: AttributionProvenance {
                patient_id: context.patient_id,
                examination_id: context.examination_id,
                run_id: context.run_id.clone(),
                eye: context.eye.clone(),
                image_sha256: pre.image_sha256.clone(),
                model_ids: vec!["hierarchiretina-stage1-convnextv2l-768-pytorch-ema".into()],
                model_sha256: vec![out.checkpoint_sha256.clone()],
                checkpoint_sha256: vec![out.checkpoint_sha256],
                folds: vec![],
                provider: "pytorch-cpu".into(),
                algorithm_version: ALGORITHM_VERSION.into(),
                parameters,
                generated_unix: context.generated_unix,
                research_only: true,
            },
            baseline_score: out.score,
            native_width: out.native_width,
            native_height: out.native_height,
            transform,
            feature_layer: out.layer,
            forward_calls: 1,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
        raw_signed: out.relu_map,
    };
    artifact.validate()?;
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preprocessing::Stage1Geometry;

    fn pre(width: u32, height: u32, geometry: Stage1Geometry) -> PreprocessedStage1Image {
        PreprocessedStage1Image {
            image_sha256: "a".repeat(64),
            original_width: width,
            original_height: height,
            geometry,
            tensor: Vec::new(),
            preprocessing_version: "t",
        }
    }

    #[test]
    fn transform_follows_the_recorded_window_and_validates() {
        let t = stage1_transform(&pre(
            640,
            480,
            Stage1Geometry {
                x: 80,
                y: 0,
                side: 480,
            },
        ))
        .unwrap();
        assert_eq!(
            (t.crop_x, t.crop_y, t.crop_width, t.crop_height),
            (80, 0, 480, 480)
        );
        assert_eq!(
            (t.offset_x, t.offset_y, t.resized_width, t.resized_height),
            (0, 0, 768, 768)
        );
        assert_eq!(t.model_size, 768);
        // Window hanging over the top-left corner: the zero-padded part is not part of the crop.
        let t = stage1_transform(&pre(
            300,
            300,
            Stage1Geometry {
                x: -30,
                y: -30,
                side: 360,
            },
        ))
        .unwrap();
        assert_eq!((t.crop_x, t.crop_y), (0, 0));
        assert!(t.offset_x > 0 && t.offset_y > 0);
        assert!(t.offset_x + t.resized_width <= 768);
        assert!(
            stage1_transform(&pre(
                300,
                300,
                Stage1Geometry {
                    x: 900,
                    y: 0,
                    side: 100
                }
            ))
            .is_none()
        );
    }

    #[test]
    fn a_missing_worker_is_an_error_and_never_a_map() {
        let tools = GradCamTools {
            python: "/nonexistent/python".into(),
            script: "/nonexistent/stage1_gradcam.py".into(),
            checkpoint: "/nonexistent.pth".into(),
            timeout: std::time::Duration::from_secs(5),
        };
        let ctx = ExplainContext {
            patient_id: 1,
            examination_id: 1,
            run_id: "r".into(),
            eye: "right".into(),
            generated_unix: 0,
        };
        let p = pre(
            640,
            480,
            Stage1Geometry {
                x: 80,
                y: 0,
                side: 480,
            },
        );
        assert!(explain_stage1_gradcam(&tools, &p, &ctx, || false).is_err());
    }
}
