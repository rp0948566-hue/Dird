//! Phase 4 attribution contract and the forward-only occlusion engine (research use only).
//!
//! An artifact holds the RAW signed attribution at its native resolution plus everything needed to
//! rebuild the model-space map and to restore it to source pixels. Colours and alpha are a view and
//! are never stored here. Nothing in this module invents values: a map exists only if a named model
//! target produced it (Grad-CAM values come from the PyTorch worker, occlusion values from real
//! forward passes through the pinned ONNX sessions).
use crate::stage3_grading::{
    FOLD_CHECKPOINT_SHA256, FullVector, Stage3GradingError, ensemble_mean,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

pub const ALGORITHM_VERSION: &str = "retinacare-attribution-1.0";
/// Stage III model input edge (square, zero padded).
pub const MODEL_INPUT: usize = 512;
/// Upstream fundus box tolerance: a pixel is retina when the channel sum exceeds 3 * 7.
const FOV_SUM: u32 = 21;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionMethod {
    GradCam,
    OcclusionSensitivity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttributionTarget {
    /// Stage I sigmoid probability of the DR screening gate.
    Stage1DrScreening,
    /// Severity grade 1-4 entry of the (fold or mean) 5-way vector, already scaled by P(gradable).
    Stage3Severity { grade: u8 },
    /// P(gradable) = 1 - 5-way ungradable entry.
    Stage3Gradability,
    /// Entry `grade` (1-5; 5 = ungradable) of the mean 5-way probability vector. Five-fold only.
    Stage3GradeProbability { grade: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttributionScope {
    Stage1Model,
    Stage3SingleFold { fold: u8 },
    Stage3FiveFoldEnsemble,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionStatus {
    NotGenerated,
    Queued,
    Generating,
    Ready,
    Unavailable,
    Failed,
    Stale,
    Cancelled,
}

#[derive(Debug, Error, PartialEq)]
pub enum AttributionError {
    #[error("unsupported method/target/scope combination: {0}")]
    Unsupported(&'static str),
    #[error("invalid attribution dimensions")]
    Dimensions,
    #[error("attribution contains a non-finite value")]
    NonFinite,
    #[error("attribution is constant or all zero")]
    Constant,
    #[error("image digest must be 64 lowercase hex characters")]
    DigestFormat,
    #[error("fold index must be 0-4")]
    Fold,
    #[error("attribution belongs to a different patient")]
    WrongPatient,
    #[error("attribution belongs to a different examination")]
    WrongExamination,
    #[error("attribution belongs to a different run")]
    WrongRun,
    #[error("attribution belongs to a different eye")]
    WrongEye,
    #[error("attribution was computed for a different image")]
    WrongImage,
    #[error("attribution was computed with different models")]
    WrongModels,
    #[error("stored map failed its integrity hash")]
    Integrity,
    #[error("occlusion was cancelled")]
    Cancelled,
    #[error("model scoring failed: {0}")]
    Scoring(String),
    #[error("invalid occlusion parameters: {0}")]
    Parameters(&'static str),
    #[error("{0}")]
    Grading(#[from] Stage3GradingError),
}

/// Release checkpoint SHA-256 of the Stage I screening gate (`stage1_convnextv2l_768_best_auc_model.pth`).
pub const STAGE1_CHECKPOINT_SHA256: &str =
    "89fd275b1624f840bb20f63994710e75e2c4d5d56635011d4ff8e69f3eb60cff";

/// Reject combinations that would mislabel a result.
///
/// Grad-CAM of the five-fold ensemble is defined as the equal-weight mean of the five per-fold
/// Grad-CAM maps, each computed on its own network's last spatial map for the SAME five-way
/// probability target (the ensemble target is the mean of the folds' probabilities). It is not the
/// gradient of the mean through one shared layer: the five networks share no feature space.
/// Occlusion is not implemented for the Stage I gate.
pub fn validate_combination(
    method: AttributionMethod,
    target: AttributionTarget,
    scope: AttributionScope,
) -> Result<(), AttributionError> {
    use AttributionScope as S;
    use AttributionTarget as T;
    match (target, scope) {
        (T::Stage1DrScreening, S::Stage1Model) => {}
        (T::Stage3Severity { grade }, S::Stage3SingleFold { fold }) if (1..=4).contains(&grade) => {
            if fold > 4 {
                return Err(AttributionError::Fold);
            }
        }
        (T::Stage3Severity { grade }, S::Stage3FiveFoldEnsemble) if (1..=4).contains(&grade) => {}
        (T::Stage3Gradability, S::Stage3SingleFold { fold }) => {
            if fold > 4 {
                return Err(AttributionError::Fold);
            }
        }
        (T::Stage3Gradability, S::Stage3FiveFoldEnsemble) => {}
        (T::Stage3GradeProbability { grade }, S::Stage3FiveFoldEnsemble)
            if (1..=5).contains(&grade) => {}
        _ => {
            return Err(AttributionError::Unsupported(
                "target does not belong to the selected model scope",
            ));
        }
    }
    if method == AttributionMethod::OcclusionSensitivity && scope == S::Stage1Model {
        return Err(AttributionError::Unsupported(
            "occlusion is not implemented for the Stage I gate; use Grad-CAM",
        ));
    }
    Ok(())
}

/// Score of a Stage III target from one or more per-fold 5-way vectors (mean when several).
pub fn stage3_target_score(
    target: AttributionTarget,
    vectors: &[FullVector],
) -> Result<f64, AttributionError> {
    let mean = ensemble_mean(vectors)?;
    match target {
        AttributionTarget::Stage3Severity { grade } if (1..=4).contains(&grade) => {
            Ok(f64::from(mean[usize::from(grade) - 1]))
        }
        AttributionTarget::Stage3Gradability => Ok(1.0 - f64::from(mean[4])),
        AttributionTarget::Stage3GradeProbability { grade } if (1..=5).contains(&grade) => {
            Ok(f64::from(mean[usize::from(grade) - 1]))
        }
        _ => Err(AttributionError::Unsupported("not a Stage III target")),
    }
}

/// Source frame to model frame (crop, longest-side resize, zero padding), recorded per result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeometryTransform {
    pub original_width: u32,
    pub original_height: u32,
    pub crop_x: u32,
    pub crop_y: u32,
    pub crop_width: u32,
    pub crop_height: u32,
    pub resized_width: u32,
    pub resized_height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub model_size: u32,
}

impl GeometryTransform {
    fn validate(&self) -> Result<(), AttributionError> {
        let ok = self.original_width > 0
            && self.original_height > 0
            && self.crop_width > 0
            && self.crop_height > 0
            && self.crop_x + self.crop_width <= self.original_width
            && self.crop_y + self.crop_height <= self.original_height
            && self.resized_width > 0
            && self.resized_height > 0
            && self.offset_x + self.resized_width <= self.model_size
            && self.offset_y + self.resized_height <= self.model_size;
        if ok {
            Ok(())
        } else {
            Err(AttributionError::Dimensions)
        }
    }
}

/// Everything a result is bound to. Any mismatch on load or display rejects the result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributionProvenance {
    pub patient_id: i64,
    pub examination_id: i64,
    pub run_id: String,
    pub eye: String,
    pub image_sha256: String,
    pub model_ids: Vec<String>,
    pub model_sha256: Vec<String>,
    pub checkpoint_sha256: Vec<String>,
    pub folds: Vec<u8>,
    pub provider: String,
    pub algorithm_version: String,
    pub parameters: BTreeMap<String, String>,
    pub generated_unix: i64,
    pub research_only: bool,
}

/// What the caller currently displays; used to refuse stale or foreign attributions.
#[derive(Debug, Clone, PartialEq)]
pub struct AttributionBinding {
    pub patient_id: i64,
    pub examination_id: i64,
    pub run_id: Option<String>,
    pub eye: String,
    pub image_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttributionMeta {
    pub id: String,
    pub method: AttributionMethod,
    pub target: AttributionTarget,
    pub scope: AttributionScope,
    pub provenance: AttributionProvenance,
    /// Real target score of the unperturbed input (probability).
    pub baseline_score: f64,
    pub native_width: u32,
    pub native_height: u32,
    pub transform: GeometryTransform,
    /// Grad-CAM: feature layer name. Occlusion: empty.
    pub feature_layer: String,
    pub forward_calls: u32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributionArtifact {
    pub meta: AttributionMeta,
    /// Raw signed attribution at native resolution, row-major.
    pub raw_signed: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Support {
    pub min: f32,
    pub max: f32,
    pub max_positive: f32,
    pub has_positive: bool,
    pub has_negative: bool,
}

impl AttributionArtifact {
    pub fn validate(&self) -> Result<(), AttributionError> {
        let meta = &self.meta;
        validate_combination(meta.method, meta.target, meta.scope)?;
        let n = (meta.native_width as usize)
            .checked_mul(meta.native_height as usize)
            .ok_or(AttributionError::Dimensions)?;
        if n == 0 || self.raw_signed.len() != n {
            return Err(AttributionError::Dimensions);
        }
        if self.raw_signed.iter().any(|v| !v.is_finite()) {
            return Err(AttributionError::NonFinite);
        }
        let first = self.raw_signed[0];
        if self.raw_signed.iter().all(|v| *v == first) {
            return Err(AttributionError::Constant);
        }
        let digest = &meta.provenance.image_sha256;
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(AttributionError::DigestFormat);
        }
        if meta.provenance.folds.iter().any(|fold| *fold > 4) {
            return Err(AttributionError::Fold);
        }
        self.provenance_matches_scope()?;
        meta.transform.validate()
    }

    /// The folds and pinned checkpoint hashes recorded in the provenance must be exactly the ones the
    /// scope claims: a fold-0 map can never carry a five-fold label, and a foreign checkpoint is refused.
    fn provenance_matches_scope(&self) -> Result<(), AttributionError> {
        let p = &self.meta.provenance;
        let pinned = |folds: &[u8]| -> Vec<String> {
            folds
                .iter()
                .map(|f| FOLD_CHECKPOINT_SHA256[usize::from(*f)].to_string())
                .collect()
        };
        match self.meta.scope {
            AttributionScope::Stage1Model => {
                if !p.folds.is_empty() {
                    return Err(AttributionError::Fold);
                }
                if p.checkpoint_sha256 != [STAGE1_CHECKPOINT_SHA256] {
                    return Err(AttributionError::WrongModels);
                }
            }
            AttributionScope::Stage3SingleFold { fold } => {
                if p.folds != [fold] {
                    return Err(AttributionError::Fold);
                }
                if p.checkpoint_sha256 != pinned(&[fold]) {
                    return Err(AttributionError::WrongModels);
                }
            }
            AttributionScope::Stage3FiveFoldEnsemble => {
                if p.folds != [0, 1, 2, 3, 4] {
                    return Err(AttributionError::Fold);
                }
                if p.checkpoint_sha256 != pinned(&p.folds) {
                    return Err(AttributionError::WrongModels);
                }
            }
        }
        Ok(())
    }

    pub fn support(&self) -> Support {
        let (mut min, mut max) = (f32::MAX, f32::MIN);
        for value in &self.raw_signed {
            min = min.min(*value);
            max = max.max(*value);
        }
        Support {
            min,
            max,
            max_positive: max.max(0.0),
            has_positive: max > 0.0,
            has_negative: min < 0.0,
        }
    }

    /// Refuse an attribution that does not belong to what is on screen.
    pub fn verify_binding(&self, binding: &AttributionBinding) -> Result<(), AttributionError> {
        let p = &self.meta.provenance;
        if p.patient_id != binding.patient_id {
            return Err(AttributionError::WrongPatient);
        }
        if p.examination_id != binding.examination_id {
            return Err(AttributionError::WrongExamination);
        }
        if binding.run_id.as_deref().is_some_and(|run| run != p.run_id) {
            return Err(AttributionError::WrongRun);
        }
        if p.eye != binding.eye {
            return Err(AttributionError::WrongEye);
        }
        if p.image_sha256 != binding.image_sha256 {
            return Err(AttributionError::WrongImage);
        }
        Ok(())
    }

    /// Check that the models named in the result are the ones the caller expects (hash pinned).
    pub fn verify_models(&self, expected_model_sha256: &[String]) -> Result<(), AttributionError> {
        let mut have = self.meta.provenance.model_sha256.clone();
        let mut want = expected_model_sha256.to_vec();
        have.sort();
        want.sort();
        if have == want {
            Ok(())
        } else {
            Err(AttributionError::WrongModels)
        }
    }

    /// Little-endian f32 payload of the raw map and its SHA-256, for the encrypted store.
    pub fn map_bytes(&self) -> Vec<u8> {
        self.raw_signed
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect()
    }

    pub fn map_sha256(&self) -> String {
        hex::encode(Sha256::digest(self.map_bytes()))
    }

    /// Rebuild from stored parts; the payload hash must match and the result must validate.
    pub fn from_parts(
        meta: AttributionMeta,
        payload: &[u8],
        expected_sha256: &str,
    ) -> Result<Self, AttributionError> {
        if hex::encode(Sha256::digest(payload)) != expected_sha256
            || !payload.len().is_multiple_of(4)
        {
            return Err(AttributionError::Integrity);
        }
        let raw_signed = payload
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| f32::from_le_bytes(*c))
            .collect();
        let artifact = Self { meta, raw_signed };
        artifact.validate()?;
        Ok(artifact)
    }

    /// Model-space (512x512) signed map: occlusion maps are rebuilt exactly from the cell grid,
    /// Grad-CAM maps are bilinearly upsampled (torch `align_corners=False` convention).
    pub fn model_space_map(&self) -> Result<Vec<f32>, AttributionError> {
        self.validate()?;
        let size = self.meta.transform.model_size as usize;
        let (w, h) = (
            self.meta.native_width as usize,
            self.meta.native_height as usize,
        );
        match self.meta.method {
            AttributionMethod::GradCam => Ok(upsample_bilinear(&self.raw_signed, w, h, size, size)),
            AttributionMethod::OcclusionSensitivity => {
                let params = OcclusionParams::from_map(&self.meta.provenance.parameters, size)?;
                occlusion_pixel_map(&self.raw_signed, &params, size)
            }
        }
    }

    /// Signed map in source pixels (`original_width x original_height`) with outside-FOV pixels
    /// forced to zero. `fov` is `fov_from_rgb` of the SAME source image.
    pub fn source_map(&self, fov: &[bool]) -> Result<Vec<f32>, AttributionError> {
        let model = self.model_space_map()?;
        restore_to_source(&model, &self.meta.transform, fov)
    }
}

/// Retina mask of the source image: channel sum above the upstream fundus tolerance.
pub fn fov_from_rgb(
    rgb: &[u8],
    width: usize,
    height: usize,
) -> Result<Vec<bool>, AttributionError> {
    if width == 0 || height == 0 || rgb.len() != width * height * 3 {
        return Err(AttributionError::Dimensions);
    }
    Ok(rgb
        .as_chunks::<3>()
        .0
        .iter()
        .map(|p| u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > FOV_SUM)
        .collect())
}

/// Bilinear upsample with half-pixel centres, edges clamped (torch `align_corners=False`).
pub fn upsample_bilinear(src: &[f32], sw: usize, sh: usize, dw: usize, dh: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; dw * dh];
    for y in 0..dh {
        let fy = ((y as f32 + 0.5) * sh as f32 / dh as f32 - 0.5).max(0.0);
        for x in 0..dw {
            let fx = ((x as f32 + 0.5) * sw as f32 / dw as f32 - 0.5).max(0.0);
            out[y * dw + x] = sample(src, sw, sh, fx, fy);
        }
    }
    out
}

fn sample(src: &[f32], w: usize, h: usize, fx: f32, fy: f32) -> f32 {
    let x0 = (fx.floor() as usize).min(w - 1);
    let y0 = (fy.floor() as usize).min(h - 1);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let top = src[y0 * w + x0] * (1.0 - tx) + src[y0 * w + x1] * tx;
    let bottom = src[y1 * w + x0] * (1.0 - tx) + src[y1 * w + x1] * tx;
    top * (1.0 - ty) + bottom * ty
}

/// Inverse of crop + longest-side resize + padding: sample the model-space map at the position
/// each source pixel maps to (OpenCV linear-resize centre convention). Pixels outside the fundus
/// crop or outside `fov` are zero; nothing is drawn where the model saw only padding.
pub fn restore_to_source(
    model_map: &[f32],
    t: &GeometryTransform,
    fov: &[bool],
) -> Result<Vec<f32>, AttributionError> {
    restore_to_grid(
        model_map,
        t,
        t.original_width as usize,
        t.original_height as usize,
        fov,
    )
}

/// Same inverse onto an `out_w x out_h` grid covering the whole source frame (for display-resolution
/// textures); `fov` must be the retina mask of the image resampled to that grid.
pub fn restore_to_grid(
    model_map: &[f32],
    t: &GeometryTransform,
    out_w: usize,
    out_h: usize,
    fov: &[bool],
) -> Result<Vec<f32>, AttributionError> {
    t.validate()?;
    let size = t.model_size as usize;
    if out_w == 0 || out_h == 0 || model_map.len() != size * size || fov.len() != out_w * out_h {
        return Err(AttributionError::Dimensions);
    }
    let (w, h) = (t.original_width as f32, t.original_height as f32);
    let sx = t.resized_width as f32 / t.crop_width as f32;
    let sy = t.resized_height as f32 / t.crop_height as f32;
    let (x_lo, x_hi) = (
        t.crop_x as f32 - 0.5,
        (t.crop_x + t.crop_width) as f32 - 0.5,
    );
    let (y_lo, y_hi) = (
        t.crop_y as f32 - 0.5,
        (t.crop_y + t.crop_height) as f32 - 0.5,
    );
    let mut out = vec![0.0f32; out_w * out_h];
    for oy in 0..out_h {
        let ys = (oy as f32 + 0.5) * h / out_h as f32 - 0.5;
        if ys < y_lo || ys >= y_hi {
            continue;
        }
        let my = (ys - t.crop_y as f32 + 0.5) * sy - 0.5 + t.offset_y as f32;
        for ox in 0..out_w {
            if !fov[oy * out_w + ox] {
                continue;
            }
            let xs = (ox as f32 + 0.5) * w / out_w as f32 - 0.5;
            if xs < x_lo || xs >= x_hi {
                continue;
            }
            let mx = (xs - t.crop_x as f32 + 0.5) * sx - 0.5 + t.offset_x as f32;
            out[oy * out_w + ox] = sample(model_map, size, size, mx.max(0.0), my.max(0.0));
        }
    }
    Ok(out)
}

/// Occlusion grid: square patches on a fixed stride, replaced by a constant in normalised input
/// units (0.0 is the ImageNet mean colour). Only RGB channels 0-2 are perturbed; the five mask
/// channels stay fixed, so this measures RGB influence, not mask-stream influence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OcclusionParams {
    pub patch: usize,
    pub stride: usize,
    pub baseline_value: f32,
}

impl Default for OcclusionParams {
    fn default() -> Self {
        Self {
            patch: 64,
            stride: 64,
            baseline_value: 0.0,
        }
    }
}

impl OcclusionParams {
    pub fn to_map(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("patch".into(), self.patch.to_string()),
            ("stride".into(), self.stride.to_string()),
            ("baseline_value".into(), self.baseline_value.to_string()),
            ("perturbed_channels".into(), "rgb_only".into()),
            ("grid".into(), "square_patches_constant_fill".into()),
        ])
    }

    pub fn from_map(map: &BTreeMap<String, String>, size: usize) -> Result<Self, AttributionError> {
        let get = |key: &str| {
            map.get(key)
                .ok_or(AttributionError::Parameters("missing parameter"))
        };
        let params = Self {
            patch: get("patch")?
                .parse()
                .map_err(|_| AttributionError::Parameters("patch"))?,
            stride: get("stride")?
                .parse()
                .map_err(|_| AttributionError::Parameters("stride"))?,
            baseline_value: get("baseline_value")?
                .parse()
                .map_err(|_| AttributionError::Parameters("baseline_value"))?,
        };
        params.validate(size)?;
        Ok(params)
    }

    pub fn validate(&self, size: usize) -> Result<(), AttributionError> {
        if self.patch == 0 || self.patch > size {
            return Err(AttributionError::Parameters("patch must be 1..=size"));
        }
        if self.stride == 0 || self.stride > self.patch {
            return Err(AttributionError::Parameters(
                "stride must be 1..=patch (full coverage)",
            ));
        }
        if !self.baseline_value.is_finite() {
            return Err(AttributionError::Parameters("baseline must be finite"));
        }
        Ok(())
    }

    /// Patch origins along one axis; a last patch is added so the border is always covered.
    pub fn origins(&self, size: usize) -> Vec<usize> {
        let mut origins: Vec<usize> = (0..=size - self.patch).step_by(self.stride).collect();
        if origins.last().is_some_and(|last| last + self.patch < size) {
            origins.push(size - self.patch);
        }
        origins
    }
}

/// Spread each cell value over the pixels its patch covers and divide by the overlap count.
pub fn occlusion_pixel_map(
    cells: &[f32],
    params: &OcclusionParams,
    size: usize,
) -> Result<Vec<f32>, AttributionError> {
    params.validate(size)?;
    let origins = params.origins(size);
    if cells.len() != origins.len() * origins.len() {
        return Err(AttributionError::Dimensions);
    }
    let mut sum = vec![0.0f32; size * size];
    let mut count = vec![0u32; size * size];
    for (gy, &oy) in origins.iter().enumerate() {
        for (gx, &ox) in origins.iter().enumerate() {
            let value = cells[gy * origins.len() + gx];
            for y in oy..oy + params.patch {
                for x in ox..ox + params.patch {
                    sum[y * size + x] += value;
                    count[y * size + x] += 1;
                }
            }
        }
    }
    Ok(sum
        .iter()
        .zip(&count)
        .map(|(s, c)| if *c == 0 { 0.0 } else { s / *c as f32 })
        .collect())
}

#[derive(Debug, Clone, PartialEq)]
pub struct OcclusionRaw {
    pub baseline_score: f64,
    /// `baseline - perturbed` per patch, row-major over the origin grid.
    pub cells: Vec<f32>,
    pub perturbed_scores: Vec<f64>,
    pub grid: usize,
    pub forward_calls: u32,
}

/// Slide the occluder over `tensor` (`[C][size][size]`, RGB = channels 0-2) and score every
/// variant through `score`. The tensor is restored after each patch. `cancelled` is polled between
/// forward calls only: a running inference cannot be interrupted. `progress(done, total)` counts
/// finished forward calls including the baseline.
pub fn run_occlusion<E: std::fmt::Display>(
    tensor: &mut [f32],
    channels: usize,
    size: usize,
    params: &OcclusionParams,
    mut score: impl FnMut(&[f32]) -> Result<f64, E>,
    mut cancelled: impl FnMut() -> bool,
    mut progress: impl FnMut(u32, u32),
) -> Result<OcclusionRaw, AttributionError> {
    params.validate(size)?;
    if channels < 3 || tensor.len() != channels * size * size {
        return Err(AttributionError::Dimensions);
    }
    let origins = params.origins(size);
    let total = (origins.len() * origins.len() + 1) as u32;
    let plane = size * size;
    let run = |input: &[f32], score: &mut dyn FnMut(&[f32]) -> Result<f64, E>| {
        let value = score(input).map_err(|e| AttributionError::Scoring(e.to_string()))?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(AttributionError::NonFinite)
        }
    };
    if cancelled() {
        return Err(AttributionError::Cancelled);
    }
    let baseline_score = run(tensor, &mut score)?;
    let mut calls = 1u32;
    progress(calls, total);
    let mut cells = Vec::with_capacity(origins.len() * origins.len());
    let mut perturbed_scores = Vec::with_capacity(cells.capacity());
    let mut saved = vec![0.0f32; 3 * params.patch * params.patch];
    for &oy in &origins {
        for &ox in &origins {
            if cancelled() {
                return Err(AttributionError::Cancelled);
            }
            let mut slot = 0;
            for channel in 0..3 {
                for y in oy..oy + params.patch {
                    let row = channel * plane + y * size + ox;
                    saved[slot..slot + params.patch]
                        .copy_from_slice(&tensor[row..row + params.patch]);
                    tensor[row..row + params.patch].fill(params.baseline_value);
                    slot += params.patch;
                }
            }
            let result = run(tensor, &mut score);
            slot = 0;
            for channel in 0..3 {
                for y in oy..oy + params.patch {
                    let row = channel * plane + y * size + ox;
                    tensor[row..row + params.patch]
                        .copy_from_slice(&saved[slot..slot + params.patch]);
                    slot += params.patch;
                }
            }
            let perturbed = result?;
            cells.push((baseline_score - perturbed) as f32);
            perturbed_scores.push(perturbed);
            calls += 1;
            progress(calls, total);
        }
    }
    Ok(OcclusionRaw {
        baseline_score,
        cells,
        perturbed_scores,
        grid: origins.len(),
        forward_calls: calls,
    })
}

/// Deletion test: remove the top-`fraction` most positive cells (largest score drop first) at once
/// and re-score. Faithful attributions make the score fall faster than random removal.
pub fn cells_to_delete(cells: &[f32], fraction: f32) -> Vec<usize> {
    let mut order: Vec<usize> = (0..cells.len()).collect();
    order.sort_by(|a, b| cells[*b].total_cmp(&cells[*a]));
    order.truncate(((cells.len() as f32) * fraction).round() as usize);
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::Infallible;

    const SIZE: usize = 512;

    fn transform() -> GeometryTransform {
        GeometryTransform {
            original_width: 640,
            original_height: 640,
            crop_x: 0,
            crop_y: 0,
            crop_width: 640,
            crop_height: 640,
            resized_width: 512,
            resized_height: 512,
            offset_x: 0,
            offset_y: 0,
            model_size: 512,
        }
    }

    fn meta(
        method: AttributionMethod,
        target: AttributionTarget,
        scope: AttributionScope,
    ) -> AttributionMeta {
        let params = OcclusionParams::default();
        AttributionMeta {
            id: "id".into(),
            method,
            target,
            scope,
            provenance: AttributionProvenance {
                patient_id: 7,
                examination_id: 9,
                run_id: "run-a".into(),
                eye: "left".into(),
                image_sha256: "a".repeat(64),
                model_ids: vec!["m".into()],
                model_sha256: vec!["h1".into()],
                checkpoint_sha256: match scope {
                    AttributionScope::Stage1Model => vec![STAGE1_CHECKPOINT_SHA256.to_string()],
                    AttributionScope::Stage3SingleFold { fold } => {
                        vec![FOLD_CHECKPOINT_SHA256[usize::from(fold)].to_string()]
                    }
                    AttributionScope::Stage3FiveFoldEnsemble => FOLD_CHECKPOINT_SHA256
                        .iter()
                        .map(|h| h.to_string())
                        .collect(),
                },
                folds: match scope {
                    AttributionScope::Stage1Model => vec![],
                    AttributionScope::Stage3SingleFold { fold } => vec![fold],
                    AttributionScope::Stage3FiveFoldEnsemble => vec![0, 1, 2, 3, 4],
                },
                provider: "cpu".into(),
                algorithm_version: ALGORITHM_VERSION.into(),
                parameters: params.to_map(),
                generated_unix: 1,
                research_only: true,
            },
            baseline_score: 0.5,
            native_width: 8,
            native_height: 8,
            transform: transform(),
            feature_layer: String::new(),
            forward_calls: 65,
            elapsed_ms: 1,
        }
    }

    fn ramp_artifact() -> AttributionArtifact {
        AttributionArtifact {
            meta: meta(
                AttributionMethod::OcclusionSensitivity,
                AttributionTarget::Stage3Severity { grade: 2 },
                AttributionScope::Stage3SingleFold { fold: 0 },
            ),
            raw_signed: (0..64).map(|i| i as f32 - 20.0).collect(),
        }
    }

    // T19: 8x8 grid, patch 64 stride 64 -> 64 perturbed forwards + baseline, deterministic.
    // T18: cell values are exactly baseline - score(perturbed) of the supplied scorer.
    #[test]
    fn grid_is_8x8_with_65_forward_calls_and_exact_deltas() {
        let mut tensor = vec![1.0f32; 8 * SIZE * SIZE];
        let mut seen = 0;
        // Score = sum of the RGB channel region (0,0)-(63,63) of channel 0; only patch (0,0) changes it.
        let scorer = |t: &[f32]| -> Result<f64, Infallible> {
            Ok(t[..64].iter().map(|v| f64::from(*v)).sum::<f64>())
        };
        let raw = run_occlusion(
            &mut tensor,
            8,
            SIZE,
            &OcclusionParams::default(),
            scorer,
            || false,
            |done, total| {
                seen = done;
                assert_eq!(total, 65);
            },
        )
        .unwrap();
        assert_eq!((raw.grid, raw.forward_calls, seen), (8, 65, 65));
        assert_eq!(raw.baseline_score, 64.0);
        assert_eq!(raw.cells[0], 64.0); // baseline 64 - perturbed 0 (value 0.0 fill)
        assert!(raw.cells[1..].iter().all(|c| *c == 0.0));
        assert!(
            tensor.iter().all(|v| *v == 1.0),
            "input restored after every patch"
        );
    }

    // T20: overlapping patches are averaged by their coverage count.
    #[test]
    fn overlap_is_normalised_by_contribution_count() {
        let params = OcclusionParams {
            patch: 64,
            stride: 32,
            baseline_value: 0.0,
        };
        assert_eq!(params.origins(SIZE).len(), 15);
        let cells = vec![1.0f32; 15 * 15];
        let map = occlusion_pixel_map(&cells, &params, SIZE).unwrap();
        assert!(
            map.iter().all(|v| (*v - 1.0).abs() < 1e-6),
            "constant cells stay constant"
        );
        let mut one = vec![0.0f32; 15 * 15];
        one[0] = 4.0; // top-left patch only: pixel (0,0) is covered once, pixel (40,40) four times
        let map = occlusion_pixel_map(&one, &params, SIZE).unwrap();
        assert_eq!(map[0], 4.0);
        assert!((map[40 * SIZE + 40] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn non_divisible_stride_still_covers_the_border() {
        let params = OcclusionParams {
            patch: 64,
            stride: 60,
            baseline_value: 0.0,
        };
        let origins = params.origins(SIZE);
        assert_eq!(*origins.last().unwrap() + 64, SIZE);
        let cells = vec![1.0f32; origins.len() * origins.len()];
        let map = occlusion_pixel_map(&cells, &params, SIZE).unwrap();
        assert!(map.iter().all(|v| *v > 0.0));
    }

    // T21/T22: cancellation before the baseline and between forwards.
    #[test]
    fn cancel_stops_between_forwards_and_restores_input() {
        let mut tensor = vec![1.0f32; 8 * SIZE * SIZE];
        let calls = std::cell::Cell::new(0);
        let result = run_occlusion(
            &mut tensor,
            8,
            SIZE,
            &OcclusionParams::default(),
            |_| -> Result<f64, Infallible> {
                calls.set(calls.get() + 1);
                Ok(1.0)
            },
            || calls.get() >= 3,
            |_, _| {},
        );
        assert_eq!(result, Err(AttributionError::Cancelled));
        assert_eq!(calls.get(), 3);
        assert!(tensor.iter().all(|v| *v == 1.0));
        let queued = run_occlusion(
            &mut vec![1.0f32; 8 * SIZE * SIZE],
            8,
            SIZE,
            &OcclusionParams::default(),
            |_| -> Result<f64, Infallible> { panic!("must not score a cancelled job") },
            || true,
            |_, _| {},
        );
        assert_eq!(queued, Err(AttributionError::Cancelled));
    }

    // T23: a failing or non-finite score surfaces as an error and still restores the input.
    #[test]
    fn scoring_failure_and_nonfinite_score_are_errors() {
        let mut tensor = vec![1.0f32; 8 * SIZE * SIZE];
        let mut n = 0;
        let failed = run_occlusion(
            &mut tensor,
            8,
            SIZE,
            &OcclusionParams::default(),
            |_| {
                n += 1;
                if n == 5 { Err("session lost") } else { Ok(0.5) }
            },
            || false,
            |_, _| {},
        );
        assert_eq!(
            failed,
            Err(AttributionError::Scoring("session lost".into()))
        );
        assert!(tensor.iter().all(|v| *v == 1.0));
        let bad = run_occlusion(
            &mut tensor,
            8,
            SIZE,
            &OcclusionParams::default(),
            |_| -> Result<f64, Infallible> { Ok(f64::NAN) },
            || false,
            |_, _| {},
        );
        assert_eq!(bad, Err(AttributionError::NonFinite));
    }

    #[test]
    fn bad_parameters_are_rejected() {
        let tensor = &mut vec![0.0f32; 8 * SIZE * SIZE];
        for params in [
            OcclusionParams {
                patch: 0,
                stride: 1,
                baseline_value: 0.0,
            },
            OcclusionParams {
                patch: 64,
                stride: 65,
                baseline_value: 0.0,
            },
            OcclusionParams {
                patch: 600,
                stride: 64,
                baseline_value: 0.0,
            },
            OcclusionParams {
                patch: 64,
                stride: 64,
                baseline_value: f32::NAN,
            },
        ] {
            let r = run_occlusion(
                tensor,
                8,
                SIZE,
                &params,
                |_| -> Result<f64, Infallible> { Ok(0.0) },
                || false,
                |_, _| {},
            );
            assert!(matches!(r, Err(AttributionError::Parameters(_))));
        }
    }

    // T01-T04: target scores read the right entry of the 5-way vector; five-fold uses the mean.
    #[test]
    fn target_scores_follow_the_five_way_vector() {
        let a: FullVector = [0.1, 0.2, 0.3, 0.2, 0.2];
        let b: FullVector = [0.3, 0.2, 0.1, 0.2, 0.2];
        let sev2 = AttributionTarget::Stage3Severity { grade: 2 };
        assert!((stage3_target_score(sev2, &[a]).unwrap() - 0.2).abs() < 1e-7);
        let gradability = stage3_target_score(AttributionTarget::Stage3Gradability, &[a]).unwrap();
        assert!((gradability - 0.8).abs() < 1e-7);
        let mean =
            stage3_target_score(AttributionTarget::Stage3Severity { grade: 1 }, &[a, b]).unwrap();
        assert!(
            (mean - 0.2).abs() < 1e-7,
            "mean of 0.1 and 0.3, not fold 0 (0.1)"
        );
        let ungradable = stage3_target_score(
            AttributionTarget::Stage3GradeProbability { grade: 5 },
            &[a, b],
        )
        .unwrap();
        assert!((ungradable - 0.2).abs() < 1e-7);
        assert!(stage3_target_score(AttributionTarget::Stage3Severity { grade: 5 }, &[a]).is_err());
        assert!(stage3_target_score(AttributionTarget::Stage1DrScreening, &[a]).is_err());
        assert!(stage3_target_score(sev2, &[]).is_err());
    }

    #[test]
    fn combinations_are_checked() {
        use AttributionMethod::*;
        use AttributionScope::*;
        use AttributionTarget::*;
        assert!(validate_combination(GradCam, Stage1DrScreening, Stage1Model).is_ok());
        assert!(
            validate_combination(
                OcclusionSensitivity,
                Stage3GradeProbability { grade: 5 },
                Stage3FiveFoldEnsemble
            )
            .is_ok()
        );
        assert!(validate_combination(GradCam, Stage1DrScreening, Stage3FiveFoldEnsemble).is_err());
        // Five-fold Grad-CAM is the mean of the per-fold maps of the same five-way target.
        assert!(
            validate_combination(
                GradCam,
                Stage3GradeProbability { grade: 3 },
                Stage3FiveFoldEnsemble
            )
            .is_ok()
        );
        assert!(validate_combination(GradCam, Stage3Gradability, Stage3FiveFoldEnsemble).is_ok());
        assert!(
            validate_combination(OcclusionSensitivity, Stage1DrScreening, Stage1Model).is_err(),
            "Stage I occlusion is not implemented"
        );
        assert!(
            validate_combination(
                OcclusionSensitivity,
                Stage3Severity { grade: 2 },
                Stage3SingleFold { fold: 5 }
            )
            .is_err()
        );
        assert!(
            validate_combination(
                OcclusionSensitivity,
                Stage3GradeProbability { grade: 2 },
                Stage3SingleFold { fold: 0 }
            )
            .is_err()
        );
    }

    // T08-T11: foreign patient, exam, run, eye or image is refused.
    #[test]
    fn binding_rejects_foreign_results() {
        let artifact = ramp_artifact();
        let ok = AttributionBinding {
            patient_id: 7,
            examination_id: 9,
            run_id: Some("run-a".into()),
            eye: "left".into(),
            image_sha256: "a".repeat(64),
        };
        assert_eq!(artifact.verify_binding(&ok), Ok(()));
        let mut b = ok.clone();
        b.patient_id = 8;
        assert_eq!(
            artifact.verify_binding(&b),
            Err(AttributionError::WrongPatient)
        );
        let mut b = ok.clone();
        b.examination_id = 10;
        assert_eq!(
            artifact.verify_binding(&b),
            Err(AttributionError::WrongExamination)
        );
        let mut b = ok.clone();
        b.run_id = Some("run-b".into());
        assert_eq!(artifact.verify_binding(&b), Err(AttributionError::WrongRun));
        let mut b = ok.clone();
        b.eye = "right".into();
        assert_eq!(artifact.verify_binding(&b), Err(AttributionError::WrongEye));
        let mut b = ok;
        b.image_sha256 = "b".repeat(64);
        assert_eq!(
            artifact.verify_binding(&b),
            Err(AttributionError::WrongImage)
        );
        assert_eq!(
            artifact.verify_models(&["h2".into()]),
            Err(AttributionError::WrongModels)
        );
        assert_eq!(artifact.verify_models(&["h1".into()]), Ok(()));
    }

    // P4-C3: fold identity and pinned checkpoint hashes are part of validity.
    #[test]
    fn fold_and_checkpoint_identity_must_match_the_scope() {
        let good = ramp_artifact(); // single fold 0
        assert_eq!(good.validate(), Ok(()));
        // A fold-1 checkpoint under a fold-0 label.
        let mut a = good.clone();
        a.meta.provenance.checkpoint_sha256 = vec![FOLD_CHECKPOINT_SHA256[1].to_string()];
        assert_eq!(a.validate(), Err(AttributionError::WrongModels));
        // A corrupted (unpinned) checkpoint hash.
        let mut a = good.clone();
        a.meta.provenance.checkpoint_sha256 = vec!["0".repeat(64)];
        assert_eq!(a.validate(), Err(AttributionError::WrongModels));
        // Fold list that disagrees with the scope.
        let mut a = good.clone();
        a.meta.provenance.folds = vec![2];
        assert_eq!(a.validate(), Err(AttributionError::Fold));
        // A fold-0 map relabelled as the five-fold ensemble.
        let mut a = good.clone();
        a.meta.scope = AttributionScope::Stage3FiveFoldEnsemble;
        a.meta.target = AttributionTarget::Stage3GradeProbability { grade: 2 };
        assert_eq!(a.validate(), Err(AttributionError::Fold));
        // A genuine five-fold record validates; dropping one fold or swapping the order does not.
        let mut five = good.clone();
        five.meta.scope = AttributionScope::Stage3FiveFoldEnsemble;
        five.meta.target = AttributionTarget::Stage3GradeProbability { grade: 2 };
        five.meta.provenance.folds = vec![0, 1, 2, 3, 4];
        five.meta.provenance.checkpoint_sha256 = FOLD_CHECKPOINT_SHA256
            .iter()
            .map(|h| h.to_string())
            .collect();
        assert_eq!(five.validate(), Ok(()));
        let mut four = five.clone();
        four.meta.provenance.folds.pop();
        four.meta.provenance.checkpoint_sha256.pop();
        assert_eq!(four.validate(), Err(AttributionError::Fold));
        let mut swapped = five.clone();
        swapped.meta.provenance.checkpoint_sha256.swap(0, 1);
        assert_eq!(swapped.validate(), Err(AttributionError::WrongModels));
        // Stage I never carries folds.
        let mut s1 = good;
        s1.meta.scope = AttributionScope::Stage1Model;
        s1.meta.method = AttributionMethod::GradCam;
        s1.meta.target = AttributionTarget::Stage1DrScreening;
        assert_eq!(
            s1.validate(),
            Err(AttributionError::Fold),
            "fold list present"
        );
        s1.meta.provenance.folds.clear();
        assert_eq!(
            s1.validate(),
            Err(AttributionError::WrongModels),
            "a fold-0 checkpoint is not the Stage I gate"
        );
        s1.meta.provenance.checkpoint_sha256 = vec![STAGE1_CHECKPOINT_SHA256.to_string()];
        assert_eq!(s1.validate(), Ok(()));
    }

    // T12-T14 and T32: shape, non-finite, constant, corrupted payload.
    #[test]
    fn invalid_maps_are_rejected() {
        let mut a = ramp_artifact();
        a.raw_signed.pop();
        assert_eq!(a.validate(), Err(AttributionError::Dimensions));
        let mut a = ramp_artifact();
        a.raw_signed[3] = f32::NAN;
        assert_eq!(a.validate(), Err(AttributionError::NonFinite));
        let mut a = ramp_artifact();
        a.raw_signed.fill(0.0);
        assert_eq!(a.validate(), Err(AttributionError::Constant));
        let mut a = ramp_artifact();
        a.meta.provenance.image_sha256 = "xyz".into();
        assert_eq!(a.validate(), Err(AttributionError::DigestFormat));
        let mut a = ramp_artifact();
        a.meta.transform.offset_x = 500;
        assert_eq!(a.validate(), Err(AttributionError::Dimensions));
        let negative_only = AttributionArtifact {
            raw_signed: (0..64).map(|i| -1.0 - i as f32).collect(),
            ..ramp_artifact()
        };
        assert!(negative_only.validate().is_ok());
        let s = negative_only.support();
        assert!(!s.has_positive && s.has_negative && s.max_positive == 0.0);
    }

    // T31/T32: byte payload round trip is bit-identical; a flipped byte is rejected.
    #[test]
    fn payload_round_trip_is_exact_and_tamper_evident() {
        let a = ramp_artifact();
        let bytes = a.map_bytes();
        let hash = a.map_sha256();
        let back = AttributionArtifact::from_parts(a.meta.clone(), &bytes, &hash).unwrap();
        assert_eq!(back, a);
        let mut bad = bytes.clone();
        bad[10] ^= 1;
        assert_eq!(
            AttributionArtifact::from_parts(a.meta.clone(), &bad, &hash),
            Err(AttributionError::Integrity)
        );
        assert_eq!(
            AttributionArtifact::from_parts(a.meta.clone(), &bytes[..bytes.len() - 1], &hash),
            Err(AttributionError::Integrity)
        );
    }

    // T25: non-square crop with padding maps back to the right source pixels.
    #[test]
    fn inverse_transform_for_non_square_crop() {
        // 1000x500 source, fundus crop = whole frame, longest side 1000 -> 512, so 512x256 with
        // 128 rows of zero padding above and below.
        let t = GeometryTransform {
            original_width: 1000,
            original_height: 500,
            crop_x: 0,
            crop_y: 0,
            crop_width: 1000,
            crop_height: 500,
            resized_width: 512,
            resized_height: 256,
            offset_x: 0,
            offset_y: 128,
            model_size: 512,
        };
        let mut model = vec![0.0f32; SIZE * SIZE];
        // Mark the model pixel for source (500, 250): mx = (500.5)*0.512-0.5, my = (250.5)*0.512-0.5+128.
        for y in 0..SIZE {
            for x in 0..SIZE {
                model[y * SIZE + x] = (x as f32) * 1000.0 + y as f32; // unique, smooth
            }
        }
        let fov = vec![true; 1000 * 500];
        let out = restore_to_source(&model, &t, &fov).unwrap();
        let mx = 500.5 * 0.512 - 0.5;
        let my = 250.5 * 0.512 - 0.5 + 128.0;
        let expected = mx * 1000.0 + my;
        assert!(
            (out[250 * 1000 + 500] - expected).abs() < 1.0,
            "{} vs {}",
            out[250 * 1000 + 500],
            expected
        );
        // Top-left source pixel samples inside the image region, never the padding rows (y < 128).
        assert!(out[0] >= 128.0 - 1.0);
    }

    // T26: pixels outside the retina mask are zero, even when the model map is non-zero there.
    #[test]
    fn outside_fov_is_zeroed() {
        let model = vec![1.0f32; SIZE * SIZE];
        let mut rgb = vec![0u8; 640 * 640 * 3];
        for y in 100..540 {
            for x in 100..540 {
                rgb[(y * 640 + x) * 3] = 120;
            }
        }
        let fov = fov_from_rgb(&rgb, 640, 640).unwrap();
        let out = restore_to_source(&model, &transform(), &fov).unwrap();
        assert_eq!(out[10 * 640 + 10], 0.0);
        assert!((out[300 * 640 + 300] - 1.0).abs() < 1e-6);
        assert!(fov_from_rgb(&rgb[..10], 640, 640).is_err());
    }

    #[test]
    fn model_space_map_matches_documented_rebuild() {
        let a = ramp_artifact();
        let map = a.model_space_map().unwrap();
        assert_eq!(map.len(), SIZE * SIZE);
        // Cell (0,0) = -20 covers pixel (0,0); cell (7,7) = 43 covers pixel (511,511).
        assert_eq!(map[0], -20.0);
        assert_eq!(map[SIZE * SIZE - 1], 43.0);
        let g = AttributionArtifact {
            meta: AttributionMeta {
                method: AttributionMethod::GradCam,
                target: AttributionTarget::Stage3Severity { grade: 2 },
                scope: AttributionScope::Stage3SingleFold { fold: 0 },
                native_width: 4,
                native_height: 4,
                ..a.meta.clone()
            },
            raw_signed: (0..16).map(|i| i as f32).collect(),
        };
        let m = g.model_space_map().unwrap();
        assert!(m[0] >= 0.0 && m[SIZE * SIZE - 1] <= 15.0);
        assert!(m[SIZE * SIZE - 1] > m[0]);
    }

    #[test]
    fn deletion_order_removes_largest_drops_first() {
        assert_eq!(cells_to_delete(&[0.1, 0.9, -0.2, 0.5], 0.5), vec![1, 3]);
    }
}
