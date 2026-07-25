use std::io::Cursor;

use exif::{In, Reader as ExifReader, Tag};
use image::{DynamicImage, ImageBuffer, ImageEncoder, ImageFormat, ImageReader, Luma};
use printpdf::{
    ImageOptimizationOptions, Mm, Op, PdfDocument, PdfPage, PdfParseOptions, PdfSaveOptions,
    PdfWarnMsg, Pt, RawImage, RawImageData, RawImageFormat, XObjectTransform,
};

pub const MDI_IMAGE_MAX_PIXELS: u64 = 50_000_000;
pub const MDI_IMAGE_MAX_SIDE: u32 = 12_000;
pub const MDI_PREVIEW_MAX_SIDE: u32 = 1_400;
pub const MDI_COPY_SOFT_LIMIT_BYTES: usize = 50 * 1024 * 1024;
pub const MDI_COPY_HARD_LIMIT_BYTES: usize = 100 * 1024 * 1024;

#[derive(Clone, Copy, Debug)]
pub struct StampPlacement {
    pub relative_x: f64,
    pub relative_y: f64,
    pub relative_width: f64,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DrugInspectionCopyError {
    DecodeImage,
    InvalidDimensions,
    StampNeedsTransparency,
    ParsePdf,
    InvalidProcessingMode,
    CopyHardLimitExceeded,
}

pub fn decode_mdi_image(
    bytes: &[u8],
    expected_format: ImageFormat,
) -> Result<DynamicImage, DrugInspectionCopyError> {
    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(expected_format);
    let dimensions = reader
        .into_dimensions()
        .map_err(|_| DrugInspectionCopyError::DecodeImage)?;
    validate_dimensions(dimensions.0, dimensions.1)?;

    let mut reader = ImageReader::new(Cursor::new(bytes));
    reader.set_format(expected_format);
    let image = reader
        .decode()
        .map_err(|_| DrugInspectionCopyError::DecodeImage)?;
    let image = apply_exif_orientation(image, bytes);
    validate_dimensions(image.width(), image.height())?;
    Ok(image)
}

pub fn validate_transparent_stamp(bytes: &[u8]) -> Result<DynamicImage, DrugInspectionCopyError> {
    let image = decode_mdi_image(bytes, ImageFormat::Png)?;
    if image.to_rgba8().pixels().all(|pixel| pixel.0[3] == u8::MAX) {
        return Err(DrugInspectionCopyError::StampNeedsTransparency);
    }
    Ok(image)
}

pub fn process_image(
    image: DynamicImage,
    mode: &str,
) -> Result<DynamicImage, DrugInspectionCopyError> {
    match mode {
        "none" => Ok(image),
        "color_enhance" => Ok(image.blur(0.45).adjust_contrast(4.0)),
        "black_white_enhance" => {
            let gray = image.to_luma8();
            let local_mean = DynamicImage::ImageLuma8(gray.clone()).blur(7.0).to_luma8();
            let mut output = ImageBuffer::<Luma<u8>, Vec<u8>>::new(gray.width(), gray.height());
            for (x, y, pixel) in output.enumerate_pixels_mut() {
                let source = i16::from(gray.get_pixel(x, y).0[0]);
                let threshold = i16::from(local_mean.get_pixel(x, y).0[0]) - 7;
                pixel.0[0] = if source >= threshold { 255 } else { 0 };
            }
            Ok(DynamicImage::ImageLuma8(output).blur(0.25))
        }
        _ => Err(DrugInspectionCopyError::InvalidProcessingMode),
    }
}

pub fn generate_customer_pdf(
    original_bytes: &[u8],
    original_content_type: &str,
    processing_mode: &str,
    stamp_bytes: &[u8],
    placement: StampPlacement,
) -> Result<Vec<u8>, DrugInspectionCopyError> {
    let stamp = validate_transparent_stamp(stamp_bytes)?;
    let mut warnings = Vec::<PdfWarnMsg>::new();
    let mut document = match original_content_type {
        "application/pdf" => PdfDocument::parse(
            original_bytes,
            &PdfParseOptions {
                fail_on_error: true,
            },
            &mut warnings,
        )
        .map_err(|_| DrugInspectionCopyError::ParsePdf)?,
        "image/jpeg" | "image/png" => {
            let format = if original_content_type == "image/jpeg" {
                ImageFormat::Jpeg
            } else {
                ImageFormat::Png
            };
            let processed =
                process_image(decode_mdi_image(original_bytes, format)?, processing_mode)?;
            image_document(processed)
        }
        _ => return Err(DrugInspectionCopyError::DecodeImage),
    };
    add_stamp_to_all_pages(&mut document, stamp, placement);
    let bytes = document.save(
        &PdfSaveOptions {
            image_optimization: Some(ImageOptimizationOptions {
                quality: Some(0.85),
                max_image_size: None,
                ..ImageOptimizationOptions::default()
            }),
            ..PdfSaveOptions::default()
        },
        &mut warnings,
    );
    if bytes.len() > MDI_COPY_HARD_LIMIT_BYTES {
        return Err(DrugInspectionCopyError::CopyHardLimitExceeded);
    }
    Ok(bytes)
}

pub fn encode_preview_png(image: &DynamicImage) -> Result<Vec<u8>, DrugInspectionCopyError> {
    let rgba = image.to_rgba8();
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            rgba.as_raw(),
            rgba.width(),
            rgba.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|_| DrugInspectionCopyError::DecodeImage)?;
    Ok(bytes)
}

