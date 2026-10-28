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
    #[error("a five-fold ensemble needs exactly five folds")]
    WrongFoldCount,
    #[error("ensemble folds must be exactly folds 0, 1, 2, 3 and 4, each once")]
    FoldSetMismatch,
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

/// Full five-fold LG-DRG result: the mean of the five full vectors, decoded after averaging.
#[derive(Debug, Clone, PartialEq)]
pub struct EnsembleGrade {
    /// Each fold's own decoded grade, kept as diagnostic evidence only (never a vote).
    pub per_fold: [SingleFoldGrade; 5],
    pub mean_full: FullVector,
    /// `(p1, p2, p3, p_gradable)` recovered from the averaged vector.
    pub probabilities: [f64; 4],
    pub grade: u8,
}

/// Official ensemble: five distinct folds (0-4, each exactly once), average the 5-way probability
/// vectors, then invert and decode the mean with the pinned thresholds. Any missing, duplicated,
/// mislabelled or non-finite fold rejects the whole ensemble; there is no partial grade.
pub fn grade_five_fold_ensemble(
    folds: &[(u8, [f32; 4])],
) -> Result<EnsembleGrade, Stage3InferenceError> {
    if folds.len() != 5 {
        return Err(Stage3GradingError::WrongFoldCount.into());
    }
    let mut seen = [false; 5];
    for (fold, _) in folds {
        match seen.get_mut(usize::from(*fold)) {
            Some(slot) if !*slot => *slot = true,
            _ => return Err(Stage3GradingError::FoldSetMismatch.into()),
        }
    }
    let mut ordered: Vec<(u8, SingleFoldGrade)> = Vec::with_capacity(5);
    for (fold, logits) in folds {
        ordered.push((*fold, grade_single_fold(logits)?));
    }
    ordered.sort_by_key(|(fold, _)| *fold);
    let vectors: Vec<FullVector> = ordered.iter().map(|(_, g)| g.full).collect();
    let mean_full = ensemble_mean(&vectors)?;
    let probabilities = corn_from_full(mean_full)?;
    let grade = decode_consecutive(probabilities, CORN_THRESHOLDS, GRADABILITY_THRESHOLD);
    let per_fold: [SingleFoldGrade; 5] = ordered
        .into_iter()
        .map(|(_, g)| g)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| Stage3GradingError::WrongFoldCount)?;
    Ok(EnsembleGrade {
        per_fold,
        mean_full,
        probabilities,
        grade,
    })
}

