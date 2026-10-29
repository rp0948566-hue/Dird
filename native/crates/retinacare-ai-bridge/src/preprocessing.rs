//! Stage I HierarchiRetina v1.0 fundus preprocessing.
use image::{ImageFormat, ImageReader};
use opencv::{
    core::{self, Mat, MatTraitConst, MatTraitConstManual, Scalar, Size, Vector},
    imgcodecs, imgproc,
    prelude::{CLAHETrait, MatExprTraitConst, MatTrait},
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Cursor, Read},
    path::Path,
};
use thiserror::Error;

pub const PREPROCESSING_VERSION: &str = "hierarchiretina-v1.0-preprocess_gate";
pub const STAGE1_IMAGE_SIZE: i32 = 768;
const MAX_ENCODED_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
const IMAGE_MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const IMAGE_STD: [f32; 3] = [0.229, 0.224, 0.225];

#[derive(Debug, Error)]
pub enum PreprocessError {
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
}

/// Where the square Stage I window sits in the source image. The window is `side` x `side` pixels with its
/// top-left corner at (`x`, `y`); parts outside the image (negative `x`/`y`, or past the far edges) are
/// zero padding. It is then resized to 768 x 768. Recorded so a model-space result can be mapped back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage1Geometry {
    pub x: i32,
    pub y: i32,
    pub side: i32,
}

/// The part of the Stage I window inside the image, with its place in the 768-pixel model frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisibleWindow {
    pub crop_x: u32,
    pub crop_y: u32,
    pub crop_width: u32,
    pub crop_height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub resized_width: u32,
    pub resized_height: u32,
}

impl Stage1Geometry {
    /// The part of the window that lies inside an image of `width` x `height`; offsets and resized sizes
    /// are in 768-pixel model space (rounded to whole model pixels, error under one pixel).
    pub fn visible(&self, width: u32, height: u32) -> Option<VisibleWindow> {
        let side = i64::from(self.side);
        let (w, h) = (i64::from(width), i64::from(height));
        let (x0, y0) = (i64::from(self.x).max(0), i64::from(self.y).max(0));
        let (x1, y1) = (
            (i64::from(self.x) + side).min(w),
            (i64::from(self.y) + side).min(h),
        );
        if side <= 0 || x1 <= x0 || y1 <= y0 {
            return None;
        }
        let k = f64::from(STAGE1_IMAGE_SIZE) / side as f64;
        let model = |v: i64| ((v - i64::from(self.x)) as f64 * k).round() as i64;
        let model_y = |v: i64| ((v - i64::from(self.y)) as f64 * k).round() as i64;
        let (ox, oy) = (model(x0), model_y(y0));
        let (ex, ey) = (model(x1), model_y(y1));
        if ex <= ox
            || ey <= oy
            || ox < 0
            || oy < 0
            || ex > i64::from(STAGE1_IMAGE_SIZE)
            || ey > i64::from(STAGE1_IMAGE_SIZE)
        {
            return None;
        }
        Some(VisibleWindow {
            crop_x: x0 as u32,
            crop_y: y0 as u32,
            crop_width: (x1 - x0) as u32,
            crop_height: (y1 - y0) as u32,
            offset_x: ox as u32,
            offset_y: oy as u32,
            resized_width: (ex - ox) as u32,
            resized_height: (ey - oy) as u32,
        })
    }
}

#[derive(Debug)]
pub struct PreprocessedStage1Image {
    pub image_sha256: String,
    pub original_width: u32,
    pub original_height: u32,
    pub geometry: Stage1Geometry,
    /// Contiguous float32 RGB tensor in CHW order (implicit batch size 1).
    pub tensor: Vec<f32>,
    pub preprocessing_version: &'static str,
}

