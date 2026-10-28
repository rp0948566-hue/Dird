//! HierarchiRetina v1.0 LG-DRG output algebra: CORN logits to a 5-way vector, fold-mean ensemble
//! and the deployed consecutive decode (`hierarchiretina/stage3/corn.py`, pinned `bf84b1d3`).
//!
//! Grades 1-4 are ordinal severity; grade 5 means ungradable and overrides severity. Grade 0 is the
//! Stage I negative route and is never produced here.
use crate::{
    device::ActualProvider, model_runtime::ModelRuntimeError, onnx_session::SessionError,
    stage2_contract::Stage2MaskResult, stage2_preprocessing::Stage2PreprocessError,
    stage3_input::Stage3InputError,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Out-of-fold fitted CORN thresholds used for every reported upstream test number (`PAPER_T`).
pub const CORN_THRESHOLDS: [f64; 3] = [0.38, 0.60, 0.19];
/// Grade 5 (ungradable) when P(gradable) is below this value (`PAPER_TG`).
pub const GRADABILITY_THRESHOLD: f64 = 0.10;
/// Stage I operating point: probability at or above it is routed to Stages II and III.
pub const STAGE1_GATE_THRESHOLD: f64 = 0.2391715943813324;

/// Probabilities for severity grades 1-4 (scaled by P(gradable)) followed by P(ungradable).
pub type FullVector = [f32; 5];

#[derive(Debug, Error, PartialEq)]
pub enum Stage3GradingError {
    #[error("model output contains a non-finite value")]
    NonFinite,
    #[error("ensemble needs at least one fold vector")]
    EmptyEnsemble,
    #[error("probability vector is not a valid distribution")]
    InvalidVector,
}

fn sigmoid(value: f32) -> f32 {
    1.0 / (1.0 + (-value).exp())
}

/// Port of `lgdrg_predict` for one image: three CORN logits and one gradability logit.
pub fn full_vector(
    sev_logits: [f32; 3],
    grad_logit: f32,
) -> Result<FullVector, Stage3GradingError> {
    if sev_logits.iter().any(|value| !value.is_finite()) || !grad_logit.is_finite() {
        return Err(Stage3GradingError::NonFinite);
    }
    let p_gradable = sigmoid(grad_logit);
    let p_ungradable = 1.0 - p_gradable;
    let probs = sev_logits.map(sigmoid);
    let cum = [
        probs[0],
        probs[0] * probs[1],
        probs[0] * probs[1] * probs[2],
    ];
    let mut severity = [1.0 - cum[0], cum[0] - cum[1], cum[1] - cum[2], cum[2]];
    for value in &mut severity {
        *value = value.max(1e-9);
    }
    let total: f32 = severity.iter().sum();
    let mut full = [0.0f32; 5];
    for (slot, value) in full.iter_mut().zip(severity) {
        *slot = value / total * p_gradable;
    }
    full[4] = p_ungradable;
    let sum: f32 = full.iter().sum();
    Ok(full.map(|value| value / sum))
}

/// Mean of the folds' 5-way vectors (`engine.ensemble_probs`). Fold order does not change the result.
pub fn ensemble_mean(vectors: &[FullVector]) -> Result<FullVector, Stage3GradingError> {
    let first = vectors.first().ok_or(Stage3GradingError::EmptyEnsemble)?;
    let mut total = *first;
    for vector in &vectors[1..] {
        for (slot, value) in total.iter_mut().zip(vector) {
            *slot += value;
        }
    }
    let count = vectors.len() as f32;
    let mean = total.map(|value| value / count);
    if mean.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return Err(Stage3GradingError::InvalidVector);
    }
    Ok(mean)
}

/// Port of `corn_from_full`: invert a (possibly averaged) 5-way vector to `(p1, p2, p3, p_gradable)`.
pub fn corn_from_full(full: FullVector) -> Result<[f64; 4], Stage3GradingError> {
    if full.iter().any(|value| !value.is_finite()) {
        return Err(Stage3GradingError::NonFinite);
    }
    let p = full.map(f64::from);
    let p_ungradable = p[4].clamp(0.0, 1.0 - 1e-9);
    let p_gradable = (1.0 - p_ungradable).clamp(1e-9, 1.0);
    let mut severity = [0.0f64; 4];
    for (slot, value) in severity.iter_mut().zip(&p[..4]) {
        *slot = (value / p_gradable).max(1e-12);
    }
    let total: f64 = severity.iter().sum();
    severity = severity.map(|value| value / total);
    let p1 = (1.0 - severity[0]).clamp(1e-9, 1.0 - 1e-9);
    let tail = (severity[2] + severity[3]).max(1e-12);
    let p2 = (tail / p1).clamp(1e-9, 1.0 - 1e-9);
    let p3 = (severity[3] / tail).clamp(1e-9, 1.0 - 1e-9);
    Ok([p1, p2, p3, p_gradable])
}