pub fn generate_image_preview(
    original_bytes: &[u8],
    original_content_type: &str,
    processing_mode: &str,
) -> Result<(Vec<u8>, u32, u32), DrugInspectionCopyError> {
    let format = match original_content_type {
        "image/jpeg" => ImageFormat::Jpeg,
        "image/png" => ImageFormat::Png,
        _ => return Err(DrugInspectionCopyError::DecodeImage),
    };
    let processed = process_image(decode_mdi_image(original_bytes, format)?, processing_mode)?
        .thumbnail(MDI_PREVIEW_MAX_SIDE, MDI_PREVIEW_MAX_SIDE);
    let width = processed.width();
    let height = processed.height();
    Ok((encode_preview_png(&processed)?, width, height))
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), DrugInspectionCopyError> {
    if width == 0
        || height == 0
        || width > MDI_IMAGE_MAX_SIDE
        || height > MDI_IMAGE_MAX_SIDE
        || u64::from(width) * u64::from(height) > MDI_IMAGE_MAX_PIXELS
    {
        Err(DrugInspectionCopyError::InvalidDimensions)
    } else {
        Ok(())
    }
}

fn apply_exif_orientation(image: DynamicImage, bytes: &[u8]) -> DynamicImage {
    let orientation = ExifReader::new()
        .read_from_container(&mut Cursor::new(bytes))
        .ok()
        .and_then(|exif| {
            exif.get_field(Tag::Orientation, In::PRIMARY)
                .and_then(|field| field.value.get_uint(0))
        })
        .unwrap_or(1);
    match orientation {
        2 => image.fliph(),
        3 => image.rotate180(),
        4 => image.flipv(),
        5 => image.rotate90().fliph(),
        6 => image.rotate90(),
        7 => image.rotate270().fliph(),
        8 => image.rotate270(),
        _ => image,
    }
}

fn image_document(image: DynamicImage) -> PdfDocument {
    const PAGE_WIDTH_PT: f32 = 595.28;
    const PAGE_HEIGHT_PT: f32 = 841.89;
    const MARGIN_PT: f32 = 20.0;
    let rgba = image.to_rgba8();
    let width = rgba.width() as f32;
    let height = rgba.height() as f32;
    let scale = ((PAGE_WIDTH_PT - 2.0 * MARGIN_PT) / width)
        .min((PAGE_HEIGHT_PT - 2.0 * MARGIN_PT) / height);
    let rendered_width = width * scale;
    let rendered_height = height * scale;
    let mut document = PdfDocument::new("药检单客户分发副本");
    let image_id = document.add_image(&RawImage {
        pixels: RawImageData::U8(rgba.into_raw()),
        width: width as usize,
        height: height as usize,
        data_format: RawImageFormat::RGBA8,
        tag: Vec::new(),
    });
    let page = PdfPage::new(
        Mm(210.0),
        Mm(297.0),
        vec![Op::UseXobject {
            id: image_id,
            transform: XObjectTransform {
                translate_x: Some(Pt((PAGE_WIDTH_PT - rendered_width) / 2.0)),
                translate_y: Some(Pt((PAGE_HEIGHT_PT - rendered_height) / 2.0)),
                scale_x: Some(scale),
                scale_y: Some(scale),
                dpi: Some(72.0),
                ..XObjectTransform::default()
            },
        }],
    );
    document.with_pages(vec![page]);
    document
}

