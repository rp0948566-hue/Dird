//! HierarchiRetina v1.0 MA/HE/EX deployment preprocessing and source-frame mask restore.
use crate::stage2_contract::{ImageRect, Stage2ModelKind, Stage2SpatialTransform};
use image::{ImageFormat, ImageReader};
use opencv::{
    core::{self, Mat, MatTraitConst, MatTraitConstManual, Point, Point2f, Size, Vector},
    imgcodecs,
    imgproc::{self, CLAHETrait},
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
/// Upstream `detect_retinal_area` padding around the largest bright region (EX).
const RETINAL_AREA_PAD: i32 = 10;

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
    #[error("spatial transform does not match the lesion model or source frame")]
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

/// CLAHE on the LAB lightness channel of an RGB image (upstream `apply_clahe` / `clahe_lab`).
pub(crate) fn clahe_lab(rgb: &Mat, clip: f64, tile: i32) -> Result<Mat, Stage2PreprocessError> {
    let mut lab = Mat::default();
    imgproc::cvt_color_def(rgb, &mut lab, imgproc::COLOR_RGB2Lab)?;
    let mut channels = Vector::<Mat>::new();
    core::split(&lab, &mut channels)?;
    let mut clahe = imgproc::create_clahe(clip, Size::new(tile, tile))?;
    let mut equalized = Mat::default();
    clahe.apply(&channels.get(0)?, &mut equalized)?;
    let mut merged_channels = Vector::<Mat>::new();
    merged_channels.push(equalized);
    merged_channels.push(channels.get(1)?);
    merged_channels.push(channels.get(2)?);
    let mut merged = Mat::default();
    core::merge(&merged_channels, &mut merged)?;
    let mut out = Mat::default();
    imgproc::cvt_color_def(&merged, &mut out, imgproc::COLOR_Lab2RGB)?;
    Ok(out)
}

/// Read an encoded source image from disk with the Stage II size and file-type limits.
pub(crate) fn read_stage2_source(path: &Path) -> Result<Vec<u8>, Stage2PreprocessError> {
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
    Ok(encoded)
}

pub fn preprocess_stage2_lesion_image(
    path: &Path,
    model_kind: Stage2ModelKind,
) -> Result<PreprocessedStage2LesionImage, Stage2PreprocessError> {
    let encoded = read_stage2_source(path)?;
    preprocess_stage2_lesion_bytes(&encoded, model_kind)
}

/// A decoded JPEG/PNG fundus image in OpenCV BGR layout, shared by lesion and vessel preprocessing.
pub(crate) struct DecodedStage2Source {
    pub width: u32,
    pub height: u32,
    pub image: Mat,
}

pub(crate) fn decode_stage2_source(
    encoded: &[u8],
) -> Result<DecodedStage2Source, Stage2PreprocessError> {
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
    Ok(DecodedStage2Source {
        width: original_width,
        height: original_height,
        image: source,
    })
}

pub fn preprocess_stage2_lesion_bytes(
    encoded: &[u8],
    model_kind: Stage2ModelKind,
) -> Result<PreprocessedStage2LesionImage, Stage2PreprocessError> {
    let DecodedStage2Source {
        width: original_width,
        height: original_height,
        image: source,
    } = decode_stage2_source(encoded)?;
    let image_sha256 = hex::encode(Sha256::digest(encoded));
    let size = STAGE2_LESION_INPUT_SIZE;
    let (model_input, transform, preprocessing_version) = match model_kind {
        Stage2ModelKind::Ma | Stage2ModelKind::He => {
            let (crop_rect, crop) = circle_crop(&source)?;
            let mut rgb = Mat::default();
            imgproc::cvt_color_def(&crop, &mut rgb, imgproc::COLOR_BGR2RGB)?;
            let mut resized = Mat::default();
            imgproc::resize(
                &rgb,
                &mut resized,
                Size::new(size, size),
                0.0,
                0.0,
                imgproc::INTER_LINEAR,
            )?;
            let version = if model_kind == Stage2ModelKind::Ma {
                "hierarchiretina-v1.0-stage2-ma"
            } else {
                "hierarchiretina-v1.0-stage2-he"
            };
            (
                resized,
                Stage2SpatialTransform::LesionCircleStretch {
                    crop: crop_rect,
                    model_size: size as u32,
                },
                version,
            )
        }
        Stage2ModelKind::Ex | Stage2ModelKind::Cws => {
            let (crop_rect, crop) = retinal_area_crop(&source)?;
            let crop_width = crop.cols();
            let crop_height = crop.rows();
            let scale = f64::from(size) / f64::from(crop_width.max(crop_height));
            let resized_width = (f64::from(crop_width) * scale) as i32;
            let resized_height = (f64::from(crop_height) * scale) as i32;
            if resized_width <= 0 || resized_height <= 0 {
                return Err(Stage2PreprocessError::InvalidDimensions);
            }
            let mut resized = Mat::default();
            imgproc::resize(
                &crop,
                &mut resized,
                Size::new(resized_width, resized_height),
                0.0,
                0.0,
                imgproc::INTER_LINEAR,
            )?;
            let offset_x = (size - resized_width) / 2;
            let offset_y = (size - resized_height) / 2;
            let mut canvas =
                Mat::new_rows_cols_with_default(size, size, core::CV_8UC3, core::Scalar::all(0.0))?;
            {
                let mut target = Mat::roi_mut(
                    &mut canvas,
                    core::Rect::new(offset_x, offset_y, resized_width, resized_height),
                )?;
                resized.copy_to(&mut target)?;
            }
            let mut rgb = Mat::default();
            imgproc::cvt_color_def(&canvas, &mut rgb, imgproc::COLOR_BGR2RGB)?;
            (
                rgb,
                Stage2SpatialTransform::LesionRectLetterbox {
                    crop: crop_rect,
                    model_size: size as u32,
                    resized_width: resized_width as u32,
                    resized_height: resized_height as u32,
                    offset_x: offset_x as u32,
                    offset_y: offset_y as u32,
                },
                if model_kind == Stage2ModelKind::Cws {
                    "hierarchiretina-v1.0-stage2-cws"
                } else {
                    "hierarchiretina-v1.0-stage2-ex"
                },
            )
        }
        _ => return Err(Stage2PreprocessError::InvalidTransform),
    };
    let bytes = model_input.data_bytes()?;
    if bytes.len() != (size * size * 3) as usize {
        return Err(Stage2PreprocessError::InvalidOutputBuffer);
    }
    let plane = (size * size) as usize;
    let mut tensor = vec![0.0f32; plane * 3];
    for (index, pixel) in bytes.chunks(3).enumerate() {
        for channel in 0..3 {
            let unit = f32::from(pixel[channel]) / 255.0;
            tensor[channel * plane + index] = (unit - IMAGE_MEAN[channel]) / IMAGE_STD[channel];
        }
    }
    Ok(PreprocessedStage2LesionImage {
        image_sha256,
        original_width,
        original_height,
        tensor,
        transform,
        preprocessing_version,
    })
}

/// Foreground mask used by both MA/HE circle detection and EX rectangle detection.
fn foreground_mask(
    source: &Mat,
    threshold: f64,
    kernel_size: i32,
) -> Result<Mat, Stage2PreprocessError> {
    let mut gray = Mat::default();
    imgproc::cvt_color_def(source, &mut gray, imgproc::COLOR_BGR2GRAY)?;
    let mut mask = Mat::default();
    imgproc::threshold(&gray, &mut mask, threshold, 255.0, imgproc::THRESH_BINARY)?;
    let kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(kernel_size, kernel_size),
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
    Ok(mask)
}

fn largest_contour(
    contours: &Vector<Vector<Point>>,
) -> Result<Option<usize>, Stage2PreprocessError> {
    let mut selected: Option<(f64, usize)> = None;
    for index in 0..contours.len() {
        let area = imgproc::contour_area(&contours.get(index)?, false)?;
        if selected.is_none_or(|(best, _)| area > best) {
            selected = Some((area, index));
        }
    }
    Ok(selected.map(|(_, index)| index))
}

fn copy_region(source: &Mat, bounds: &ImageRect) -> Result<Mat, Stage2PreprocessError> {
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
    Ok(crop)
}

fn circle_crop(source: &Mat) -> Result<(ImageRect, Mat), Stage2PreprocessError> {
    let width = source.cols();
    let height = source.rows();
    let mask = foreground_mask(source, 20.0, 15)?;
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours(
        &mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )?;
    let mut bounds = ImageRect {
        x: 0,
        y: 0,
        width: width as u32,
        height: height as u32,
    };
    if let Some(index) = largest_contour(&contours)? {
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
    let crop = copy_region(source, &bounds)?;
    Ok((bounds, crop))
}

/// Upstream `detect_retinal_area` (EX): bounding rectangle of the largest bright region plus padding.
fn retinal_area_crop(source: &Mat) -> Result<(ImageRect, Mat), Stage2PreprocessError> {
    let width = source.cols();
    let height = source.rows();
    let mask = foreground_mask(source, 15.0, 11)?;
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours(
        &mask,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )?;
    let mut bounds = ImageRect {
        x: 0,
        y: 0,
        width: width as u32,
        height: height as u32,
    };
    if let Some(index) = largest_contour(&contours)? {
        let rect = imgproc::bounding_rect(&contours.get(index)?)?;
        let x1 = (rect.x - RETINAL_AREA_PAD).max(0);
        let y1 = (rect.y - RETINAL_AREA_PAD).max(0);
        let x2 = (rect.x + rect.width + RETINAL_AREA_PAD).min(width);
        let y2 = (rect.y + rect.height + RETINAL_AREA_PAD).min(height);
        if x2 > x1 && y2 > y1 {
            bounds = ImageRect {
                x: x1 as u32,
                y: y1 as u32,
                width: (x2 - x1) as u32,
                height: (y2 - y1) as u32,
            };
        }
    }
    let crop = copy_region(source, &bounds)?;
    Ok((bounds, crop))
}

/// Sigmoid of finite, correctly sized logits in the model's 1024 x 1024 frame.
pub fn stage2_probabilities_from_logits(logits: &[f32]) -> Result<Vec<f32>, Stage2PreprocessError> {
    let expected = (STAGE2_LESION_INPUT_SIZE * STAGE2_LESION_INPUT_SIZE) as usize;
    if logits.len() != expected || logits.iter().any(|value| !value.is_finite()) {
        return Err(Stage2PreprocessError::InvalidLogits);
    }
    Ok(logits.iter().map(|&logit| sigmoid(logit)).collect())
}

/// Flip a channel-major `planes x size x size` buffer horizontally or vertically (test-time augmentation).
pub fn flip_planes(data: &[f32], planes: usize, horizontal: bool) -> Vec<f32> {
    flip_planes_hv(
        data,
        planes,
        STAGE2_LESION_INPUT_SIZE as usize,
        horizontal,
        !horizontal,
    )
}

/// Flip a channel-major `planes x size x size` buffer along width and/or height (test-time augmentation).
pub fn flip_planes_hv(
    data: &[f32],
    planes: usize,
    size: usize,
    flip_width: bool,
    flip_height: bool,
) -> Vec<f32> {
    let mut flipped = vec![0.0f32; data.len()];
    for plane in 0..planes {
        for row in 0..size {
            for col in 0..size {
                let src_row = if flip_height { size - 1 - row } else { row };
                let src_col = if flip_width { size - 1 - col } else { col };
                flipped[plane * size * size + row * size + col] =
                    data[plane * size * size + src_row * size + src_col];
            }
        }
    }
    flipped
}

pub fn restore_stage2_lesion_mask(
    logits: &[f32],
    transform: &Stage2SpatialTransform,
    original_width: u32,
    original_height: u32,
    threshold: f32,
) -> Result<Vec<u8>, Stage2PreprocessError> {
    let probabilities = stage2_probabilities_from_logits(logits)?;
    restore_stage2_probability_mask(
        &probabilities,
        transform,
        original_width,
        original_height,
        threshold,
    )
}

/// Deployment post-processing applied at crop resolution before pasting into the source frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage2MaskPostprocess {
    /// Strict threshold only (MA, HE, EX).
    Threshold,
    /// Upstream CWS: 5x5 elliptical closing and removal of small 8-connected components.
    CwsComponents,
}

/// Map a 1024 x 1024 probability map back to the source frame, removing letterbox padding first.
pub fn restore_stage2_probability_mask(
    probabilities: &[f32],
    transform: &Stage2SpatialTransform,
    original_width: u32,
    original_height: u32,
    threshold: f32,
) -> Result<Vec<u8>, Stage2PreprocessError> {
    restore_stage2_probability_mask_with(
        probabilities,
        transform,
        original_width,
        original_height,
        threshold,
        Stage2MaskPostprocess::Threshold,
    )
}

pub fn restore_stage2_probability_mask_with(
    probabilities: &[f32],
    transform: &Stage2SpatialTransform,
    original_width: u32,
    original_height: u32,
    threshold: f32,
    postprocess: Stage2MaskPostprocess,
) -> Result<Vec<u8>, Stage2PreprocessError> {
    let size = STAGE2_LESION_INPUT_SIZE as u32;
    let (crop, region) = match transform {
        Stage2SpatialTransform::LesionCircleStretch { crop, model_size } => {
            if *model_size != size {
                return Err(Stage2PreprocessError::InvalidTransform);
            }
            (
                *crop,
                ImageRect {
                    x: 0,
                    y: 0,
                    width: size,
                    height: size,
                },
            )
        }
        Stage2SpatialTransform::LesionRectLetterbox {
            crop,
            model_size,
            resized_width,
            resized_height,
            offset_x,
            offset_y,
        } => {
            if *model_size != size
                || offset_x
                    .checked_add(*resized_width)
                    .is_none_or(|end| end > size)
                || offset_y
                    .checked_add(*resized_height)
                    .is_none_or(|end| end > size)
            {
                return Err(Stage2PreprocessError::InvalidTransform);
            }
            (
                *crop,
                ImageRect {
                    x: *offset_x,
                    y: *offset_y,
                    width: *resized_width,
                    height: *resized_height,
                },
            )
        }
        _ => return Err(Stage2PreprocessError::InvalidTransform),
    };
    let expected = (STAGE2_LESION_INPUT_SIZE * STAGE2_LESION_INPUT_SIZE) as usize;
    if probabilities.len() != expected
        || probabilities
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
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
    let flat = Mat::from_slice(probabilities)?;
    let model_map = flat.reshape(1, STAGE2_LESION_INPUT_SIZE)?;
    let region_map = Mat::roi(
        &model_map,
        core::Rect::new(
            region.x as i32,
            region.y as i32,
            region.width as i32,
            region.height as i32,
        ),
    )?;
    let mut crop_map = Mat::default();
    imgproc::resize(
        &region_map,
        &mut crop_map,
        Size::new(crop.width as i32, crop.height as i32),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    let rows = crop.height as i32;
    let cols = crop.width as i32;
    let mut binary = vec![0u8; (rows * cols) as usize];
    for row in 0..rows {
        for col in 0..cols {
            binary[(row * cols + col) as usize] =
                u8::from(*crop_map.at_2d::<f32>(row, col)? > threshold);
        }
    }
    if postprocess == Stage2MaskPostprocess::CwsComponents {
        // Upstream: min_area = int(40 * max(1, crop_area / 1024^2)).
        let crop_scale = f64::max(
            1.0,
            f64::from(crop.width) * f64::from(crop.height) / f64::from(size * size),
        );
        let min_area = (40.0 * crop_scale) as i32;
        binary = cws_components(&binary, rows, cols, min_area)?;
    }
    let frame_len = (original_width as usize) * (original_height as usize);
    let mut mask = vec![0u8; frame_len];
    for row in 0..rows {
        for col in 0..cols {
            if binary[(row * cols + col) as usize] == 1 {
                let x = crop.x + col as u32;
                let y = crop.y + row as u32;
                mask[y as usize * original_width as usize + x as usize] = 255;
            }
        }
    }
    Ok(mask)
}

/// Upstream `cws_postprocess`: 5x5 ellipse closing, then keep 8-connected components of at least `min_area` px.
fn cws_components(
    binary: &[u8],
    rows: i32,
    cols: i32,
    min_area: i32,
) -> Result<Vec<u8>, Stage2PreprocessError> {
    let flat = Mat::from_slice(binary)?;
    let source = flat.reshape(1, rows)?;
    let kernel = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(5, 5),
        Point::new(-1, -1),
    )?;
    let mut closed = Mat::default();
    imgproc::morphology_ex(
        &source,
        &mut closed,
        imgproc::MORPH_CLOSE,
        &kernel,
        Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        imgproc::morphology_default_border_value()?,
    )?;
    let mut labels = Mat::default();
    let mut stats = Mat::default();
    let mut centroids = Mat::default();
    let count = imgproc::connected_components_with_stats(
        &closed,
        &mut labels,
        &mut stats,
        &mut centroids,
        8,
        core::CV_32S,
    )?;
    let mut keep = vec![0u8; count as usize];
    for label in 1..count {
        keep[label as usize] =
            u8::from(*stats.at_2d::<i32>(label, imgproc::CC_STAT_AREA)? >= min_area);
    }
    let mut out = vec![0u8; (rows * cols) as usize];
    for row in 0..rows {
        for col in 0..cols {
            let label = *labels.at_2d::<i32>(row, col)? as usize;
            out[(row * cols + col) as usize] = keep[label];
        }
    }
    Ok(out)
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
    fn preprocesses_ex_to_letterboxed_tensor_with_rect_registration() {
        let prepared =
            preprocess_stage2_lesion_bytes(&synthetic_png(), Stage2ModelKind::Ex).unwrap();
        assert_eq!(prepared.tensor.len(), 3 * 1024 * 1024);
        assert!(prepared.tensor.iter().all(|pixel| pixel.is_finite()));
        let Stage2SpatialTransform::LesionRectLetterbox {
            crop,
            model_size,
            resized_width,
            resized_height,
            offset_x,
            offset_y,
        } = prepared.transform
        else {
            panic!("wrong EX transform");
        };
        assert_eq!(model_size, 1024);
        assert!(crop.width <= 96 && crop.height <= 80);
        assert_eq!(resized_width.max(resized_height), 1024);
        assert_eq!(offset_x, (1024 - resized_width) / 2);
        assert_eq!(offset_y, (1024 - resized_height) / 2);
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
    fn letterbox_restore_ignores_padding_and_keeps_registered_crop() {
        let transform = Stage2SpatialTransform::LesionRectLetterbox {
            crop: ImageRect {
                x: 1,
                y: 1,
                width: 3,
                height: 2,
            },
            model_size: 1024,
            resized_width: 1024,
            resized_height: 512,
            offset_x: 0,
            offset_y: 256,
        };
        // Padding rows are fully positive; the real letterboxed region is fully negative.
        let mut probabilities = vec![0.0f32; 1024 * 1024];
        probabilities[..256 * 1024].fill(1.0);
        assert!(
            restore_stage2_probability_mask(&probabilities, &transform, 5, 4, 0.325)
                .unwrap()
                .iter()
                .all(|&pixel| pixel == 0)
        );
        probabilities[..256 * 1024].fill(0.0);
        probabilities[256 * 1024..].fill(1.0);
        let mask =
            restore_stage2_probability_mask(&probabilities, &transform, 5, 4, 0.325).unwrap();
        for y in 0..4 {
            for x in 0..5 {
                let inside = (1..4).contains(&x) && (1..3).contains(&y);
                assert_eq!(mask[y * 5 + x], if inside { 255 } else { 0 });
            }
        }
    }

    #[test]
    fn flips_planes_horizontally_and_vertically_as_involutions() {
        let size = STAGE2_LESION_INPUT_SIZE as usize;
        let data: Vec<f32> = (0..2 * size * size).map(|index| index as f32).collect();
        let horizontal = flip_planes(&data, 2, true);
        let vertical = flip_planes(&data, 2, false);
        assert_eq!(flip_planes(&horizontal, 2, true), data);
        assert_eq!(flip_planes(&vertical, 2, false), data);
        assert_eq!(horizontal[0], data[size - 1]);
        assert_eq!(vertical[0], data[(size - 1) * size]);
    }

    #[test]
    fn cws_components_close_small_gaps_and_drop_small_components() {
        let (rows, cols) = (40, 40);
        let mut binary = vec![0u8; rows * cols];
        // 10x10 block with a one-pixel hole: closing fills it, area 100 >= 40 keeps it.
        for row in 5..15 {
            for col in 5..15 {
                binary[row * cols + col] = 1;
            }
        }
        binary[10 * cols + 10] = 0;
        // Single isolated pixel: area 1 < 40 is removed.
        binary[35 * cols + 35] = 1;
        let kept = cws_components(&binary, rows as i32, cols as i32, 40).unwrap();
        assert_eq!(kept[10 * cols + 10], 1);
        assert_eq!(kept[35 * cols + 35], 0);
        assert_eq!(kept.iter().map(|&v| usize::from(v)).sum::<usize>(), 100);
    }

    #[test]
    fn rejects_nonfinite_logits_and_mismatched_transforms() {
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
        let bad_size = Stage2SpatialTransform::LesionCircleStretch {
            crop: ImageRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            model_size: 512,
        };
        assert!(matches!(
            restore_stage2_probability_mask(&vec![0.0; 1024 * 1024], &bad_size, 2, 2, 0.5),
            Err(Stage2PreprocessError::InvalidTransform)
        ));
    }
}