/// Port of `decode_consecutive`: grade 5 if `p_gradable < tg`, else 1 + number of leading thresholds passed.
pub fn decode_consecutive(probabilities: [f64; 4], thresholds: [f64; 3], tg: f64) -> u8 {
    let [p1, p2, p3, p_gradable] = probabilities;
    if p_gradable < tg {
        return 5;
    }
    let first = p1 > thresholds[0];
    let second = first && p2 > thresholds[1];
    let third = second && p3 > thresholds[2];
    1 + u8::from(first) + u8::from(second) + u8::from(third)
}

/// Deployed decode of a single-fold or averaged vector (`decode_full` with the pinned thresholds).
pub fn decode_full(full: FullVector) -> Result<u8, Stage3GradingError> {
    Ok(decode_consecutive(
        corn_from_full(full)?,
        CORN_THRESHOLDS,
        GRADABILITY_THRESHOLD,
    ))
}

/// Release checkpoint SHA-256 of fold k (`weights_manifest.json` at the pinned commit).
pub const FOLD_CHECKPOINT_SHA256: [&str; 5] = [
    "c1593eacca1148213c89424f19c47b4997d30eaf62cdff971ed147e529ccf57c",
    "9efb5c932ef7142e36f4e6b0b1f170936fdf771df659dbd53a0b82fa889714eb",
    "2336a1a56cb82c2f6eb47eb981e57974d9f47b066382e2c2b1b9d923b6f91e0f",
    "67341f8973c759bcf3dcb72e1dc75058d37a4b573e511fd7e180e663a9acdb99",
    "239107b0c4f30cdf3aa92dd16b1bb7df564e4add5edac831ca59b85ca1ee888e",
];

/// Whether a grade came from one fold (technical test) or the full five-fold ensemble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum Stage3Scope {
    SingleFoldTechnicalTest { fold: u8 },
    FiveFoldEnsemble,
}

#[derive(Debug, Clone)]
pub struct Stage3Request {
    pub examination_id: i64,
    pub image_path: PathBuf,
    /// Exactly one complete Stage II result per mask kind, for the same image and examination.
    pub masks: Vec<Stage2MaskResult>,
    pub model_id: String,
}

/// Research-only LG-DRG result. Grade 5 means ungradable; grades 1-4 are ordinal severity.
/// Grade 0 (Stage I negative route) is decided by the cascade, never by this model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage3GradeResult {
    pub examination_id: i64,
    pub source_image_sha256: String,
    pub scope: Stage3Scope,
    pub model_version: String,
    pub checkpoint_sha256: Vec<String>,
    pub model_sha256: Vec<String>,
    /// Raw CORN logits and gradability logit; present for a single fold only.
    pub severity_logits: Option<[f32; 3]>,
    pub gradability_logit: Option<f32>,
    pub full_probabilities: FullVector,
    pub ordinal_probabilities: [f64; 3],
    pub gradable_probability: f64,
    pub corn_thresholds: [f64; 3],
    pub gradability_threshold: f64,
    pub grade: u8,
    pub ungradable: bool,
    pub provider: ActualProvider,
    pub elapsed_microseconds: u128,
    pub research_only: bool,
}

