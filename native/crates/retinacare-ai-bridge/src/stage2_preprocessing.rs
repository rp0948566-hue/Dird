//! HierarchiRetina v1.0 MA/HE deployment preprocessing and source-frame mask restore.
use crate::stage2_contract::{ImageRect, Stage2ModelKind, Stage2SpatialTransform};
use image::{ImageFormat, ImageReader};
use opencv::{
    core::{self, Mat, MatTraitConst, MatTraitConstManual, Point, Point2f, Size, Vector},
    imgcodecs, imgproc,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::Path,
};
use thiserror::Error;

pub const STAGE2_LESION_INPUT_SIZE: i32 = 1024;
const MAX_ENCODED_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
const IMAGE_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGE_STD: [f32; 3] = [0.229, 0.224, 0.225];

#[derive(Debug, Error)]
pub enum Stage2PreprocessError {
    #[error("image file cannot be read: {0}")]
    Io(#[from] std::io::Error),
    #[error("image header is invalid: {0}")]
    Header(#[from] image::ImageError),
    #[error("only JPEG and PNG fundus images are supported")]
    UnsupportedFormat,
    #[error("encoded image exceeds {limit} bytes")]
    EncodedTooLarge { limit: usize },
    #[error("image dimensions must be nonzero")]
    InvalidDimensions,
    #[error("image has {pixels} pixels; maximum is {limit}")]
    ImageTooLarge { pixels: u64, limit: u64 },
    #[error("OpenCV could not decode the image")]
    DecodeFailed,
    #[error("decoded image is not an 8-bit, three-channel BGR image")]
    UnsupportedDecodedImage,
    #[error("OpenCV preprocessing failed: {0}")]
    OpenCv(#[from] opencv::Error),
    #[error("OpenCV produced an unexpected image buffer")]
    InvalidOutputBuffer,
    #[error("model output must contain exactly 1024 x 1024 finite logits")]
    InvalidLogits,
    #[error("spatial transform is not a lesion circle crop transform")]
    InvalidTransform,
}

#[derive(Debug)]
pub struct PreprocessedStage2LesionImage {
    pub image_sha256: String,
    pub original_width: u32,
    pub original_height: u32,
    pub tensor: Vec<f32>,
    pub transform: Stage2SpatialTransform,
    pub preprocessing_version: &'static str,
}

pub fn preprocess_stage2_lesion_image(
    path: &Path,
    model_kind: Stage2ModelKind,
) -> Result<PreprocessedStage2LesionImage, Stage2PreprocessError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(Stage2PreprocessError::DecodeFailed);
    }
    if metadata.len() > MAX_ENCODED_BYTES as u64 {
        return Err(Stage2PreprocessError::EncodedTooLarge {
            limit: MAX_ENCODED_BYTES,
        });
    }
    let mut encoded = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(MAX_ENCODED_BYTES as u64 + 1)
        .read_to_end(&mut encoded)?;
    preprocess_stage2_lesion_bytes(&encoded, model_kind)
}

pub fn preprocess_stage2_lesion_bytes(
    encoded: &[u8],
    model_kind: Stage2ModelKind,
) -> Result<PreprocessedStage2LesionImage, Stage2PreprocessError> {
    if !matches!(model_kind, Stage2ModelKind::Ma | Stage2ModelKind::He) {
        return Err(Stage2PreprocessError::InvalidTransform);
    }
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(Stage2PreprocessError::EncodedTooLarge {
            limit: MAX_ENCODED_BYTES,
        });
    }
    let reader = ImageReader::new(Cursor::new(encoded)).with_guessed_format()?;
    if !matches!(reader.format(), Some(ImageFormat::Jpeg | ImageFormat::Png)) {
        return Err(Stage2PreprocessError::UnsupportedFormat);
    }
    let (original_width, original_height) = reader.into_dimensions()?;
    if original_width == 0 || original_height == 0 {
        return Err(Stage2PreprocessError::InvalidDimensions);
    }
    let pixels = u64::from(original_width) * u64::from(original_height);
    if pixels > MAX_IMAGE_PIXELS {
        return Err(Stage2PreprocessError::ImageTooLarge {
            pixels,
            limit: MAX_IMAGE_PIXELS,
        });
    }
    let source = imgcodecs::imdecode(&Vector::from_slice(encoded), imgcodecs::IMREAD_COLOR)?;
    if source.empty() {
        return Err(Stage2PreprocessError::DecodeFailed);
    }
    if source.depth() != core::CV_8U || source.channels() != 3 {
        return Err(Stage2PreprocessError::UnsupportedDecodedImage);
    }
    let image_sha256 = hex::encode(Sha256::digest(encoded));
    let (crop_rect, crop) = circle_crop(&source)?;
    let mut rgb = Mat::default();
    imgproc::cvt_color_def(&crop, &mut rgb, imgproc::COLOR_BGR2RGB)?;
    let mut resized = Mat::default();
    imgproc::resize(
        &rgb,
        &mut resized,
        Size::new(STAGE2_LESION_INPUT_SIZE, STAGE2_LESION_INPUT_SIZE),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    let bytes = resized.data_bytes()?;
    if bytes.len() != (STAGE2_LESION_INPUT_SIZE * STAGE2_LESION_INPUT_SIZE * 3) as usize {
        return Err(Stage2PreprocessError::InvalidOutputBuffer);
    }
    let plane = (STAGE2_LESION_INPUT_SIZE * STAGE2_LESION_INPUT_SIZE) as usize;
    let mut tensor = vec![0.0f32; plane * 3];
    for (index, pixel) in bytes.chunks(3).enumerate() {
        for channel in 0..3 {
            let unit = f32::from(pixel[channel]) / 255.0;
            tensor[channel * plane + index] = (unit - IMAGE_MEAN[channel]) / IMAGE_STD[channel];
        }
    }
    let preprocessing_version = match model_kind {
        Stage2ModelKind::Ma => "hierarchiretina-v1.0-stage2-ma",
        Stage2ModelKind::He => "hierarchiretina-v1.0-stage2-he",
        _ => return Err(Stage2PreprocessError::InvalidTransform),
    };
    Ok(PreprocessedStage2LesionImage {
        image_sha256,
        original_width,
        original_height,
        tensor,
        transform: Stage2SpatialTransform::LesionCircleStretch {
            crop: crop_rect,
            model_size: STAGE2_LESION_INPUT_SIZE as u32,
        },
        preprocessing_version,
    })
}

fn circle_crop(source: &Mat) -> Result<(ImageRect, Mat), Stage2PreprocessError> {
    let width = source.cols();
    let height = source.rows();
    let mut gray = Mat::default();
    imgproc::cvt_color_def(source, &mut gray, imgproc::COLOR_BGR2GRAY)?;
    let mut mask = Mat::default();
    imgproc::threshold(&gray, &mut mask, 20.0, 255.0, imgproc::THRESH_BINARY)?;
    let kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(15, 15),
        Point::new(-1, -1),
    )?;
    for (operation, iterations) in [(imgproc::MORPH_CLOSE, 3), (imgproc::MORPH_OPEN, 2)] {
        let mut next = Mat::default();
        imgproc::morphology_ex(
            &mask,
            &mut next,
            operation,
            &kernel,
            Point::new(-1, -1),
            iterations,
            core::BORDER_CONSTANT,
            imgproc::morphology_default_border_value()?,
        )?;
        mask = next;
    }
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours(
        &mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )?;
    let mut selected: Option<(f64, usize)> = None;
    for index in 0..contours.len() {
        let area = imgproc::contour_area(&contours.get(index)?, false)?;
        if selected.is_none_or(|(best, _)| area > best) {
            selected = Some((area, index));
        }
    }
    let mut bounds = ImageRect {
        x: 0,
        y: 0,
        width: width as u32,
        height: height as u32,
    };
    if let Some((_, index)) = selected {
        let contour = contours.get(index)?;
        let mut center = Point2f::default();
        let mut radius = 0.0f32;
        imgproc::min_enclosing_circle(&contour, &mut center, &mut radius)?;
        let cx = center.x as i32;
        let cy = center.y as i32;
        let radius = radius as i32;
        if radius > height.min(width) / 10 {
            let x1 = (cx - radius).max(0);
            let y1 = (cy - radius).max(0);
            let x2 = (cx + radius).min(width);
            let y2 = (cy + radius).min(height);
            if x2 > x1 && y2 > y1 {
                bounds = ImageRect {
                    x: x1 as u32,
                    y: y1 as u32,
                    width: (x2 - x1) as u32,
                    height: (y2 - y1) as u32,
                };
            }
        }
    }
    let roi = Mat::roi(
        source,
        core::Rect::new(
            bounds.x as i32,
            bounds.y as i32,
            bounds.width as i32,
            bounds.height as i32,
        ),
    )?;
    let mut crop = Mat::default();
    roi.copy_to(&mut crop)?;
    Ok((bounds, crop))
}