pub fn preprocess_stage1_image(path: &Path) -> Result<PreprocessedStage1Image, PreprocessError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(PreprocessError::DecodeFailed);
    }
    if metadata.len() > MAX_ENCODED_BYTES as u64 {
        return Err(PreprocessError::EncodedTooLarge {
            limit: MAX_ENCODED_BYTES,
        });
    }
    let mut encoded = Vec::with_capacity(metadata.len() as usize);
    File::open(path)?
        .take(MAX_ENCODED_BYTES as u64 + 1)
        .read_to_end(&mut encoded)?;
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(PreprocessError::EncodedTooLarge {
            limit: MAX_ENCODED_BYTES,
        });
    }
    preprocess_stage1_bytes(&encoded)
}

pub fn preprocess_stage1_bytes(encoded: &[u8]) -> Result<PreprocessedStage1Image, PreprocessError> {
    if encoded.len() > MAX_ENCODED_BYTES {
        return Err(PreprocessError::EncodedTooLarge {
            limit: MAX_ENCODED_BYTES,
        });
    }
    let reader = ImageReader::new(Cursor::new(encoded)).with_guessed_format()?;
    if !matches!(reader.format(), Some(ImageFormat::Jpeg | ImageFormat::Png)) {
        return Err(PreprocessError::UnsupportedFormat);
    }
    let (width, height) = reader.into_dimensions()?;
    ensure_dimensions_within_limits(width, height)?;

    let image_sha256 = hex::encode(Sha256::digest(encoded));
    let source = imgcodecs::imdecode(&Vector::from_slice(encoded), imgcodecs::IMREAD_COLOR)?;
    if source.empty() {
        return Err(PreprocessError::DecodeFailed);
    }
    if source.depth() != core::CV_8U || source.channels() != 3 {
        return Err(PreprocessError::UnsupportedDecodedImage);
    }
    let (processed, geometry) = preprocess_gate_bgr(&source)?;
    let tensor = normalize_rgb_to_chw(&processed)?;
    Ok(PreprocessedStage1Image {
        image_sha256,
        original_width: width,
        original_height: height,
        geometry,
        tensor,
        preprocessing_version: PREPROCESSING_VERSION,
    })
}