#[derive(Debug, Error)]
pub enum Stage3InferenceError {
    #[error("Stage III request is invalid: {0}")]
    InvalidRequest(&'static str),
    #[error(transparent)]
    Runtime(#[from] ModelRuntimeError),
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Input(#[from] Stage3InputError),
    #[error("Stage III source image could not be read: {0}")]
    Source(#[from] Stage2PreprocessError),
    #[error(transparent)]
    Grading(#[from] Stage3GradingError),
    #[error("loaded ONNX model is not an authentic LG-DRG fold")]
    IncompatibleModel,
    #[error("LG-DRG model returned {0} values; expected 4")]
    BadOutput(usize),
}

/// One fold's decoded output.
#[derive(Debug, Clone, PartialEq)]
pub struct SingleFoldGrade {
    pub severity_logits: [f32; 3],
    pub gradability_logit: f32,
    pub full: FullVector,
    /// `(p1, p2, p3, p_gradable)` recovered by `corn_from_full`.
    pub probabilities: [f64; 4],
    pub grade: u8,
}

/// Grade a single fold's raw output: three CORN logits then the gradability logit.
pub fn grade_single_fold(logits: &[f32]) -> Result<SingleFoldGrade, Stage3InferenceError> {
    let [s1, s2, s3, g] = *logits else {
        return Err(Stage3InferenceError::BadOutput(logits.len()));
    };
    let full = full_vector([s1, s2, s3], g)?;
    let probabilities = corn_from_full(full)?;
    let grade = decode_consecutive(probabilities, CORN_THRESHOLDS, GRADABILITY_THRESHOLD);
    Ok(SingleFoldGrade {
        severity_logits: [s1, s2, s3],
        gradability_logit: g,
        full,
        probabilities,
        grade,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct Fold {
        sev: [f32; 3],
        grad: f32,
    }

    #[derive(Deserialize)]
    struct Single {
        sev: [f32; 3],
        grad: f32,
        full: [f32; 5],
        p: [f64; 4],
        grade: u8,
    }

    #[derive(Deserialize)]
    struct Ensemble {
        folds: Vec<Fold>,
        mean_full: [f32; 5],
        grade: u8,
    }

    #[derive(Deserialize)]
    struct Vectors {
        thresholds: [f64; 3],
        gradability_threshold: f64,
        single: Vec<Single>,
        ensemble: Vec<Ensemble>,
    }

    fn vectors() -> Vectors {
        serde_json::from_str(include_str!("../fixtures/stage3_corn_vectors.json")).unwrap()
    }

    #[test]
    fn pinned_thresholds_match_the_upstream_fixture_header() {
        let v = vectors();
        assert_eq!(v.thresholds, CORN_THRESHOLDS);
        assert_eq!(v.gradability_threshold, GRADABILITY_THRESHOLD);
    }

    #[test]
    fn single_fold_vectors_probabilities_and_grades_match_upstream() {
        for case in vectors().single {
            let full = full_vector(case.sev, case.grad).unwrap();
            for (rust, python) in full.iter().zip(case.full) {
                assert!(
                    (rust - python).abs() < 2e-6,
                    "full {full:?} vs {:?}",
                    case.full
                );
            }
            let p = corn_from_full(case.full).unwrap();
            for (rust, python) in p.iter().zip(case.p) {
                assert!((rust - python).abs() < 1e-9, "p {p:?} vs {:?}", case.p);
            }
            assert_eq!(decode_full(case.full).unwrap(), case.grade);
        }
    }

    #[test]
    fn five_fold_mean_and_decision_match_upstream() {
        let cases = vectors().ensemble;
        let distinct: std::collections::BTreeSet<u8> =
            cases.iter().map(|case| case.grade).collect();
        assert!(
            distinct.len() >= 2,
            "fixture should exercise several grades: {distinct:?}"
        );
        for case in cases {
            let folds: Vec<FullVector> = case
                .folds
                .iter()
                .map(|fold| full_vector(fold.sev, fold.grad).unwrap())
                .collect();
            let mean = ensemble_mean(&folds).unwrap();
            for (rust, python) in mean.iter().zip(case.mean_full) {
                assert!((rust - python).abs() < 2e-6);
            }
            assert_eq!(decode_full(case.mean_full).unwrap(), case.grade);
            let mut reversed = folds.clone();
            reversed.reverse();
            let reversed_mean = ensemble_mean(&reversed).unwrap();
            for (a, b) in mean.iter().zip(reversed_mean) {
                assert!((a - b).abs() < 1e-6, "fold order must not matter");
            }
        }
    }

    #[test]
    fn full_vector_is_a_distribution_and_rejects_nonfinite_logits() {
        let full = full_vector([0.3, -1.0, 2.0], 0.5).unwrap();
        assert!((full.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!(full.iter().all(|value| *value >= 0.0));
        assert_eq!(
            full_vector([f32::NAN, 0.0, 0.0], 0.0),
            Err(Stage3GradingError::NonFinite)
        );
        assert_eq!(
            full_vector([0.0, 0.0, 0.0], f32::INFINITY),
            Err(Stage3GradingError::NonFinite)
        );
        assert_eq!(ensemble_mean(&[]), Err(Stage3GradingError::EmptyEnsemble));
    }

    #[test]
    fn single_fold_grade_needs_exactly_four_finite_logits() {
        assert!(matches!(
            grade_single_fold(&[0.0, 0.0, 0.0]),
            Err(Stage3InferenceError::BadOutput(3))
        ));
        assert!(matches!(
            grade_single_fold(&[0.0, 0.0, f32::NAN, 0.0]),
            Err(Stage3InferenceError::Grading(Stage3GradingError::NonFinite))
        ));
        // Strongly negative gradability logit: ungradable overrides severity.
        let grade = grade_single_fold(&[9.0, 9.0, 9.0, -9.0]).unwrap().grade;
        assert_eq!(grade, 5);
        let grade = grade_single_fold(&[9.0, 9.0, 9.0, 9.0]).unwrap().grade;
        assert_eq!(grade, 4);
    }

    #[test]
    fn consecutive_decode_stops_at_first_failed_threshold_and_ungradable_wins() {
        let t = CORN_THRESHOLDS;
        // p1 passes, p2 fails, p3 would pass: consecutive rule stops, giving grade 2.
        assert_eq!(decode_consecutive([0.9, 0.5, 0.9, 0.9], t, 0.10), 2);
        // All pass: grade 4.
        assert_eq!(decode_consecutive([0.9, 0.9, 0.9, 0.9], t, 0.10), 4);
        // First fails: grade 1 even if later ones are high.
        assert_eq!(decode_consecutive([0.1, 0.9, 0.9, 0.9], t, 0.10), 1);
        // Strict comparison at the boundary.
        assert_eq!(decode_consecutive([0.38, 0.9, 0.9, 0.9], t, 0.10), 1);
        // Ungradable overrides severity, and exactly tg is still gradable (strictly below).
        assert_eq!(decode_consecutive([0.9, 0.9, 0.9, 0.0999], t, 0.10), 5);
        assert_eq!(decode_consecutive([0.9, 0.9, 0.9, 0.10], t, 0.10), 4);
    }
}