pub fn restore_stage2_lesion_mask(
    logits: &[f32],
    transform: &Stage2SpatialTransform,
    original_width: u32,
    original_height: u32,
    threshold: f32,
) -> Result<Vec<u8>, Stage2PreprocessError> {
    let Stage2SpatialTransform::LesionCircleStretch { crop, model_size } = transform else {
        return Err(Stage2PreprocessError::InvalidTransform);
    };
    let expected = (STAGE2_LESION_INPUT_SIZE * STAGE2_LESION_INPUT_SIZE) as usize;
    if *model_size != STAGE2_LESION_INPUT_SIZE as u32
        || logits.len() != expected
        || logits.iter().any(|value| !value.is_finite())
        || !threshold.is_finite()
        || !(0.0..=1.0).contains(&threshold)
    {
        return Err(Stage2PreprocessError::InvalidLogits);
    }
    if original_width == 0
        || original_height == 0
        || u64::from(original_width) * u64::from(original_height) > MAX_IMAGE_PIXELS
        || crop.width == 0
        || crop.height == 0
        || crop
            .x
            .checked_add(crop.width)
            .is_none_or(|x| x > original_width)
        || crop
            .y
            .checked_add(crop.height)
            .is_none_or(|y| y > original_height)
    {
        return Err(Stage2PreprocessError::InvalidTransform);
    }
    let probabilities: Vec<f32> = logits.iter().map(|&logit| sigmoid(logit)).collect();
    let flat = Mat::from_slice(&probabilities)?;
    let model_map = flat.reshape(1, STAGE2_LESION_INPUT_SIZE)?;
    let mut crop_map = Mat::default();
    imgproc::resize(
        &model_map,
        &mut crop_map,
        Size::new(crop.width as i32, crop.height as i32),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    let frame_len = (original_width as usize) * (original_height as usize);
    let mut mask = vec![0u8; frame_len];
    for row in 0..crop.height as i32 {
        for col in 0..crop.width as i32 {
            if *crop_map.at_2d::<f32>(row, col)? > threshold {
                let x = crop.x + col as u32;
                let y = crop.y + row as u32;
                mask[y as usize * original_width as usize + x as usize] = 255;
            }
        }
    }
    Ok(mask)
}

fn sigmoid(value: f32) -> f32 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::core::Scalar;
    use opencv::prelude::VectorToVec;

    fn synthetic_png() -> Vec<u8> {
        let mut image =
            Mat::new_rows_cols_with_default(80, 96, core::CV_8UC3, Scalar::all(0.0)).unwrap();
        imgproc::circle(
            &mut image,
            Point::new(48, 40),
            30,
            Scalar::new(180.0, 140.0, 100.0, 0.0),
            -1,
            imgproc::LINE_8,
            0,
        )
        .unwrap();
        let mut encoded = Vector::new();
        imgcodecs::imencode(".png", &image, &mut encoded, &Vector::new()).unwrap();
        encoded.to_vec()
    }

    #[test]
    fn preprocesses_ma_to_finite_imagenet_tensor_with_circle_registration() {
        let prepared =
            preprocess_stage2_lesion_bytes(&synthetic_png(), Stage2ModelKind::Ma).unwrap();
        assert_eq!(
            (prepared.original_width, prepared.original_height),
            (96, 80)
        );
        assert_eq!(prepared.tensor.len(), 3 * 1024 * 1024);
        assert!(prepared.tensor.iter().all(|pixel| pixel.is_finite()));
        let Stage2SpatialTransform::LesionCircleStretch { crop, model_size } = prepared.transform
        else {
            panic!("wrong MA transform");
        };
        assert!(crop.width < 96 && crop.height < 80);
        assert_eq!(model_size, 1024);
    }

    #[test]
    fn restores_thresholded_ma_map_inside_registered_source_crop() {
        let transform = Stage2SpatialTransform::LesionCircleStretch {
            crop: ImageRect {
                x: 2,
                y: 1,
                width: 3,
                height: 2,
            },
            model_size: 1024,
        };
        let logits = vec![0.0f32; 1024 * 1024];
        let mask = restore_stage2_lesion_mask(&logits, &transform, 7, 5, 0.01).unwrap();
        assert_eq!(mask.len(), 35);
        for y in 0..5 {
            for x in 0..7 {
                let inside = (2..5).contains(&x) && (1..3).contains(&y);
                assert_eq!(mask[y * 7 + x], if inside { 255 } else { 0 });
            }
        }
    }

    #[test]
    fn rejects_nonfinite_logits_and_non_ma_transforms() {
        let transform = Stage2SpatialTransform::LesionCircleStretch {
            crop: ImageRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            model_size: 1024,
        };
        let mut logits = vec![0.0; 1024 * 1024];
        logits[0] = f32::NAN;
        assert!(matches!(
            restore_stage2_lesion_mask(&logits, &transform, 2, 2, 0.01),
            Err(Stage2PreprocessError::InvalidLogits)
        ));
    }
}
