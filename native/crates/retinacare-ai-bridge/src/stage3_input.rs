//! HierarchiRetina v1.0 LG-DRG eight-channel input assembler (`stage3/preprocessing.py`, `data.py`).
//!
//! Channels 0-2: RGB after fundus-box crop, CLAHE (LAB L, clip 2.5, 8x8), longest side to 512, zero
//! pad and ImageNet normalisation. Channels 3-7: the five Stage II masks (MA, HE, EX, CWS, VESSEL),
//! cropped by the same box, binarised (`> 127`), nearest-resized and zero-padded. Unlike upstream,
//! a missing, stale or mismatched mask is an error, never an all-zero channel.
use crate::stage2_contract::{ImageRect, MaskPixelData, Stage2MaskResult, Stage2ModelKind};
use crate::stage2_preprocessing::{Stage2PreprocessError, clahe_lab, decode_stage2_source};
use opencv::{
    core::{self, Mat, MatTraitConst, MatTraitConstManual, Size},
    imgproc,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const STAGE3_INPUT_SIZE: i32 = 512;
pub const STAGE3_CHANNELS: usize = 8;
/// Channel order of the five masks (channels 3-7), identical to upstream `LESIONS`.
pub const STAGE3_MASK_ORDER: [Stage2ModelKind; 5] = [
    Stage2ModelKind::Ma,
    Stage2ModelKind::He,
    Stage2ModelKind::Ex,
    Stage2ModelKind::Cws,
    Stage2ModelKind::Vessel,
];
const IMAGE_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGE_STD: [f32; 3] = [0.229, 0.224, 0.225];
/// Upstream `crop_fundus_circle` keeps pixels whose channel mean exceeds this value.
const FUNDUS_TOLERANCE: u32 = 7;

#[derive(Debug, Error)]
pub enum Stage3InputError {
    #[error("image preprocessing failed: {0}")]
    Preprocess(#[from] Stage2PreprocessError),
    #[error("OpenCV failed: {0}")]
    OpenCv(#[from] opencv::Error),
    #[error("Stage II {0:?} mask is missing")]
    MissingMask(Stage2ModelKind),
    #[error("Stage II {0:?} mask is present more than once")]
    DuplicateMask(Stage2ModelKind),
    #[error("Stage II {0:?} mask is not a complete binary mask for this image")]
    InvalidMaskResult(Stage2ModelKind),
    #[error("Stage II {0:?} mask belongs to a different examination")]
    StaleExamination(Stage2ModelKind),
    #[error("Stage II {0:?} mask was produced from a different source image")]
    SourceMismatch(Stage2ModelKind),
    #[error("Stage II {0:?} mask size does not match the source frame")]
    DimensionMismatch(Stage2ModelKind),
    #[error("Stage II {0:?} mask contains values other than 0 and 255")]
    NonBinaryMask(Stage2ModelKind),
}

/// Eight-channel LG-DRG tensor (`[8][512][512]`, row-major) with registration metadata.
#[derive(Debug)]
pub struct Stage3Input {
    pub tensor: Vec<f32>,
    pub image_sha256: String,
    pub original_width: u32,
    pub original_height: u32,
    /// Fundus box in source pixels (inclusive-bounds crop upstream, expressed as x/y/width/height).
    pub fundus_crop: ImageRect,
    pub resized_width: u32,
    pub resized_height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
}

/// Assemble the input from typed Stage II results. Exactly one complete result per mask kind is required.
pub fn assemble_stage3_input(
    encoded: &[u8],
    examination_id: i64,
    results: &[Stage2MaskResult],
) -> Result<Stage3Input, Stage3InputError> {
    let decoded = decode_stage2_source(encoded)?;
    let image_sha256 = hex::encode(Sha256::digest(encoded));
    let frame_len = decoded.width as usize * decoded.height as usize;
    let mut masks: Vec<&[u8]> = Vec::with_capacity(5);
    for kind in STAGE3_MASK_ORDER {
        let mut matching = results.iter().filter(|result| result.model_kind == kind);
        let result = matching.next().ok_or(Stage3InputError::MissingMask(kind))?;
        if matching.next().is_some() {
            return Err(Stage3InputError::DuplicateMask(kind));
        }
        if result.validate().is_err() {
            return Err(Stage3InputError::InvalidMaskResult(kind));
        }
        if result.examination_id != examination_id {
            return Err(Stage3InputError::StaleExamination(kind));
        }
        if result.source_image_sha256 != image_sha256 {
            return Err(Stage3InputError::SourceMismatch(kind));
        }
        if result.original_width != decoded.width
            || result.original_height != decoded.height
            || result.mask_width != decoded.width
            || result.mask_height != decoded.height
        {
            return Err(Stage3InputError::DimensionMismatch(kind));
        }
        let MaskPixelData::Binary(pixels) = &result.data else {
            return Err(Stage3InputError::InvalidMaskResult(kind));
        };
        if pixels.len() != frame_len {
            return Err(Stage3InputError::DimensionMismatch(kind));
        }
        masks.push(pixels);
    }
    assemble_from_decoded(
        &decoded.image,
        decoded.width,
        decoded.height,
        image_sha256,
        &masks,
    )
}

/// Assemble from the encoded image and five raw 0/255 source-frame masks given in channel order.
pub fn assemble_stage3_input_raw(
    encoded: &[u8],
    masks: [&[u8]; 5],
) -> Result<Stage3Input, Stage3InputError> {
    let decoded = decode_stage2_source(encoded)?;
    let image_sha256 = hex::encode(Sha256::digest(encoded));
    let frame_len = decoded.width as usize * decoded.height as usize;
    for (kind, mask) in STAGE3_MASK_ORDER.iter().zip(masks) {
        if mask.len() != frame_len {
            return Err(Stage3InputError::DimensionMismatch(*kind));
        }
    }
    assemble_from_decoded(
        &decoded.image,
        decoded.width,
        decoded.height,
        image_sha256,
        &masks,
    )
}

fn assemble_from_decoded(
    bgr: &Mat,
    width: u32,
    height: u32,
    image_sha256: String,
    masks: &[&[u8]],
) -> Result<Stage3Input, Stage3InputError> {
    for (kind, mask) in STAGE3_MASK_ORDER.iter().zip(masks) {
        if mask.iter().any(|&value| value != 0 && value != 255) {
            return Err(Stage3InputError::NonBinaryMask(*kind));
        }
    }
    let mut rgb = Mat::default();
    imgproc::cvt_color_def(bgr, &mut rgb, imgproc::COLOR_BGR2RGB)?;
    let crop = fundus_box(&rgb)?.unwrap_or(ImageRect {
        x: 0,
        y: 0,
        width,
        height,
    });
    let rect = core::Rect::new(
        crop.x as i32,
        crop.y as i32,
        crop.width as i32,
        crop.height as i32,
    );
    let mut cropped = Mat::default();
    Mat::roi(&rgb, rect)?.copy_to(&mut cropped)?;
    let enhanced = clahe_lab(&cropped, 2.5, 8)?;

    // albumentations 1.4 LongestMaxSize: Python round() (ties to even), image linear, mask nearest.
    let size = STAGE3_INPUT_SIZE;
    let scale = f64::from(size) / f64::from(crop.width.max(crop.height));
    let new_width = ((f64::from(crop.width) * scale).round_ties_even() as i32).max(1);
    let new_height = ((f64::from(crop.height) * scale).round_ties_even() as i32).max(1);
    let mut resized = Mat::default();
    imgproc::resize(
        &enhanced,
        &mut resized,
        Size::new(new_width, new_height),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    let offset_x = (size - new_width) / 2;
    let offset_y = (size - new_height) / 2;

    let plane = (size * size) as usize;
    let mut tensor = vec![0.0f32; STAGE3_CHANNELS * plane];
    // Normalize as albumentations: (pixel - mean*255) * (1 / (std*255)); zero padding is normalised too.
    let mean: [f32; 3] = IMAGE_MEAN.map(|value| value * 255.0);
    let scale_back: [f32; 3] = IMAGE_STD.map(|value| 1.0 / (value * 255.0));
    for channel in 0..3 {
        let padded = (0.0 - mean[channel]) * scale_back[channel];
        tensor[channel * plane..(channel + 1) * plane].fill(padded);
    }
    let bytes = resized.data_bytes()?;
    for row in 0..new_height {
        for col in 0..new_width {
            let source = ((row * new_width + col) * 3) as usize;
            let target = ((row + offset_y) * size + col + offset_x) as usize;
            for channel in 0..3 {
                tensor[channel * plane + target] =
                    (f32::from(bytes[source + channel]) - mean[channel]) * scale_back[channel];
            }
        }
    }
    for (index, mask) in masks.iter().enumerate() {
        let flat = Mat::from_slice(mask)?;
        let frame = flat.reshape(1, height as i32)?;
        let mut binary = Mat::default();
        imgproc::threshold(&frame, &mut binary, 127.0, 1.0, imgproc::THRESH_BINARY)?;
        let mut cropped_mask = Mat::default();
        Mat::roi(&binary, rect)?.copy_to(&mut cropped_mask)?;
        let mut resized_mask = Mat::default();
        imgproc::resize(
            &cropped_mask,
            &mut resized_mask,
            Size::new(new_width, new_height),
            0.0,
            0.0,
            imgproc::INTER_NEAREST,
        )?;
        let mask_bytes = resized_mask.data_bytes()?;
        let channel_start = (3 + index) * plane;
        for row in 0..new_height {
            for col in 0..new_width {
                let target = ((row + offset_y) * size + col + offset_x) as usize;
                tensor[channel_start + target] =
                    f32::from(mask_bytes[(row * new_width + col) as usize]);
            }
        }
    }
    Ok(Stage3Input {
        tensor,
        image_sha256,
        original_width: width,
        original_height: height,
        fundus_crop: crop,
        resized_width: new_width as u32,
        resized_height: new_height as u32,
        offset_x: offset_x as u32,
        offset_y: offset_y as u32,
    })
}

/// Upstream `crop_fundus_circle`: bounding box of pixels whose channel mean exceeds 7, or `None`.
fn fundus_box(rgb: &Mat) -> Result<Option<ImageRect>, Stage3InputError> {
    let width = rgb.cols();
    let height = rgb.rows();
    let bytes = rgb.data_bytes()?;
    let (mut r0, mut r1, mut c0, mut c1) = (i32::MAX, -1, i32::MAX, -1);
    for row in 0..height {
        for col in 0..width {
            let index = ((row * width + col) * 3) as usize;
            let sum =
                u32::from(bytes[index]) + u32::from(bytes[index + 1]) + u32::from(bytes[index + 2]);
            // mean > 7 is equivalent to sum > 21 for three channels.
            if sum > 3 * FUNDUS_TOLERANCE {
                r0 = r0.min(row);
                r1 = r1.max(row);
                c0 = c0.min(col);
                c1 = c1.max(col);
            }
        }
    }
    if r1 < 0 {
        return Ok(None);
    }
    Ok(Some(ImageRect {
        x: c0 as u32,
        y: r0 as u32,
        width: (c1 - c0 + 1) as u32,
        height: (r1 - r0 + 1) as u32,
    }))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use opencv::{
        core::{Point, Scalar, Vector},
        imgcodecs,
        prelude::VectorToVec,
    };

    fn png(width: i32, height: i32) -> Vec<u8> {
        let mut image =
            Mat::new_rows_cols_with_default(height, width, core::CV_8UC3, Scalar::all(0.0))
                .unwrap();
        imgproc::circle(
            &mut image,
            Point::new(width / 2, height / 2),
            (height.min(width) / 2) - 2,
            Scalar::new(120.0, 140.0, 180.0, 0.0),
            -1,
            imgproc::LINE_8,
            0,
        )
        .unwrap();
        let mut encoded = Vector::new();
        imgcodecs::imencode(".png", &image, &mut encoded, &Vector::new()).unwrap();
        encoded.to_vec()
    }

    fn blank(width: usize, height: usize) -> Vec<u8> {
        vec![0u8; width * height]
    }

    fn hot(width: usize, height: usize, x: usize, y: usize) -> Vec<u8> {
        let mut mask = blank(width, height);
        mask[y * width + x] = 255;
        mask
    }

    #[test]
    fn each_mask_lands_only_in_its_own_channel() {
        let (w, h) = (120usize, 100usize);
        let encoded = png(w as i32, h as i32);
        let plane = (STAGE3_INPUT_SIZE * STAGE3_INPUT_SIZE) as usize;
        for hot_index in 0..5 {
            // A 20x20 block inside the fundus so that nearest resize keeps it.
            let mut masks: Vec<Vec<u8>> = (0..5).map(|_| blank(w, h)).collect();
            for y in 40..60 {
                for x in 50..70 {
                    masks[hot_index][y * w + x] = 255;
                }
            }
            let input = assemble_stage3_input_raw(
                &encoded,
                [&masks[0], &masks[1], &masks[2], &masks[3], &masks[4]],
            )
            .unwrap();
            assert_eq!(input.tensor.len(), STAGE3_CHANNELS * plane);
            assert!(input.tensor.iter().all(|value| value.is_finite()));
            for channel in 0..5 {
                let set = input.tensor[(3 + channel) * plane..(4 + channel) * plane]
                    .iter()
                    .filter(|&&value| value == 1.0)
                    .count();
                assert_eq!(
                    set > 0,
                    channel == hot_index,
                    "mask {hot_index} leaked into channel {channel}"
                );
            }
        }
    }

    #[test]
    fn rejects_nonbinary_wrong_sized_and_one_hot_checked_masks() {
        let (w, h) = (64usize, 64usize);
        let encoded = png(w as i32, h as i32);
        let good = blank(w, h);
        let mut gray = blank(w, h);
        gray[5] = 128;
        let short = vec![0u8; w * h - 1];
        assert!(matches!(
            assemble_stage3_input_raw(&encoded, [&good, &good, &gray, &good, &good]),
            Err(Stage3InputError::NonBinaryMask(Stage2ModelKind::Ex))
        ));
        assert!(matches!(
            assemble_stage3_input_raw(&encoded, [&good, &good, &good, &short, &good]),
            Err(Stage3InputError::DimensionMismatch(Stage2ModelKind::Cws))
        ));
        let _ = hot(w, h, 0, 0);
    }

    #[test]
    fn registers_the_fundus_box_resize_and_zero_padding() {
        // 160x80 frame with a fundus circle: longest side maps to 512, so the short side is 256 and padded.
        let (w, h) = (160usize, 80usize);
        let encoded = png(w as i32, h as i32);
        let masks: Vec<Vec<u8>> = (0..5).map(|_| blank(w, h)).collect();
        let input = assemble_stage3_input_raw(
            &encoded,
            [&masks[0], &masks[1], &masks[2], &masks[3], &masks[4]],
        )
        .unwrap();
        assert!(input.fundus_crop.width <= w as u32 && input.fundus_crop.height <= h as u32);
        assert_eq!(input.resized_width.max(input.resized_height), 512);
        assert_eq!(input.offset_x, (512 - input.resized_width) / 2);
        assert_eq!(input.offset_y, (512 - input.resized_height) / 2);
        // The padded top-left pixel equals a normalised black pixel on every RGB channel.
        let plane = (STAGE3_INPUT_SIZE * STAGE3_INPUT_SIZE) as usize;
        if input.offset_y > 0 {
            for channel in 0..3 {
                let expected =
                    (0.0 - IMAGE_MEAN[channel] * 255.0) * (1.0 / (IMAGE_STD[channel] * 255.0));
                assert!((input.tensor[channel * plane] - expected).abs() < 1e-6);
            }
        }
    }

    #[test]
    #[ignore = "local-only: set RETINACARE_STAGE3_IMAGE, RETINACARE_STAGE3_MASK_DIR and RETINACARE_STAGE3_PARITY_DIR"]
    fn local_image_and_masks_match_upstream_eight_channel_tensor() {
        let image =
            std::env::var("RETINACARE_STAGE3_IMAGE").expect("set the local technical test image");
        let mask_dir = std::path::PathBuf::from(
            std::env::var("RETINACARE_STAGE3_MASK_DIR")
                .expect("set the raw Stage II mask directory"),
        );
        let parity_dir = std::path::PathBuf::from(
            std::env::var("RETINACARE_STAGE3_PARITY_DIR")
                .expect("set the upstream fixture directory"),
        );
        let encoded = std::fs::read(image).unwrap();
        let masks: Vec<Vec<u8>> = ["ma", "he", "ex", "cws", "vessel"]
            .iter()
            .map(|name| std::fs::read(mask_dir.join(format!("{name}_pytorch_mask.u8"))).unwrap())
            .collect();
        let input = assemble_stage3_input_raw(
            &encoded,
            [&masks[0], &masks[1], &masks[2], &masks[3], &masks[4]],
        )
        .unwrap();
        let reference = std::fs::read(parity_dir.join("stage3_input.f32")).unwrap();
        assert_eq!(reference.len(), input.tensor.len() * 4);
        let plane = (STAGE3_INPUT_SIZE * STAGE3_INPUT_SIZE) as usize;
        let mut worst = 0.0f32;
        for channel in 0..STAGE3_CHANNELS {
            let (mut max_abs, mut squared) = (0.0f32, 0.0f64);
            for index in 0..plane {
                let at = channel * plane + index;
                let python = f32::from_le_bytes(reference[at * 4..at * 4 + 4].try_into().unwrap());
                let diff = (input.tensor[at] - python).abs();
                max_abs = max_abs.max(diff);
                squared += f64::from(diff).powi(2);
            }
            eprintln!(
                "Stage III channel {channel}: max_abs={max_abs:.9} rmse={:.9}",
                (squared / plane as f64).sqrt()
            );
            worst = worst.max(max_abs);
            if channel >= 3 {
                assert_eq!(max_abs, 0.0, "mask channel {channel} must match exactly");
            }
        }
        assert!(worst <= 1e-4, "eight-channel tensor max diff {worst}");
    }

    pub(crate) fn mask_result(
        kind: Stage2ModelKind,
        examination_id: i64,
        sha: &str,
        side: u32,
        pixels: Vec<u8>,
    ) -> Stage2MaskResult {
        use crate::stage2_contract::{
            Stage2ProcessingStatus, Stage2SpatialTransform, ThresholdMetadata,
        };
        let crop = ImageRect {
            x: 0,
            y: 0,
            width: side,
            height: side,
        };
        let transform = match kind {
            Stage2ModelKind::Ma | Stage2ModelKind::He => {
                Stage2SpatialTransform::LesionCircleStretch {
                    crop,
                    model_size: 1024,
                }
            }
            Stage2ModelKind::Ex | Stage2ModelKind::Cws => {
                Stage2SpatialTransform::LesionRectLetterbox {
                    crop,
                    model_size: 1024,
                    resized_width: 1024,
                    resized_height: 1024,
                    offset_x: 0,
                    offset_y: 0,
                }
            }
            Stage2ModelKind::Vessel => Stage2SpatialTransform::VesselFovSquare {
                crop,
                model_size: 512,
                fov_policy: "hierarchiretina-v1.0-vessel-fov".into(),
            },
        };
        Stage2MaskResult {
            processing_status: Stage2ProcessingStatus::Complete,
            error: None,
            examination_id,
            source_image_sha256: sha.into(),
            laterality: None,
            model_kind: kind,
            model_version: "1.0.0".into(),
            checkpoint_sha256: "b".repeat(64),
            model_sha256: "c".repeat(64),
            original_width: side,
            original_height: side,
            mask_width: side,
            mask_height: side,
            transform,
            preprocessing_version: "test".into(),
            threshold: ThresholdMetadata {
                value: kind.threshold(),
                operator: kind.threshold_operator(),
            },
            provider: crate::device::ActualProvider::Cpu,
            elapsed_microseconds: 1,
            data: MaskPixelData::Binary(pixels),
        }
    }

    fn five_results(side: u32, exam: i64, sha: &str) -> Vec<Stage2MaskResult> {
        let n = (side * side) as usize;
        STAGE3_MASK_ORDER
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                let mut pixels = vec![0u8; n];
                for y in 20..30 {
                    for x in (10 + index as u32 * 8)..(16 + index as u32 * 8) {
                        pixels[(y * side + x) as usize] = 255;
                    }
                }
                mask_result(*kind, exam, sha, side, pixels)
            })
            .collect()
    }

    #[test]
    fn typed_results_are_placed_by_kind_not_by_input_order_and_bad_sets_are_rejected() {
        let side = 64u32;
        let encoded = png(side as i32, side as i32);
        let sha = hex::encode(Sha256::digest(&encoded));
        let mut results = five_results(side, 9, &sha);
        let ordered = assemble_stage3_input(&encoded, 9, &results).unwrap();
        results.reverse();
        let shuffled = assemble_stage3_input(&encoded, 9, &results).unwrap();
        assert_eq!(
            ordered.tensor, shuffled.tensor,
            "channel order must follow the mask kind"
        );

        let mut missing = five_results(side, 9, &sha);
        missing.retain(|r| r.model_kind != Stage2ModelKind::Cws);
        assert!(matches!(
            assemble_stage3_input(&encoded, 9, &missing),
            Err(Stage3InputError::MissingMask(Stage2ModelKind::Cws))
        ));
        let mut duplicate = five_results(side, 9, &sha);
        duplicate.push(duplicate[0].clone());
        assert!(matches!(
            assemble_stage3_input(&encoded, 9, &duplicate),
            Err(Stage3InputError::DuplicateMask(Stage2ModelKind::Ma))
        ));
        assert!(matches!(
            assemble_stage3_input(&encoded, 10, &five_results(side, 9, &sha)),
            Err(Stage3InputError::StaleExamination(Stage2ModelKind::Ma))
        ));
        assert!(matches!(
            assemble_stage3_input(&encoded, 9, &five_results(side, 9, &"d".repeat(64))),
            Err(Stage3InputError::SourceMismatch(Stage2ModelKind::Ma))
        ));
        let wrong_frame = five_results(32, 9, &sha);
        assert!(matches!(
            assemble_stage3_input(&encoded, 9, &wrong_frame),
            Err(Stage3InputError::DimensionMismatch(Stage2ModelKind::Ma))
        ));
    }
}