fn add_stamp_to_all_pages(
    document: &mut PdfDocument,
    stamp: DynamicImage,
    placement: StampPlacement,
) {
    let rgba = stamp.to_rgba8();
    let pixel_width = rgba.width() as f32;
    let pixel_height = rgba.height() as f32;
    let stamp_id = document.add_image(&RawImage {
        pixels: RawImageData::U8(rgba.into_raw()),
        width: pixel_width as usize,
        height: pixel_height as usize,
        data_format: RawImageFormat::RGBA8,
        tag: Vec::new(),
    });
    for page in &mut document.pages {
        let page_width = page.media_box.width.0;
        let page_height = page.media_box.height.0;
        let rendered_width = page_width * placement.relative_width as f32;
        let scale = rendered_width / pixel_width;
        let rendered_height = pixel_height * scale;
        let x = page_width * placement.relative_x as f32;
        let y = page_height * (1.0 - placement.relative_y as f32) - rendered_height;
        page.ops.push(Op::UseXobject {
            id: stamp_id.clone(),
            transform: XObjectTransform {
                translate_x: Some(Pt(x)),
                translate_y: Some(Pt(y.max(0.0))),
                scale_x: Some(scale),
                scale_y: Some(scale),
                dpi: Some(72.0),
                ..XObjectTransform::default()
            },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    fn png(width: u32, height: u32, alpha: u8) -> Vec<u8> {
        let image = ImageBuffer::from_pixel(width, height, Rgba([220, 20, 60, alpha]));
        encode_preview_png(&DynamicImage::ImageRgba8(image)).expect("PNG should encode")
    }

    #[test]
    fn requires_a_transparent_stamp_and_generates_a_parseable_pdf() {
        assert_eq!(
            validate_transparent_stamp(&png(20, 20, 255)),
            Err(DrugInspectionCopyError::StampNeedsTransparency)
        );
        let original = png(120, 200, 255);
        let output = generate_customer_pdf(
            &original,
            "image/png",
            "black_white_enhance",
            &png(40, 30, 90),
            StampPlacement {
                relative_x: 0.7,
                relative_y: 0.75,
                relative_width: 0.2,
            },
        )
        .expect("customer PDF should generate");
        assert!(output.starts_with(b"%PDF-"));
        let mut warnings = Vec::new();
        let parsed = PdfDocument::parse(&output, &PdfParseOptions::default(), &mut warnings)
            .expect("generated PDF should parse");
        assert_eq!(parsed.page_count(), 1);
    }

    #[test]
    fn rejects_dimensions_over_the_pixel_or_side_limits() {
        assert_eq!(
            validate_dimensions(12_001, 10),
            Err(DrugInspectionCopyError::InvalidDimensions)
        );
        assert_eq!(
            validate_dimensions(10_000, 5_001),
            Err(DrugInspectionCopyError::InvalidDimensions)
        );
    }

    #[test]
    fn preview_uses_real_processing_and_is_bounded() {
        let original = png(2_000, 1_000, 255);
        let (bytes, width, height) =
            generate_image_preview(&original, "image/png", "black_white_enhance")
                .expect("preview should generate");
        assert!(bytes.starts_with(b"\x89PNG"));
        assert_eq!((width, height), (1_400, 700));
        assert_eq!(
            generate_image_preview(&original, "image/png", "unknown"),
            Err(DrugInspectionCopyError::InvalidProcessingMode)
        );
    }
}
