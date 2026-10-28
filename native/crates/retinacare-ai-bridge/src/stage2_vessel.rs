//! HierarchiRetina v1.0 SwinHRUNet++ vessel preprocessing, flip TTA and source-frame restore.
//!
//! Mirrors `hierarchiretina/stage2/vessel_inference.py` and `vessel_metrics.py` at the pinned
//! commit: FOV mask (BGR input, RGB grey weights, quirk kept), LAB-L CLAHE, green enhancement,
//! square FOV crop, 512 resize, ImageNet normalisation, 4-view flip TTA, FOV-masked
//! restore, `>= 0.46` threshold, full-frame FOV, 3x3 opening and components under 30 px removed.
use crate::stage2_contract::{ImageRect, Stage2SpatialTransform};
use crate::stage2_preprocessing::clahe_lab;
use crate::stage2_preprocessing::{
    Stage2PreprocessError, decode_stage2_source, flip_planes_hv, read_stage2_source,
};
use opencv::{
    core::{
        self, Mat, MatExprTraitConst, MatTrait, MatTraitConst, MatTraitConstManual, MatTraitManual,
        Point, Scalar, Size, Vector,
    },
    imgproc,
};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const VESSEL_INPUT_SIZE: i32 = 512;
pub const VESSEL_FOV_POLICY: &str = "hierarchiretina-v1.0-vessel-fov";
pub const VESSEL_PREPROCESSING_VERSION: &str = "hierarchiretina-v1.0-stage2-vessel";
const IMAGE_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGE_STD: [f32; 3] = [0.229, 0.224, 0.225];
const FOV_MARGIN: f64 = 0.01;
const GREEN_STRENGTH: f32 = 0.35;
const MIN_COMPONENT_AREA: i32 = 30;

/// Vessel model input and the masks needed to restore probabilities to the source frame.
#[derive(Debug)]
pub struct PreprocessedStage2VesselImage {
    pub image_sha256: String,
    pub original_width: u32,
    pub original_height: u32,
    pub tensor: Vec<f32>,
    pub transform: Stage2SpatialTransform,
    /// Model-resolution FOV (512 x 512, values 0 or 1).
    pub model_fov: Vec<u8>,
    /// Full-resolution FOV of the source frame (values 0 or 1).
    pub frame_fov: Vec<u8>,
}

pub fn preprocess_stage2_vessel_image(
    path: &Path,
) -> Result<PreprocessedStage2VesselImage, Stage2PreprocessError> {
    let encoded = read_stage2_source(path)?;
    preprocess_stage2_vessel_bytes(&encoded)
}