/// Combine five single-fold results (folds 0-4, each once, same examination, image, provider and
/// pinned checkpoint hashes) into the official full-ensemble result. Anything else is rejected:
/// the caller must never present a partial or mixed-up ensemble as a five-fold grade.
pub fn combine_five_folds(
    results: &[Stage3GradeResult],
) -> Result<Stage3GradeResult, Stage3InferenceError> {
    let reject = |reason: &'static str| Stage3InferenceError::InvalidRequest(reason);
    let first = results.first().ok_or(Stage3GradingError::WrongFoldCount)?;
    let mut folds = Vec::with_capacity(5);
    for result in results {
        let Stage3Scope::SingleFoldTechnicalTest { fold } = result.scope else {
            return Err(reject("ensemble input must be single-fold results"));
        };
        if result.examination_id != first.examination_id
            || result.source_image_sha256 != first.source_image_sha256
            || result.provider != first.provider
            || !result.research_only
        {
            return Err(reject(
                "fold results disagree on examination, image or provider",
            ));
        }
        if result.checkpoint_sha256.len() != 1
            || result.model_sha256.len() != 1
            || FOLD_CHECKPOINT_SHA256
                .get(usize::from(fold))
                .is_none_or(|expected| result.checkpoint_sha256[0] != *expected)
        {
            return Err(reject(
                "fold result does not carry its pinned checkpoint hash",
            ));
        }
        let (Some(severity), Some(gradability)) =
            (result.severity_logits, result.gradability_logit)
        else {
            return Err(reject("fold result is missing its raw logits"));
        };
        folds.push((fold, [severity[0], severity[1], severity[2], gradability]));
    }
    let ensemble = grade_five_fold_ensemble(&folds)?;
    let mut ordered: Vec<&Stage3GradeResult> = results.iter().collect();
    ordered.sort_by_key(|r| match r.scope {
        Stage3Scope::SingleFoldTechnicalTest { fold } => fold,
        Stage3Scope::FiveFoldEnsemble => u8::MAX,
    });
    Ok(Stage3GradeResult {
        examination_id: first.examination_id,
        source_image_sha256: first.source_image_sha256.clone(),
        scope: Stage3Scope::FiveFoldEnsemble,
        model_version: first.model_version.clone(),
        checkpoint_sha256: ordered
            .iter()
            .map(|r| r.checkpoint_sha256[0].clone())
            .collect(),
        model_sha256: ordered.iter().map(|r| r.model_sha256[0].clone()).collect(),
        severity_logits: None,
        gradability_logit: None,
        full_probabilities: ensemble.mean_full,
        ordinal_probabilities: [
            ensemble.probabilities[0],
            ensemble.probabilities[1],
            ensemble.probabilities[2],
        ],
        gradable_probability: ensemble.probabilities[3],
        corn_thresholds: CORN_THRESHOLDS,
        gradability_threshold: GRADABILITY_THRESHOLD,
        grade: ensemble.grade,
        ungradable: ensemble.grade == 5,
        provider: first.provider,
        elapsed_microseconds: results.iter().map(|r| r.elapsed_microseconds).sum(),
        research_only: true,
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

    fn five(logits: [[f32; 4]; 5]) -> Vec<(u8, [f32; 4])> {
        logits
            .iter()
            .enumerate()
            .map(|(i, l)| (i as u8, *l))
            .collect()
    }

    #[test]
    fn ensemble_decodes_the_mean_of_probabilities_not_a_vote_or_a_logit_mean() {
        // Four folds say severity 1, one fold is extremely severe: the decoded mean is what counts.
        let mut logits = [[-6.0, -6.0, -6.0, 6.0]; 5];
        logits[4] = [9.0, 9.0, 9.0, 6.0];
        let result = grade_five_fold_ensemble(&five(logits)).unwrap();
        let vectors: Vec<FullVector> = result.per_fold.iter().map(|g| g.full).collect();
        assert_eq!(result.mean_full, ensemble_mean(&vectors).unwrap());
        assert_eq!(result.grade, decode_full(result.mean_full).unwrap());
        let votes: Vec<u8> = result.per_fold.iter().map(|g| g.grade).collect();
        assert_eq!(
            votes.iter().filter(|g| **g == 1).count(),
            4,
            "folds disagree on purpose"
        );
    }

    #[test]
    fn ensemble_is_independent_of_the_order_the_folds_arrive_in() {
        let logits = [
            [1.0, 0.5, -1.0, 3.0],
            [0.2, 1.5, -0.3, 2.0],
            [-0.5, 0.1, 0.4, 4.0],
            [2.0, -1.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 0.5],
        ];
        let forward = grade_five_fold_ensemble(&five(logits)).unwrap();
        let mut shuffled = five(logits);
        shuffled.reverse();
        let reversed = grade_five_fold_ensemble(&shuffled).unwrap();
        assert_eq!(forward.grade, reversed.grade);
        assert_eq!(forward.mean_full, reversed.mean_full);
    }

    #[test]
    fn partial_duplicated_or_mislabelled_ensembles_are_rejected() {
        let good = [[1.0f32, 1.0, -1.0, 2.0]; 5];
        let four = &five(good)[..4];
        assert!(matches!(
            grade_five_fold_ensemble(four),
            Err(Stage3InferenceError::Grading(
                Stage3GradingError::WrongFoldCount
            ))
        ));
        let mut duplicate = five(good);
        duplicate[2].0 = 1; // fold 1 used twice, fold 2 missing
        assert!(matches!(
            grade_five_fold_ensemble(&duplicate),
            Err(Stage3InferenceError::Grading(
                Stage3GradingError::FoldSetMismatch
            ))
        ));
        let mut out_of_range = five(good);
        out_of_range[4].0 = 7;
        assert!(grade_five_fold_ensemble(&out_of_range).is_err());
    }

    #[test]
    fn one_nonfinite_fold_rejects_the_whole_ensemble() {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut logits = [[1.0f32, 1.0, -1.0, 2.0]; 5];
            logits[3][1] = bad;
            assert!(grade_five_fold_ensemble(&five(logits)).is_err());
        }
    }

    #[test]
    fn official_strict_comparisons_at_the_exact_thresholds() {
        // p_gradable < 0.10 is ungradable; exactly 0.10 is gradable. p_k > t is strict.
        assert_eq!(
            decode_consecutive([0.9, 0.9, 0.9, 0.0999], CORN_THRESHOLDS, 0.10),
            5
        );
        assert_eq!(
            decode_consecutive([0.9, 0.9, 0.9, 0.10], CORN_THRESHOLDS, 0.10),
            4
        );
        assert_eq!(
            decode_consecutive([0.38, 0.9, 0.9, 1.0], CORN_THRESHOLDS, 0.10),
            1
        );
        assert_eq!(
            decode_consecutive([0.3801, 0.60, 0.9, 1.0], CORN_THRESHOLDS, 0.10),
            2
        );
        assert_eq!(
            decode_consecutive([0.9, 0.6001, 0.19, 1.0], CORN_THRESHOLDS, 0.10),
            3
        );
        assert_eq!(
            decode_consecutive([0.9, 0.9, 0.1901, 1.0], CORN_THRESHOLDS, 0.10),
            4
        );
    }

    #[test]
    fn ungradable_ensemble_overrides_severity() {
        let logits = [[9.0f32, 9.0, 9.0, -10.0]; 5];
        assert_eq!(grade_five_fold_ensemble(&five(logits)).unwrap().grade, 5);
    }

    fn fold_result(fold: u8, logits: [f32; 4]) -> Stage3GradeResult {
        let decoded = grade_single_fold(&logits).unwrap();
        Stage3GradeResult {
            examination_id: 7,
            source_image_sha256: "a".repeat(64),
            scope: Stage3Scope::SingleFoldTechnicalTest { fold },
            model_version: "1.0.0".to_string(),
            checkpoint_sha256: vec![FOLD_CHECKPOINT_SHA256[usize::from(fold)].to_string()],
            model_sha256: vec![format!("{:064x}", fold)],
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
            provider: ActualProvider::Cpu,
            elapsed_microseconds: 1_000,
            research_only: true,
        }
    }

    fn five_results() -> Vec<Stage3GradeResult> {
        let logits = [
            [1.0, 0.5, -1.0, 3.0],
            [0.2, 1.5, -0.3, 2.0],
            [-0.5, 0.1, 0.4, 4.0],
            [2.0, -1.0, 0.0, 1.0],
            [0.0, 0.0, 0.0, 0.5],
        ];
        logits
            .iter()
            .enumerate()
            .map(|(i, l)| fold_result(i as u8, *l))
            .collect()
    }

    #[test]
    fn combining_five_fold_results_matches_the_logit_level_ensemble_and_keeps_all_hashes() {
        let results = five_results();
        let combined = combine_five_folds(&results).unwrap();
        let direct = grade_five_fold_ensemble(
            &results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let s = r.severity_logits.unwrap();
                    (i as u8, [s[0], s[1], s[2], r.gradability_logit.unwrap()])
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(combined.scope, Stage3Scope::FiveFoldEnsemble);
        assert_eq!(combined.grade, direct.grade);
        assert_eq!(combined.full_probabilities, direct.mean_full);
        assert_eq!(
            combined.checkpoint_sha256,
            FOLD_CHECKPOINT_SHA256.map(str::to_string).to_vec()
        );
        assert_eq!(combined.model_sha256.len(), 5);
        assert!(combined.research_only && combined.severity_logits.is_none());
        assert_eq!(combined.elapsed_microseconds, 5_000);
    }

    #[test]
    fn combining_rejects_a_substituted_checkpoint_missing_fold_or_mixed_exam() {
        let mut wrong_hash = five_results();
        wrong_hash[2].checkpoint_sha256 = wrong_hash[1].checkpoint_sha256.clone();
        assert!(
            combine_five_folds(&wrong_hash).is_err(),
            "fold 1 file passed off as fold 2"
        );
        let mut four = five_results();
        four.pop();
        assert!(combine_five_folds(&four).is_err());
        let mut mixed = five_results();
        mixed[3].examination_id = 8;
        assert!(combine_five_folds(&mixed).is_err());
        let mut other_image = five_results();
        other_image[4].source_image_sha256 = "b".repeat(64);
        assert!(combine_five_folds(&other_image).is_err());
        let mut already_ensemble = five_results();
        already_ensemble[0].scope = Stage3Scope::FiveFoldEnsemble;
        assert!(combine_five_folds(&already_ensemble).is_err());
    }
}
