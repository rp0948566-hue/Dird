#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_metadata_matches_the_pinned_upstream_contracts() {
        assert_eq!(Stage2ModelKind::Ma.input_size(), 1024);
        assert_eq!(Stage2ModelKind::Ma.threshold(), 0.01);
        assert_eq!(
            Stage2ModelKind::Ma.threshold_operator(),
            ThresholdOperator::GreaterThan
        );
        assert!(!Stage2ModelKind::Ma.uses_flip_tta());
        assert_eq!(Stage2ModelKind::He.threshold(), 0.20);
        assert_eq!(Stage2ModelKind::Ex.threshold(), 0.325);
        assert!(Stage2ModelKind::Ex.uses_flip_tta());
        assert_eq!(Stage2ModelKind::Cws.threshold(), 0.40);
        assert!(Stage2ModelKind::Cws.uses_flip_tta());
        assert_eq!(Stage2ModelKind::Vessel.input_size(), 512);
        assert_eq!(Stage2ModelKind::Vessel.threshold(), 0.46);
        assert_eq!(
            Stage2ModelKind::Vessel.threshold_operator(),
            ThresholdOperator::GreaterThanOrEqual
        );
        assert!(Stage2ModelKind::Vessel.uses_flip_tta());
    }

    #[test]
    fn mask_result_rejects_shape_value_and_model_transform_mismatches() {
        let mut result = valid_result();
        assert!(result.validate().is_ok());

        result.data = MaskPixelData::Probability(vec![0.25]);
        assert!(matches!(
            result.validate(),
            Err(Stage2ContractError::PixelCount { .. })
        ));

        let mut result = valid_result();
        result.data = MaskPixelData::Probability(vec![f32::NAN; 4]);
        assert_eq!(
            result.validate(),
            Err(Stage2ContractError::NonFiniteProbability)
        );

        let mut result = valid_result();
        result.model_kind = Stage2ModelKind::Ex;
        result.threshold = ThresholdMetadata {
            value: Stage2ModelKind::Ex.threshold(),
            operator: Stage2ModelKind::Ex.threshold_operator(),
        };
        assert_eq!(
            result.validate(),
            Err(Stage2ContractError::TransformModelMismatch)
        );

        let mut result = valid_result();
        result.data = MaskPixelData::Binary(vec![0, 1, 255, 0]);
        assert_eq!(
            result.validate(),
            Err(Stage2ContractError::InvalidBinaryPixel)
        );

        let mut result = valid_result();
        result.error = Some("must not accompany complete mask".into());
        assert_eq!(
            result.validate(),
            Err(Stage2ContractError::InvalidProcessingState)
        );

        assert!(!ThresholdOperator::GreaterThan.applies(0.01, 0.01));
        assert!(ThresholdOperator::GreaterThanOrEqual.applies(0.46, 0.46));

        let mut result = valid_result();
        result.transform = Stage2SpatialTransform::LesionRectLetterbox {
            crop: ImageRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            model_size: 1024,
            resized_width: 1024,
            resized_height: 1024,
            offset_x: 1,
            offset_y: 0,
        };
        assert_eq!(
            result.validate(),
            Err(Stage2ContractError::InvalidTransform)
        );
    }

    fn valid_result() -> Stage2MaskResult {
        Stage2MaskResult {
            processing_status: Stage2ProcessingStatus::Complete,
            error: None,
            examination_id: 7,
            source_image_sha256: "a".repeat(64),
            laterality: Some(EyeLaterality::Right),
            model_kind: Stage2ModelKind::Ma,
            model_version: "v1.0".into(),
            checkpoint_sha256: "b".repeat(64),
            model_sha256: "c".repeat(64),
            original_width: 2,
            original_height: 2,
            mask_width: 2,
            mask_height: 2,
            transform: Stage2SpatialTransform::LesionCircleStretch {
                crop: ImageRect {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                model_size: 1024,
            },
            preprocessing_version: "upstream-v1.0".into(),
            threshold: ThresholdMetadata {
                value: 0.01,
                operator: ThresholdOperator::GreaterThan,
            },
            provider: crate::device::ActualProvider::Cpu,
            elapsed_microseconds: 1,
            data: MaskPixelData::Probability(vec![0.1; 4]),
        }
    }
}
use crate::{device::ActualProvider, model_runtime::ModelRuntimeError};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage2ModelKind {
    Ma,
    He,
    Ex,
    Cws,
    Vessel,
}

impl Stage2ModelKind {
    pub const fn input_size(self) -> u32 {
        match self {
            Self::Vessel => 512,
            _ => 1024,
        }
    }