pub fn preprocess_stage2_vessel_bytes(
    encoded: &[u8],
) -> Result<PreprocessedStage2VesselImage, Stage2PreprocessError> {
    let decoded = decode_stage2_source(encoded)?;
    let image_sha256 = hex::encode(Sha256::digest(encoded));
    let frame_fov_mat = generate_fov(&decoded.image)?;
    let mut rgb = Mat::default();
    imgproc::cvt_color_def(&decoded.image, &mut rgb, imgproc::COLOR_BGR2RGB)?;
    let enhanced = enhance_green(&clahe_lab(&rgb, 2.0, 8)?)?;
    let (x1, y1, x2, y2) = fov_crop_box(&frame_fov_mat)?;
    let crop_rect = core::Rect::new(x1, y1, x2 - x1, y2 - y1);
    let crop = Mat::roi(&enhanced, crop_rect)?;
    let mut crop_owned = Mat::default();
    crop.copy_to(&mut crop_owned)?;
    let fov_crop = Mat::roi(&frame_fov_mat, crop_rect)?;
    let mut fov_crop_owned = Mat::default();
    fov_crop.copy_to(&mut fov_crop_owned)?;

    let size = VESSEL_INPUT_SIZE;
    let mut resized_image = Mat::default();
    imgproc::resize(
        &crop_owned,
        &mut resized_image,
        Size::new(size, size),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    let mut resized_fov = Mat::default();
    imgproc::resize(
        &fov_crop_owned,
        &mut resized_fov,
        Size::new(size, size),
        0.0,
        0.0,
        imgproc::INTER_NEAREST,
    )?;
    let model_fov = mat_to_binary(&resized_fov)?;
    let bytes = resized_image.data_bytes()?;
    let plane = (size * size) as usize;
    if bytes.len() != plane * 3 {
        return Err(Stage2PreprocessError::InvalidOutputBuffer);
    }
    let mut tensor = vec![0.0f32; plane * 3];
    for (index, pixel) in bytes.chunks(3).enumerate() {
        for channel in 0..3 {
            let unit = f32::from(pixel[channel]) / 255.0;
            tensor[channel * plane + index] = (unit - IMAGE_MEAN[channel]) / IMAGE_STD[channel];
        }
    }
    let transform = Stage2SpatialTransform::VesselFovSquare {
        crop: ImageRect {
            x: x1 as u32,
            y: y1 as u32,
            width: (x2 - x1) as u32,
            height: (y2 - y1) as u32,
        },
        model_size: size as u32,
        fov_policy: VESSEL_FOV_POLICY.into(),
    };
    Ok(PreprocessedStage2VesselImage {
        image_sha256,
        original_width: decoded.width,
        original_height: decoded.height,
        tensor,
        transform,
        model_fov,
        frame_fov: mat_to_binary(&frame_fov_mat)?,
    })
}

/// Upstream `generate_fov_mask(img, is_rgb=True)`: note the RGB grey weights applied to BGR input.
fn generate_fov(bgr: &Mat) -> Result<Mat, Stage2PreprocessError> {
    let mut gray = Mat::default();
    imgproc::cvt_color_def(bgr, &mut gray, imgproc::COLOR_RGB2GRAY)?;
    let mut binary = Mat::default();
    imgproc::threshold(&gray, &mut binary, 12.0, 255.0, imgproc::THRESH_BINARY)?;
    let kernel25 = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(25, 25),
        Point::new(-1, -1),
    )?;
    for operation in [imgproc::MORPH_CLOSE, imgproc::MORPH_OPEN] {
        let mut next = Mat::default();
        imgproc::morphology_ex(
            &binary,
            &mut next,
            operation,
            &kernel25,
            Point::new(-1, -1),
            1,
            core::BORDER_CONSTANT,
            imgproc::morphology_default_border_value()?,
        )?;
        binary = next;
    }
    let mut contours = Vector::<Vector<Point>>::new();
    imgproc::find_contours(
        &binary,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        Point::new(0, 0),
    )?;
    let mut fov = Mat::zeros(gray.rows(), gray.cols(), core::CV_8UC1)?.to_mat()?;
    if let Some(index) = largest_contour_index(&contours)? {
        let mut single = Vector::<Vector<Point>>::new();
        single.push(contours.get(index)?);
        imgproc::draw_contours(
            &mut fov,
            &single,
            0,
            Scalar::all(255.0),
            -1,
            imgproc::LINE_8,
            &core::no_array(),
            i32::MAX,
            Point::new(0, 0),
        )?;
    } else {
        fov.set_to(&Scalar::all(255.0), &core::no_array())?;
    }
    let kernel9 = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(9, 9),
        Point::new(-1, -1),
    )?;
    let mut eroded = Mat::default();
    imgproc::erode(
        &fov,
        &mut eroded,
        &kernel9,
        Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        imgproc::morphology_default_border_value()?,
    )?;
    let mut binary_fov = Mat::default();
    imgproc::threshold(&eroded, &mut binary_fov, 127.0, 1.0, imgproc::THRESH_BINARY)?;
    Ok(binary_fov)
}

