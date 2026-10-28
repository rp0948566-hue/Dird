//! Stage I binary screening-gate score decoding; this is not DR grading.
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Exact v1.0 threshold from the upstream `convnextv2_768` config.
pub const STAGE1_GATE_THRESHOLD: f32 = 0.239_171_6;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage1GateDecision {
    /// The Stage I gate routes this sample to grade 0.
    NegativeRoute,
    /// The cascade must continue for grades 1–5 and ungradable samples.
    ContinueCascade,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecodedStage1Gate {
    pub raw_logit: f32,
    /// Sigmoid score; not a calibrated clinical confidence.
    pub screening_score: f32,
    pub threshold: f32,
    pub decision: Stage1GateDecision,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Stage1DecodeError {
    #[error("Stage I model must return one logit; found {0} values")]
    OutputCount(usize),
    #[error("Stage I model returned a non-finite logit")]
    NonFiniteLogit,
    #[error("screening probability must be finite and in [0, 1]")]
    InvalidProbability,
}

pub fn decode_stage1_gate(logits: &[f32]) -> Result<DecodedStage1Gate, Stage1DecodeError> {
    if logits.len() != 1 {
        return Err(Stage1DecodeError::OutputCount(logits.len()));
    }
    let raw_logit = logits[0];
    if !raw_logit.is_finite() {
        return Err(Stage1DecodeError::NonFiniteLogit);
    }
    let screening_score = if raw_logit >= 0.0 {
        1.0 / (1.0 + (-raw_logit).exp())
    } else {
        let exp = raw_logit.exp();
        exp / (1.0 + exp)
    };
    Ok(DecodedStage1Gate {
        raw_logit,
        screening_score,
        threshold: STAGE1_GATE_THRESHOLD,
        decision: route_for_probability(screening_score)?,
    })
}

fn route_for_probability(probability: f32) -> Result<Stage1GateDecision, Stage1DecodeError> {
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(Stage1DecodeError::InvalidProbability);
    }
    Ok(if probability >= STAGE1_GATE_THRESHOLD {
        Stage1GateDecision::ContinueCascade
    } else {
        Stage1GateDecision::NegativeRoute
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_output_with_wrong_element_count() {
        assert!(matches!(
            decode_stage1_gate(&[]),
            Err(Stage1DecodeError::OutputCount(0))
        ));
        assert!(matches!(
            decode_stage1_gate(&[0.0, 1.0]),
            Err(Stage1DecodeError::OutputCount(2))
        ));
    }

    #[test]
    fn rejects_non_finite_logit() {
        assert!(matches!(
            decode_stage1_gate(&[f32::NAN]),
            Err(Stage1DecodeError::NonFiniteLogit)
        ));
        assert!(matches!(
            decode_stage1_gate(&[f32::INFINITY]),
            Err(Stage1DecodeError::NonFiniteLogit)
        ));
    }

    #[test]
    fn sigmoid_is_stable_at_large_positive_and_negative_logits() {
        let positive = decode_stage1_gate(&[1000.0]).unwrap();
        let negative = decode_stage1_gate(&[-1000.0]).unwrap();
        assert_eq!(positive.screening_score, 1.0);
        assert_eq!(positive.decision, Stage1GateDecision::ContinueCascade);
        assert_eq!(negative.screening_score, 0.0);
        assert_eq!(negative.decision, Stage1GateDecision::NegativeRoute);
    }

    #[test]
    fn threshold_equality_routes_positive_and_below_routes_negative() {
        let at_threshold = route_for_probability(STAGE1_GATE_THRESHOLD).unwrap();
        let below = route_for_probability(STAGE1_GATE_THRESHOLD - f32::EPSILON).unwrap();
        assert_eq!(at_threshold, Stage1GateDecision::ContinueCascade);
        assert_eq!(below, Stage1GateDecision::NegativeRoute);
    }

    #[test]
    fn rejects_invalid_probabilities() {
        for probability in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
            assert!(matches!(
                route_for_probability(probability),
                Err(Stage1DecodeError::InvalidProbability)
            ));
        }
    }

    #[test]
    fn decoded_score_serializes_as_a_typed_gate_result() {
        let result = decode_stage1_gate(&[0.0]).unwrap();
        let json = serde_json::to_value(result).unwrap();
        assert_eq!(json["raw_logit"], 0.0);
        assert_eq!(json["screening_score"], 0.5);
        assert_eq!(json["threshold"], STAGE1_GATE_THRESHOLD);
        assert_eq!(json["decision"], "continue_cascade");
    }
}