    pub const fn threshold(self) -> f32 {
        match self {
            Self::Ma => 0.01,
            Self::He => 0.20,
            Self::Ex => 0.325,
            Self::Cws => 0.40,
            Self::Vessel => 0.46,
        }
    }

    pub const fn threshold_operator(self) -> ThresholdOperator {
        match self {
            Self::Vessel => ThresholdOperator::GreaterThanOrEqual,
            _ => ThresholdOperator::GreaterThan,
        }
    }

    pub const fn uses_flip_tta(self) -> bool {
        matches!(self, Self::Ex | Self::Cws | Self::Vessel)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EyeLaterality {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stage2Request {
    pub examination_id: i64,
    pub image_path: PathBuf,
    pub laterality: Option<EyeLaterality>,
    pub model_kind: Stage2ModelKind,
    pub model_id: String,
}

impl Stage2Request {
    pub fn validate(&self) -> Result<(), Stage2RequestError> {
        if self.examination_id <= 0 {
            return Err(Stage2RequestError::InvalidExaminationId);
        }
        if self.model_id.trim().is_empty() {
            return Err(Stage2RequestError::MissingModelId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdOperator {
    GreaterThan,
    GreaterThanOrEqual,
}

impl ThresholdOperator {
    pub fn applies(self, probability: f32, threshold: f32) -> bool {
        match self {
            Self::GreaterThan => probability > threshold,
            Self::GreaterThanOrEqual => probability >= threshold,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ThresholdMetadata {
    pub value: f32,
    pub operator: ThresholdOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Stage2SpatialTransform {
    LesionCircleStretch {
        crop: ImageRect,
        model_size: u32,
    },
    LesionRectLetterbox {
        crop: ImageRect,
        model_size: u32,
        resized_width: u32,
        resized_height: u32,
        offset_x: u32,
        offset_y: u32,
    },
    VesselFovSquare {
        crop: ImageRect,
        model_size: u32,
        fov_policy: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "encoding", content = "pixels", rename_all = "snake_case")]
pub enum MaskPixelData {
    Probability(Vec<f32>),
    Binary(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage2ProcessingStatus {
    Complete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stage2MaskResult {
    pub processing_status: Stage2ProcessingStatus,
    pub error: Option<String>,
    pub examination_id: i64,
    pub source_image_sha256: String,
    pub laterality: Option<EyeLaterality>,
    pub model_kind: Stage2ModelKind,
    pub model_version: String,
    pub checkpoint_sha256: String,
    pub model_sha256: String,
    pub original_width: u32,
    pub original_height: u32,
    pub mask_width: u32,
    pub mask_height: u32,
    pub transform: Stage2SpatialTransform,
    pub preprocessing_version: String,
    pub threshold: ThresholdMetadata,
    pub provider: ActualProvider,
    pub elapsed_microseconds: u128,
    pub data: MaskPixelData,
}

impl Stage2MaskResult {
    pub fn validate(&self) -> Result<(), Stage2ContractError> {
        if self.processing_status != Stage2ProcessingStatus::Complete || self.error.is_some() {
            return Err(Stage2ContractError::InvalidProcessingState);
        }
        if self.examination_id <= 0
            || self.model_version.trim().is_empty()
            || self.preprocessing_version.trim().is_empty()
        {
            return Err(Stage2ContractError::MissingIdentity);
        }
        for digest in [
            &self.source_image_sha256,
            &self.checkpoint_sha256,
            &self.model_sha256,
        ] {
            if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(Stage2ContractError::InvalidDigest);
            }
        }
        if self.original_width == 0
            || self.original_height == 0
            || self.mask_width != self.original_width
            || self.mask_height != self.original_height
        {
            return Err(Stage2ContractError::InvalidDimensions);
        }
        if !self.threshold.value.is_finite()
            || self.threshold.value != self.model_kind.threshold()
            || self.threshold.operator != self.model_kind.threshold_operator()
        {
            return Err(Stage2ContractError::InvalidThresholdMetadata);
        }
        self.validate_transform()?;

        let expected = (self.mask_width as usize)
            .checked_mul(self.mask_height as usize)
            .ok_or(Stage2ContractError::InvalidDimensions)?;
        let actual = match &self.data {
            MaskPixelData::Probability(pixels) => {
                for pixel in pixels {
                    if !pixel.is_finite() {
                        return Err(Stage2ContractError::NonFiniteProbability);
                    }
                    if !(0.0..=1.0).contains(pixel) {
                        return Err(Stage2ContractError::ProbabilityOutOfRange);
                    }
                }
                pixels.len()
            }
            MaskPixelData::Binary(pixels) => {
                if pixels.iter().any(|pixel| !matches!(*pixel, 0 | 255)) {
                    return Err(Stage2ContractError::InvalidBinaryPixel);
                }
                pixels.len()
            }
        };
        if actual != expected {
            return Err(Stage2ContractError::PixelCount { expected, actual });
        }
        Ok(())
    }

    fn validate_transform(&self) -> Result<(), Stage2ContractError> {
        let crop = match &self.transform {
            Stage2SpatialTransform::LesionCircleStretch { crop, model_size } => {
                if !matches!(self.model_kind, Stage2ModelKind::Ma | Stage2ModelKind::He)
                    || *model_size != self.model_kind.input_size()
                {
                    return Err(Stage2ContractError::TransformModelMismatch);
                }
                crop
            }
            Stage2SpatialTransform::LesionRectLetterbox {
                crop,
                model_size,
                resized_width,
                resized_height,
                offset_x,
                offset_y,
            } => {
                let end_x = offset_x.checked_add(*resized_width);
                let end_y = offset_y.checked_add(*resized_height);
                if *resized_width == 0
                    || *resized_height == 0
                    || end_x.is_none_or(|end| end > *model_size)
                    || end_y.is_none_or(|end| end > *model_size)
                {
                    return Err(Stage2ContractError::InvalidTransform);
                }
                if !matches!(self.model_kind, Stage2ModelKind::Ex | Stage2ModelKind::Cws)
                    || *model_size != self.model_kind.input_size()
                {
                    return Err(Stage2ContractError::TransformModelMismatch);
                }
                crop
            }
            Stage2SpatialTransform::VesselFovSquare {
                crop,
                model_size,
                fov_policy,
            } => {
                if fov_policy != "hierarchiretina-v1.0-vessel-fov" {
                    return Err(Stage2ContractError::InvalidTransform);
                }
                if self.model_kind != Stage2ModelKind::Vessel
                    || *model_size != self.model_kind.input_size()
                {
                    return Err(Stage2ContractError::TransformModelMismatch);
                }
                crop
            }
        };
        let right = crop.x.checked_add(crop.width);
        let bottom = crop.y.checked_add(crop.height);
        if crop.width == 0
            || crop.height == 0
            || right.is_none_or(|end| end > self.original_width)
            || bottom.is_none_or(|end| end > self.original_height)
        {
            return Err(Stage2ContractError::InvalidTransform);
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Stage2RequestError {
    #[error("Stage II request requires a persisted examination ID")]
    InvalidExaminationId,
    #[error("Stage II request requires a model ID")]
    MissingModelId,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Stage2ContractError {
    #[error("successful Stage II mask must have complete status and no error")]
    InvalidProcessingState,
    #[error("Stage II mask result is missing required identity metadata")]
    MissingIdentity,
    #[error("Stage II result contains an invalid SHA-256 value")]
    InvalidDigest,
    #[error("Stage II mask dimensions must be nonzero and match original frame dimensions")]
    InvalidDimensions,
    #[error("threshold does not match the pinned model operating point")]
    InvalidThresholdMetadata,
    #[error("transform does not match the model kind")]
    TransformModelMismatch,
    #[error("spatial transform is invalid or out of source bounds")]
    InvalidTransform,
    #[error("mask pixel count mismatch: expected {expected}, got {actual}")]
    PixelCount { expected: usize, actual: usize },
    #[error("probability map contains a non-finite value")]
    NonFiniteProbability,
    #[error("probability map values must be between 0 and 1")]
    ProbabilityOutOfRange,
    #[error("binary mask pixels must be 0 or 255")]
    InvalidBinaryPixel,
}

#[derive(Debug, Error)]
pub enum Stage2InferenceError {
    #[error(transparent)]
    Request(#[from] Stage2RequestError),
    #[error(transparent)]
    Runtime(#[from] ModelRuntimeError),
    #[error(transparent)]
    Session(#[from] crate::onnx_session::SessionError),
    #[error(transparent)]
    Contract(#[from] Stage2ContractError),
    #[error("loaded ONNX model does not match Stage II model kind `{0:?}`")]
    IncompatibleModel(Stage2ModelKind),
    #[error("Stage II preprocessing failed: {0}")]
    Preprocessing(#[from] super::stage2_preprocessing::Stage2PreprocessError),
    #[error("Stage II output decoding failed: {0}")]
    Decode(String),
}