fn largest_contour_index(
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

/// Upstream `enhance_green`: blend the green channel with its histogram-equalised version.
fn enhance_green(rgb: &Mat) -> Result<Mat, Stage2PreprocessError> {
    let mut channels = Vector::<Mat>::new();
    core::split(rgb, &mut channels)?;
    let green = channels.get(1)?;
    let mut equalized = Mat::default();
    imgproc::equalize_hist(&green, &mut equalized)?;
    let green_bytes = green.data_bytes()?.to_vec();
    let equalized_bytes = equalized.data_bytes()?.to_vec();
    let mut blended = Mat::new_rows_cols_with_default(
        green.rows(),
        green.cols(),
        core::CV_8UC1,
        Scalar::all(0.0),
    )?;
    {
        let out = blended.data_bytes_mut()?;
        for (index, (&g, &g2)) in green_bytes.iter().zip(equalized_bytes.iter()).enumerate() {
            let value = f32::from(g) * (1.0 - GREEN_STRENGTH) + f32::from(g2) * GREEN_STRENGTH;
            out[index] = value.clamp(0.0, 255.0) as u8;
        }
    }
    let mut result_channels = Vector::<Mat>::new();
    result_channels.push(channels.get(0)?);
    result_channels.push(blended);
    result_channels.push(channels.get(2)?);
    let mut merged = Mat::default();
    core::merge(&result_channels, &mut merged)?;
    Ok(merged)
}

/// Upstream `fov_crop_box(fov, margin=0.01)`: square box around the FOV bounding box, clipped.
fn fov_crop_box(fov: &Mat) -> Result<(i32, i32, i32, i32), Stage2PreprocessError> {
    let width = fov.cols();
    let height = fov.rows();
    let bytes = fov.data_bytes()?;
    let mut x_min = i32::MAX;
    let mut x_max = -1;
    let mut y_min = i32::MAX;
    let mut y_max = -1;
    for row in 0..height {
        for col in 0..width {
            if bytes[(row * width + col) as usize] > 0 {
                x_min = x_min.min(col);
                x_max = x_max.max(col);
                y_min = y_min.min(row);
                y_max = y_max.max(row);
            }
        }
    }
    if x_max < 0 {
        return Ok((0, 0, width, height));
    }
    let margin = ((x_max - x_min).max(y_max - y_min) as f64 * FOV_MARGIN) as i32;
    let x1 = (x_min - margin).max(0);
    let y1 = (y_min - margin).max(0);
    let x2 = (x_max + margin).min(width);
    let y2 = (y_max + margin).min(height);
    let side = (x2 - x1).max(y2 - y1);
    let cx = (x1 + x2) / 2;
    let cy = (y1 + y2) / 2;
    let x1 = (cx - side / 2).max(0);
    let y1 = (cy - side / 2).max(0);
    let x2 = (x1 + side).min(width);
    let y2 = (y1 + side).min(height);
    Ok((x1, y1, x2, y2))
}

fn mat_to_binary(mat: &Mat) -> Result<Vec<u8>, Stage2PreprocessError> {
    Ok(mat.data_bytes()?.to_vec())
}

/// Upstream TTA (`predict_tta_batch`): mean of sigmoid maps over identity, horizontal, vertical and both flips.
pub fn vessel_tta_probabilities<E: From<Stage2PreprocessError>>(
    mut logits_for_view: impl FnMut(&[f32]) -> Result<Vec<f32>, E>,
    tensor: &[f32],
) -> Result<Vec<f32>, E> {
    let size = VESSEL_INPUT_SIZE as usize;
    let views = [(false, false), (true, false), (false, true), (true, true)];
    let mut total: Option<Vec<f32>> = None;
    for (flip_width, flip_height) in views {
        let input = flip_planes_hv(tensor, 3, size, flip_width, flip_height);
        let logits = logits_for_view(&input)?;
        let probabilities = sigmoid_map(&logits)?;
        let probabilities = flip_planes_hv(&probabilities, 1, size, flip_width, flip_height);
        total = Some(match total {
            None => probabilities,
            Some(running) => running
                .iter()
                .zip(probabilities.iter())
                .map(|(a, b)| a + b)
                .collect(),
        });
    }
    Ok(total
        .unwrap_or_default()
        .into_iter()
        .map(|value| value / 4.0)
        .collect())
}

fn sigmoid_map(logits: &[f32]) -> Result<Vec<f32>, Stage2PreprocessError> {
    if logits.len() != VESSEL_INPUT_SIZE as usize * VESSEL_INPUT_SIZE as usize
        || logits.iter().any(|value| !value.is_finite())
    {
        return Err(Stage2PreprocessError::InvalidLogits);
    }
    Ok(logits
        .iter()
        .map(|&logit| 1.0 / (1.0 + (-logit).exp()))
        .collect())
}

/// Upstream `to_original_resolution` and `postprocess_mask`: restore to the source frame, threshold `>= 0.46`.
pub fn restore_stage2_vessel_mask(
    probabilities: &[f32],
    model_fov: &[u8],
    transform: &Stage2SpatialTransform,
    frame_fov: &[u8],
    original_width: u32,
    original_height: u32,
    threshold: f32,
) -> Result<Vec<u8>, Stage2PreprocessError> {
    let Stage2SpatialTransform::VesselFovSquare {
        crop,
        model_size,
        fov_policy,
    } = transform
    else {
        return Err(Stage2PreprocessError::InvalidTransform);
    };
    let size = VESSEL_INPUT_SIZE as usize;
    if *model_size != VESSEL_INPUT_SIZE as u32
        || fov_policy != VESSEL_FOV_POLICY
        || probabilities.len() != size * size
        || model_fov.len() != size * size
        || frame_fov.len() != (original_width as usize) * (original_height as usize)
        || probabilities
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        || !threshold.is_finite()
    {
        return Err(Stage2PreprocessError::InvalidLogits);
    }
    if crop.width == 0
        || crop.height == 0
        || crop
            .x
            .checked_add(crop.width)
            .is_none_or(|end| end > original_width)
        || crop
            .y
            .checked_add(crop.height)
            .is_none_or(|end| end > original_height)
    {
        return Err(Stage2PreprocessError::InvalidTransform);
    }
    let masked: Vec<f32> = probabilities
        .iter()
        .zip(model_fov.iter())
        .map(|(&p, &f)| p * f32::from(f))
        .collect();
    let masked_mat = Mat::from_slice(&masked)?;
    let model_map = masked_mat.reshape(1, VESSEL_INPUT_SIZE)?;
    let mut crop_map = Mat::default();
    imgproc::resize(
        &model_map,
        &mut crop_map,
        Size::new(crop.width as i32, crop.height as i32),
        0.0,
        0.0,
        imgproc::INTER_LINEAR,
    )?;
    let mut full = vec![0.0f32; (original_width as usize) * (original_height as usize)];
    for row in 0..crop.height as i32 {
        for col in 0..crop.width as i32 {
            let x = crop.x as usize + col as usize;
            let y = crop.y as usize + row as usize;
            full[y * original_width as usize + x] = *crop_map.at_2d::<f32>(row, col)?;
        }
    }
    let binary: Vec<u8> = full
        .iter()
        .zip(frame_fov.iter())
        .map(|(&p, &f)| u8::from(p >= threshold && f > 0))
        .collect();
    let binary_flat = Mat::from_slice(&binary)?;
    let binary_mat = binary_flat.reshape(1, original_height as i32)?;
    let kernel3 = imgproc::get_structuring_element(
        imgproc::MORPH_ELLIPSE,
        Size::new(3, 3),
        Point::new(-1, -1),
    )?;
    let mut opened = Mat::default();
    imgproc::morphology_ex(
        &binary_mat,
        &mut opened,
        imgproc::MORPH_OPEN,
        &kernel3,
        Point::new(-1, -1),
        1,
        core::BORDER_CONSTANT,
        imgproc::morphology_default_border_value()?,
    )?;
    let binary_mat = opened;
    let mut labels = Mat::default();
    let mut stats = Mat::default();
    let mut centroids = Mat::default();
    let count = imgproc::connected_components_with_stats(
        &binary_mat,
        &mut labels,
        &mut stats,
        &mut centroids,
        8,
        core::CV_32S,
    )?;
    let mut keep = vec![0u8; count as usize];
    for label in 1..count {
        keep[label as usize] =
            u8::from(*stats.at_2d::<i32>(label, imgproc::CC_STAT_AREA)? >= MIN_COMPONENT_AREA);
    }
    let mut output = vec![0u8; (original_width as usize) * (original_height as usize)];
    for row in 0..original_height as i32 {
        for col in 0..original_width as i32 {
            let label = *labels.at_2d::<i32>(row, col)? as usize;
            if keep[label] == 1 {
                output[row as usize * original_width as usize + col as usize] = 255;
            }
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fov_crop_box_matches_upstream_inclusive_max_quirk() {
        let mut fov = Mat::zeros(40, 60, core::CV_8UC1).unwrap().to_mat().unwrap();
        fov.roi_mut(core::Rect::new(10, 5, 20, 20))
            .unwrap()
            .set_to(&Scalar::all(1.0), &core::no_array())
            .unwrap();
        // FOV spans x 10..=29, y 5..=24. Upstream uses the inclusive max as the exclusive end,
        // so the square box drops the last FOV row/column: (10, 5, 29, 24).
        assert_eq!(fov_crop_box(&fov).unwrap(), (10, 5, 29, 24));
    }

    #[test]
    fn empty_fov_falls_back_to_full_frame_box() {
        let fov = Mat::zeros(12, 16, core::CV_8UC1).unwrap().to_mat().unwrap();
        assert_eq!(fov_crop_box(&fov).unwrap(), (0, 0, 16, 12));
    }

    #[test]
    fn vessel_restore_rejects_wrong_fov_policy_and_nonfinite_probabilities() {
        let transform = Stage2SpatialTransform::VesselFovSquare {
            crop: ImageRect {
                x: 0,
                y: 0,
                width: 4,
                height: 4,
            },
            model_size: 512,
            fov_policy: "wrong".into(),
        };
        let probabilities = vec![0.0f32; 512 * 512];
        let model_fov = vec![1u8; 512 * 512];
        let frame_fov = vec![1u8; 16];
        assert!(matches!(
            restore_stage2_vessel_mask(
                &probabilities,
                &model_fov,
                &transform,
                &frame_fov,
                4,
                4,
                0.46
            ),
            Err(Stage2PreprocessError::InvalidLogits)
        ));
    }

    #[test]
    fn vessel_restore_keeps_components_at_least_thirty_pixels() {
        let (width, height) = (40u32, 40u32);
        let transform = Stage2SpatialTransform::VesselFovSquare {
            crop: ImageRect {
                x: 0,
                y: 0,
                width,
                height,
            },
            model_size: 512,
            fov_policy: VESSEL_FOV_POLICY.into(),
        };
        let mut probabilities = vec![0.0f32; 512 * 512];
        // Model-resolution block (~ 40x40 frame pixels) well above 30 px after restore.
        for row in 100..400 {
            for col in 100..400 {
                probabilities[row * 512 + col] = 1.0;
            }
        }
        // A single positive pixel far away is removed.
        probabilities[450 * 512 + 450] = 1.0;
        let model_fov = vec![1u8; 512 * 512];
        let frame_fov = vec![1u8; (width * height) as usize];
        let mask = restore_stage2_vessel_mask(
            &probabilities,
            &model_fov,
            &transform,
            &frame_fov,
            width,
            height,
            0.46,
        )
        .unwrap();
        assert!(mask.contains(&255));
        assert_eq!(mask.len(), (width * height) as usize);
        assert!(mask.iter().all(|&pixel| pixel == 0 || pixel == 255));
    }
}