fn ensure_dimensions_within_limits(width: u32, height: u32) -> Result<(), PreprocessError> {
    if width == 0 || height == 0 {
        return Err(PreprocessError::InvalidDimensions);
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > MAX_IMAGE_PIXELS {
        return Err(PreprocessError::ImageTooLarge {
            pixels,
            limit: MAX_IMAGE_PIXELS,
        });
    }
    Ok(())
}

fn preprocess_gate_bgr(source: &Mat) -> Result<(Mat, Stage1Geometry), PreprocessError> {
    let mut gray = Mat::default();
    imgproc::cvt_color_def(source, &mut gray, imgproc::COLOR_BGR2GRAY)?;
    let mut thresholded = Mat::default();
    imgproc::threshold(
        &gray,
        &mut thresholded,
        (0.04_f64 * 255.0) as i32 as f64,
        255.0,
        imgproc::THRESH_BINARY,
    )?;
    let kernel = imgproc::get_structuring_element(
        imgproc::MORPH_RECT,
        Size::new(5, 5),
        core::Point::new(-1, -1),
    )?;
    let mut closed = Mat::default();
    imgproc::morphology_ex_def(&thresholded, &mut closed, imgproc::MORPH_CLOSE, &kernel)?;
    let mut mask = Mat::default();
    imgproc::morphology_ex_def(&closed, &mut mask, imgproc::MORPH_OPEN, &kernel)?;
    let bounds = detect_retinal_bounds(&mask)?;
    let geometry = square_window(source.rows(), source.cols(), bounds);
    let mut image = square_retinal_crop(source, bounds)?;
    let interpolation = if image.rows().max(image.cols()) > STAGE1_IMAGE_SIZE {
        imgproc::INTER_AREA
    } else {
        imgproc::INTER_LANCZOS4
    };
    let mut resized = Mat::default();
    imgproc::resize(
        &image,
        &mut resized,
        Size::new(STAGE1_IMAGE_SIZE, STAGE1_IMAGE_SIZE),
        0.0,
        0.0,
        interpolation,
    )?;
    image = green_channel_boost(&resized)?;
    image = ben_graham(&image)?;
    image = lab_clahe(&image)?;
    Ok((unsharp_mask(&image)?, geometry))
}

/// The same window `square_retinal_crop` cuts, as plain numbers.
fn square_window(height: i32, width: i32, bounds: Option<core::Rect>) -> Stage1Geometry {
    let (side, cx, cy) = if let Some(bounds) = bounds {
        (
            bounds.width.max(bounds.height),
            bounds.x + bounds.width / 2,
            bounds.y + bounds.height / 2,
        )
    } else {
        (height.min(width), width / 2, height / 2)
    };
    Stage1Geometry {
        x: cx - side / 2,
        y: cy - side / 2,
        side,
    }
}

fn detect_retinal_bounds(mask: &Mat) -> Result<Option<core::Rect>, PreprocessError> {
    let (width, height) = (mask.cols(), mask.rows());
    let mut labels = Mat::default();
    let mut stats = Mat::default();
    let mut centroids = Mat::default();
    let count = imgproc::connected_components_with_stats(
        mask,
        &mut labels,
        &mut stats,
        &mut centroids,
        8,
        core::CV_32S,
    )?;
    let mut best = None;
    let mut best_area = -1;
    for label in 1..count {
        let area = *stats.at_2d::<i32>(label, imgproc::CC_STAT_AREA)?;
        if area > best_area {
            best = Some(label);
            best_area = area;
        }
    }
    let Some(label) = best else { return Ok(None) };
    let x = *stats.at_2d::<i32>(label, imgproc::CC_STAT_LEFT)?;
    let y = *stats.at_2d::<i32>(label, imgproc::CC_STAT_TOP)?;
    let box_width = *stats.at_2d::<i32>(label, imgproc::CC_STAT_WIDTH)?;
    let box_height = *stats.at_2d::<i32>(label, imgproc::CC_STAT_HEIGHT)?;
    if f64::from(box_width) * f64::from(box_height) < 0.2 * f64::from(width) * f64::from(height) {
        return Ok(None);
    }
    let x1 = (x - 8).max(0);
    let y1 = (y - 8).max(0);
    let x2 = (x + box_width + 8).min(width);
    let y2 = (y + box_height + 8).min(height);
    Ok(Some(core::Rect::new(x1, y1, x2 - x1, y2 - y1)))
}

fn square_retinal_crop(source: &Mat, bounds: Option<core::Rect>) -> Result<Mat, PreprocessError> {
    let (height, width) = (source.rows(), source.cols());
    let (side, cx, cy) = if let Some(bounds) = bounds {
        (
            bounds.width.max(bounds.height),
            bounds.x + bounds.width / 2,
            bounds.y + bounds.height / 2,
        )
    } else {
        (height.min(width), width / 2, height / 2)
    };
    let x1 = cx - side / 2;
    let y1 = cy - side / 2;
    let x2 = x1 + side;
    let y2 = y1 + side;
    let left = (-x1).max(0);
    let top = (-y1).max(0);
    let right = (x2 - width).max(0);
    let bottom = (y2 - height).max(0);
    let rect = core::Rect::new(
        x1.max(0),
        y1.max(0),
        side - left - right,
        side - top - bottom,
    );
    let view = Mat::roi(source, rect)?;
    let mut crop = Mat::default();
    if left > 0 || top > 0 || right > 0 || bottom > 0 {
        core::copy_make_border(
            &view,
            &mut crop,
            top,
            bottom,
            left,
            right,
            core::BORDER_CONSTANT,
            Scalar::all(0.0),
        )?;
    } else {
        view.copy_to(&mut crop)?;
    }
    Ok(crop)
}

fn green_channel_boost(source: &Mat) -> Result<Mat, PreprocessError> {
    let mut channels = Vector::<Mat>::new();
    core::split(source, &mut channels)?;
    let green = channels.get(1)?;
    let mut equalized = Mat::default();
    let mut clahe = imgproc::create_clahe(3.0, Size::new(8, 8))?;
    clahe.apply(&green, &mut equalized)?;
    let mut boosted = Mat::default();
    core::add_weighted(&green, 0.75, &equalized, 0.25, 0.0, &mut boosted, -1)?;
    channels.set(1, boosted)?;
    let mut result = Mat::default();
    core::merge(&channels, &mut result)?;
    Ok(result)
}

fn ben_graham(source: &Mat) -> Result<Mat, PreprocessError> {
    let sigma = (10.0_f64).max((source.rows() as f64 * 0.045).trunc());
    let mut blurred = Mat::default();
    imgproc::gaussian_blur_def(source, &mut blurred, Size::new(0, 0), sigma)?;
    let mut result = Mat::default();
    core::add_weighted(source, 4.0, &blurred, -4.0, 128.0, &mut result, -1)?;
    let mut mask = Mat::zeros(source.rows(), source.cols(), source.typ())?.to_mat()?;
    let radius = (source.rows().min(source.cols()) as f64 * 0.49).trunc() as i32;
    imgproc::circle(
        &mut mask,
        core::Point::new(source.cols() / 2, source.rows() / 2),
        radius,
        Scalar::all(255.0),
        imgproc::FILLED,
        imgproc::LINE_8,
        0,
    )?;
    let mut outside = Mat::default();
    core::bitwise_not_def(&mask, &mut outside)?;
    result.set_to(&Scalar::all(128.0), &outside)?;
    Ok(result)
}

fn lab_clahe(source: &Mat) -> Result<Mat, PreprocessError> {
    let mut lab = Mat::default();
    imgproc::cvt_color_def(source, &mut lab, imgproc::COLOR_BGR2Lab)?;
    let mut channels = Vector::<Mat>::new();
    core::split(&lab, &mut channels)?;
    let mut equalized = Mat::default();
    let mut clahe = imgproc::create_clahe(2.2, Size::new(8, 8))?;
    clahe.apply(&channels.get(0)?, &mut equalized)?;
    channels.set(0, equalized)?;
    core::merge(&channels, &mut lab)?;
    let mut result = Mat::default();
    imgproc::cvt_color_def(&lab, &mut result, imgproc::COLOR_Lab2BGR)?;
    Ok(result)
}

fn unsharp_mask(source: &Mat) -> Result<Mat, PreprocessError> {
    let mut blurred = Mat::default();
    imgproc::gaussian_blur_def(source, &mut blurred, Size::new(0, 0), 1.2)?;
    let mut result = Mat::default();
    core::add_weighted(source, 1.3, &blurred, -0.3, 0.0, &mut result, -1)?;
    Ok(result)
}

fn normalize_rgb_to_chw(bgr: &Mat) -> Result<Vec<f32>, PreprocessError> {
    let mut rgb = Mat::default();
    imgproc::cvt_color_def(bgr, &mut rgb, imgproc::COLOR_BGR2RGB)?;
    let pixels = rgb.data_bytes()?;
    let plane_size = (STAGE1_IMAGE_SIZE * STAGE1_IMAGE_SIZE) as usize;
    if pixels.len() != plane_size * 3 {
        return Err(PreprocessError::InvalidOutputBuffer);
    }
    let mut tensor = vec![0.0; plane_size * 3];
    for (index, pixel) in pixels.as_chunks::<3>().0.iter().enumerate() {
        for channel in 0..3 {
            tensor[channel * plane_size + index] =
                (f32::from(pixel[channel]) / 255.0 - IMAGE_MEAN[channel]) / IMAGE_STD[channel];
        }
    }
    Ok(tensor)
}

#[cfg(test)]
mod tests {
    #[test]
    fn square_window_matches_the_crop_for_centred_padded_and_boundless_images() {
        // Retina box 100 wide x 60 high at (50, 20), 8 px margin is already inside `bounds`.
        let b = core::Rect::new(50, 20, 100, 60);
        let g = square_window(200, 300, Some(b));
        assert_eq!(
            g,
            Stage1Geometry {
                x: 50,
                y: 0,
                side: 100
            }
        ); // centre (100,50): x1=50, y1=0
        // The window reaches past the top edge: negative y means zero padding on top.
        let b = core::Rect::new(10, 0, 80, 100);
        let g = square_window(100, 100, Some(b));
        assert_eq!(g.side, 100);
        assert!(g.x <= 10 && g.y == 0);
        // No retina found: the largest centred square.
        let g = square_window(480, 640, None);
        assert_eq!(
            g,
            Stage1Geometry {
                x: 80,
                y: 0,
                side: 480
            }
        );
        // The window and the pixels square_retinal_crop returns have the same size.
        let src =
            Mat::new_rows_cols_with_default(120, 200, core::CV_8UC3, Scalar::all(10.0)).unwrap();
        let bounds = Some(core::Rect::new(30, 10, 140, 100));
        let crop = square_retinal_crop(&src, bounds).unwrap();
        let g = square_window(120, 200, bounds);
        assert_eq!((crop.rows(), crop.cols()), (g.side, g.side));
    }

    #[test]
    fn visible_part_of_the_window_maps_into_model_space_without_overflow() {
        // Window larger than the image on the left and top: the visible part starts inside the model frame.
        let g = Stage1Geometry {
            x: -50,
            y: -10,
            side: 200,
        };
        let v = g.visible(150, 190).unwrap();
        let (ox, oy, rw, rh) = (v.offset_x, v.offset_y, v.resized_width, v.resized_height);
        assert_eq!(
            (v.crop_x, v.crop_y, v.crop_width, v.crop_height),
            (0, 0, 150, 190)
        );
        let k: f64 = 768.0 / 200.0;
        assert_eq!(
            (ox, oy),
            ((50.0 * k).round() as u32, (10.0 * k).round() as u32)
        );
        assert!(ox + rw <= 768 && oy + rh <= 768);
        assert_eq!(
            ox + rw,
            768,
            "the visible part reaches the far model edge when it reaches the window edge"
        );
        // Window entirely inside: the visible part fills the whole model frame.
        let g = Stage1Geometry {
            x: 10,
            y: 20,
            side: 100,
        };
        assert_eq!(
            g.visible(500, 500),
            Some(VisibleWindow {
                crop_x: 10,
                crop_y: 20,
                crop_width: 100,
                crop_height: 100,
                offset_x: 0,
                offset_y: 0,
                resized_width: 768,
                resized_height: 768
            })
        );
        // Window with no overlap, or a degenerate one.
        assert_eq!(
            Stage1Geometry {
                x: 600,
                y: 0,
                side: 100
            }
            .visible(500, 500),
            None
        );
        assert_eq!(
            Stage1Geometry {
                x: 0,
                y: 0,
                side: 0
            }
            .visible(500, 500),
            None
        );
    }
    use super::*;
    use opencv::{core, imgcodecs, prelude::*};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

    fn temp_path(extension: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "retinacare-preprocess-{}-{}.{}",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed),
            extension
        ))
    }

    fn solid_png() -> Vec<u8> {
        let image = core::Mat::new_rows_cols_with_default(
            96,
            96,
            core::CV_8UC3,
            core::Scalar::new(35.0, 90.0, 145.0, 0.0),
        )
        .unwrap();
        let mut encoded = core::Vector::<u8>::new();
        imgcodecs::imencode(".png", &image, &mut encoded, &core::Vector::new()).unwrap();
        encoded.to_vec()
    }

    #[test]
    fn opencv_binding_smoke_reports_installed_library_version() {
        let info = opencv::core::get_build_information().expect("OpenCV core must initialize");
        assert!(info.contains("4.10.0"), "unexpected OpenCV version");
    }

    #[test]
    fn preprocesses_valid_png_into_finite_nchw_imagenet_tensor() {
        let path = temp_path("png");
        fs::write(&path, solid_png()).unwrap();
        let image = preprocess_stage1_image(&path).unwrap();
        fs::remove_file(path).unwrap();

        assert_eq!((image.original_width, image.original_height), (96, 96));
        assert_eq!(image.tensor.len(), 3 * 768 * 768);
        assert!(image.tensor.iter().all(|value| value.is_finite()));
        assert_eq!(image.image_sha256.len(), 64);
        assert_eq!(image.preprocessing_version, PREPROCESSING_VERSION);
    }

    #[test]
    fn rejects_corrupt_image_bytes() {
        let path = temp_path("png");
        fs::write(&path, b"not a jpeg or png").unwrap();
        assert!(preprocess_stage1_image(&path).is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_encoded_files_above_the_configured_limit_before_reading_them() {
        let path = temp_path("png");
        fs::File::create(&path)
            .unwrap()
            .set_len((MAX_ENCODED_BYTES + 1) as u64)
            .unwrap();
        assert!(matches!(
            preprocess_stage1_image(&path),
            Err(PreprocessError::EncodedTooLarge { .. })
        ));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_dimensions_over_the_configured_pixel_limit() {
        assert!(matches!(
            ensure_dimensions_within_limits(8_000, 8_000),
            Err(PreprocessError::ImageTooLarge { .. })
        ));
        assert!(matches!(
            ensure_dimensions_within_limits(0, 20),
            Err(PreprocessError::InvalidDimensions)
        ));
    }

    fn encode(ext: &str, image: &core::Mat) -> Vec<u8> {
        let mut encoded = core::Vector::<u8>::new();
        imgcodecs::imencode(ext, image, &mut encoded, &core::Vector::new()).unwrap();
        encoded.to_vec()
    }

    fn solid(rows: i32, cols: i32, kind: i32, value: f64) -> core::Mat {
        core::Mat::new_rows_cols_with_default(rows, cols, kind, core::Scalar::all(value)).unwrap()
    }

    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = !0u32;
        for byte in bytes {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn truncated_png_and_jpeg_are_rejected_without_a_tensor() {
        let image = solid(96, 96, core::CV_8UC3, 120.0);
        for ext in [".png", ".jpg"] {
            let full = encode(ext, &image);
            let cut = &full[..full.len() / 2];
            assert!(preprocess_stage1_bytes(cut).is_err(), "{ext} truncated");
        }
    }

    #[test]
    fn unsupported_encodings_are_rejected_as_unsupported_format() {
        let image = solid(32, 32, core::CV_8UC3, 120.0);
        for ext in [".bmp", ".tiff"] {
            assert!(
                matches!(
                    preprocess_stage1_bytes(&encode(ext, &image)),
                    Err(PreprocessError::UnsupportedFormat)
                ),
                "{ext}"
            );
        }
        assert!(matches!(
            preprocess_stage1_bytes(&[]),
            Err(PreprocessError::UnsupportedFormat | PreprocessError::Header(_))
        ));
    }

    #[test]
    fn alpha_gray_and_sixteen_bit_pngs_decode_to_a_finite_three_channel_tensor() {
        // IMREAD_COLOR folds alpha, gray and 16-bit sources into 8-bit BGR; none may panic or yield NaN.
        for (kind, value) in [
            (core::CV_8UC4, 90.0),
            (core::CV_8UC1, 90.0),
            (core::CV_16UC3, 20_000.0),
        ] {
            let bytes = encode(".png", &solid(64, 64, kind, value));
            let out = preprocess_stage1_bytes(&bytes).unwrap();
            assert_eq!(out.tensor.len(), 3 * 768 * 768);
            assert!(out.tensor.iter().all(|v| v.is_finite()), "kind {kind}");
        }
    }

    #[test]
    fn tiny_and_blank_images_do_not_crash_or_produce_non_finite_values() {
        // A 1x1 frame and an all-black frame have no detectable field of view; the gate score for such
        // input is meaningless, but preprocessing must stay finite and bounded.
        for (rows, cols, value) in [(1, 1, 128.0), (64, 64, 0.0), (64, 64, 255.0)] {
            let bytes = encode(".png", &solid(rows, cols, core::CV_8UC3, value));
            let out = preprocess_stage1_bytes(&bytes).unwrap();
            assert!(
                out.tensor.iter().all(|v| v.is_finite()),
                "{rows}x{cols} {value}"
            );
        }
    }

    #[test]
    fn header_claiming_huge_dimensions_is_rejected_before_any_pixel_decode() {
        let mut png = encode(".png", &solid(8, 8, core::CV_8UC3, 10.0));
        // IHDR data starts at byte 16 (width, height); its CRC covers bytes 12..29.
        png[16..20].copy_from_slice(&40_000u32.to_be_bytes());
        png[20..24].copy_from_slice(&40_000u32.to_be_bytes());
        let crc = crc32(&png[12..29]);
        png[29..33].copy_from_slice(&crc.to_be_bytes());
        assert!(matches!(
            preprocess_stage1_bytes(&png),
            Err(PreprocessError::ImageTooLarge { .. })
        ));
    }

    #[test]
    #[ignore = "local-only: requires an authorized image path and Python/OpenCV reference outputs"]
    fn local_fundus_matches_python_tensor_and_typed_onnx_result() {
        use crate::{
            device::ProviderRequest, model_runtime::ModelRuntime, stage1::Stage1GateDecision,
        };

        let image_path = std::env::var_os("RETINACARE_STAGE1_IMAGE")
            .expect("set RETINACARE_STAGE1_IMAGE to a local retinal image");
        let tensor_path = std::env::var_os("RETINACARE_STAGE1_REFERENCE_TENSOR")
            .expect("set RETINACARE_STAGE1_REFERENCE_TENSOR to the Python reference tensor");
        let metadata_path = std::env::var_os("RETINACARE_STAGE1_REFERENCE_JSON")
            .expect("set RETINACARE_STAGE1_REFERENCE_JSON to Python metadata");
        let reference_json: serde_json::Value =
            serde_json::from_slice(&fs::read(metadata_path).unwrap()).unwrap();
        let reference = fs::read(tensor_path).unwrap();
        assert_eq!(reference.len(), 3 * 768 * 768 * 4);
        let reference: Vec<f32> = reference
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| f32::from_le_bytes(*bytes))
            .collect();
        let prepared = preprocess_stage1_image(Path::new(&image_path)).unwrap();
        assert_eq!(prepared.image_sha256, reference_json["image_sha256"]);
        let differences: Vec<f64> = prepared
            .tensor
            .iter()
            .zip(&reference)
            .map(|(rust, python)| f64::from((*rust - *python).abs()))
            .collect();
        let max_abs = differences.iter().copied().fold(0.0_f64, f64::max);
        let rmse = (differences.iter().map(|diff| diff * diff).sum::<f64>()
            / differences.len() as f64)
            .sqrt();
        assert!(max_abs <= 1e-6, "max tensor error {max_abs}; RMSE {rmse}");

        let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../models/stage1_convnextv2l_768/manifest.json");
        let mut runtime = ModelRuntime::default();
        let loaded = runtime.load(&repo, ProviderRequest::Cpu).unwrap();
        let result = runtime
            .infer_stage1_image(&loaded.model_id, Path::new(&image_path))
            .unwrap();
        let python_logit = reference_json["torch_logit"].as_f64().unwrap() as f32;
        let logit_error = f64::from((result.gate.raw_logit - python_logit).abs());
        assert!(
            logit_error <= 1e-3,
            "PyTorch/Rust ONNX logit error {logit_error}"
        );
        let python_continues = reference_json["continue_cascade"].as_bool().unwrap();
        assert_eq!(
            result.gate.decision == Stage1GateDecision::ContinueCascade,
            python_continues
        );
        assert_eq!(result.image.as_ref().unwrap().sha256, prepared.image_sha256);
        assert_eq!(result.preprocessing_version, PREPROCESSING_VERSION);
        assert!(result.preprocessing_microseconds.unwrap() > 0);
        println!(
            "tensor max_abs={max_abs:.3e} rmse={rmse:.3e}; PyTorch/Rust logit_abs={logit_error:.3e}; route={:?}; image_sha256={}",
            result.gate.decision, prepared.image_sha256
        );
    }
}
