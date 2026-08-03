use std::error::Error;
use std::panic::{self, AssertUnwindSafe};

use image::{DynamicImage, GrayImage, RgbImage, RgbaImage};
use lopdf::{Dictionary, Document, Object, Stream};

/// Largest image flerp will decode, in pixels. Keeps a malformed `/Width` from
/// turning into a multi-gigabyte allocation.
const MAX_PIXELS: u64 = 80_000_000;

/// A raster image lifted out of a PDF and decoded to real pixels.
#[derive(Clone)]
pub struct PdfImageAsset {
    pub page: usize,
    pub width: u32,
    pub height: u32,
    pub encoding: String,
    pub color_space: String,
    pub image: DynamicImage,
}

/// An image flerp found but could not turn into pixels.
#[derive(Clone)]
pub struct SkippedImage {
    pub page: usize,
    pub width: u32,
    pub height: u32,
    pub reason: String,
}

/// Where one page starts inside [`PdfDocument::text`].
#[derive(Clone)]
pub struct PdfPage {
    pub number: usize,
    pub start_line: usize,
    pub line_count: usize,
    pub image_count: usize,
}

#[derive(Clone)]
pub struct PdfDocument {
    /// Every page's text, joined in page order. Page boundaries are tracked in
    /// `pages` rather than injected as marker lines, so search results and the
    /// structural stats stay faithful to the document.
    pub text: String,
    pub pages: Vec<PdfPage>,
    pub images: Vec<PdfImageAsset>,
    pub skipped: Vec<SkippedImage>,
    /// Set when the page tree could be read for text but not for images.
    pub image_error: Option<String>,
}

impl PdfDocument {
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Page containing `line` (0-based), as an index into `pages`.
    pub fn page_of_line(&self, line: usize) -> usize {
        match self
            .pages
            .binary_search_by(|page| page.start_line.cmp(&line))
        {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        }
    }
}

pub fn load(file_path: &str) -> Result<PdfDocument, Box<dyn Error>> {
    let page_texts = extract_page_texts(file_path)?;

    let mut text = String::new();
    let mut pages = Vec::with_capacity(page_texts.len());
    let mut line_cursor = 0usize;

    for (index, page_text) in page_texts.iter().enumerate() {
        let normalized = page_text.replace("\r\n", "\n");
        let body = normalized.trim_matches('\n');
        // An empty page still occupies the single newline we push below.
        let line_count = body.lines().count().max(1);

        pages.push(PdfPage {
            number: index + 1,
            start_line: line_cursor,
            line_count,
            image_count: 0,
        });

        text.push_str(body);
        text.push('\n');
        line_cursor += line_count;
    }

    let mut document = PdfDocument {
        text,
        pages,
        images: Vec::new(),
        skipped: Vec::new(),
        image_error: None,
    };

    // Image extraction is best-effort: a document whose text we can read is
    // still worth opening even if its page tree defeats us.
    match Document::load(file_path) {
        Ok(pdf) => collect_images(&pdf, &mut document),
        Err(error) => document.image_error = Some(error.to_string()),
    }

    Ok(document)
}

/// `pdf-extract` panics on some malformed documents. Contain that so a bad file
/// surfaces as an error instead of taking the terminal down mid-render.
fn extract_page_texts(file_path: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        pdf_extract::extract_text_by_pages(file_path)
    }));
    panic::set_hook(previous_hook);

    match outcome {
        Ok(result) => Ok(result?),
        Err(_) => Err("the PDF text layer could not be parsed".into()),
    }
}

fn collect_images(pdf: &Document, document: &mut PdfDocument) {
    let mut seen = Vec::new();

    for (page_number, page_id) in pdf.get_pages() {
        let page_number = page_number as usize;
        let Ok(raw_images) = pdf.get_page_images(page_id) else {
            continue;
        };

        for raw in raw_images {
            // The same XObject is usually shared by every page that shows it
            // (headers, logos, watermarks). Keep the first occurrence only.
            if seen.contains(&raw.id) {
                continue;
            }
            seen.push(raw.id);

            let width = raw.width.max(0) as u32;
            let height = raw.height.max(0) as u32;

            match decode_image(pdf, &raw) {
                Ok((image, encoding)) => {
                    document.images.push(PdfImageAsset {
                        page: page_number,
                        width: image.width(),
                        height: image.height(),
                        encoding,
                        color_space: raw
                            .color_space
                            .clone()
                            .unwrap_or_else(|| "unspecified".to_string()),
                        image,
                    });
                }
                Err(reason) => document.skipped.push(SkippedImage {
                    page: page_number,
                    width,
                    height,
                    reason,
                }),
            }
        }
    }

    for page in &mut document.pages {
        page.image_count = document
            .images
            .iter()
            .filter(|asset| asset.page == page.number)
            .count();
    }
}

fn decode_image(
    pdf: &Document,
    raw: &lopdf::xobject::PdfImage<'_>,
) -> Result<(DynamicImage, String), String> {
    let width = u32::try_from(raw.width).map_err(|_| "invalid width".to_string())?;
    let height = u32::try_from(raw.height).map_err(|_| "invalid height".to_string())?;
    if width == 0 || height == 0 {
        return Err("zero-sized image".to_string());
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(format!("image is too large to decode ({width}x{height})"));
    }

    let filters = raw.filters.clone().unwrap_or_default();

    let (image, encoding) = match filters.last().map(String::as_str) {
        Some("DCTDecode") => {
            let bytes = predecode(raw.origin_dict, raw.content, &filters[..filters.len() - 1])?;
            let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
                .map_err(|error| format!("JPEG decode failed: {error}"))?;
            (image, "JPEG (DCTDecode)".to_string())
        }
        Some("JPXDecode") => return Err("JPEG 2000 (JPXDecode) is not supported".to_string()),
        Some("CCITTFaxDecode") => {
            return Err("CCITT fax encoding is not supported".to_string());
        }
        Some("JBIG2Decode") => return Err("JBIG2 encoding is not supported".to_string()),
        _ => {
            let bytes = predecode(raw.origin_dict, raw.content, &filters)?;
            let image = samples_to_image(pdf, raw, &bytes, width, height)?;
            let encoding = if filters.is_empty() {
                "raw samples".to_string()
            } else {
                format!("{} samples", filters.join(" + "))
            };
            (image, encoding)
        }
    };

    Ok((apply_soft_mask(pdf, raw.origin_dict, image), encoding))
}

/// Run the decompression filters that sit in front of the pixel data.
fn predecode(dict: &Dictionary, content: &[u8], filters: &[String]) -> Result<Vec<u8>, String> {
    if filters.is_empty() {
        return Ok(content.to_vec());
    }

    let mut dict = dict.clone();
    dict.set(
        "Filter",
        Object::Array(
            filters
                .iter()
                .map(|filter| Object::Name(filter.as_bytes().to_vec()))
                .collect(),
        ),
    );

    Stream::new(dict, content.to_vec())
        .decompressed_content()
        .map_err(|error| format!("{} could not be decoded: {error}", filters.join(" + ")))
}

enum ColorSpaceKind {
    Gray,
    Rgb,
    Cmyk,
    Indexed {
        base_components: usize,
        base: Box<ColorSpaceKind>,
        lookup: Vec<u8>,
    },
    Unknown,
}

impl ColorSpaceKind {
    fn components(&self) -> Option<usize> {
        match self {
            ColorSpaceKind::Gray => Some(1),
            ColorSpaceKind::Rgb => Some(3),
            ColorSpaceKind::Cmyk => Some(4),
            ColorSpaceKind::Indexed { .. } => Some(1),
            ColorSpaceKind::Unknown => None,
        }
    }
}

fn resolve<'a>(pdf: &'a Document, object: &'a Object) -> &'a Object {
    match object {
        Object::Reference(id) => pdf.get_object(*id).unwrap_or(object),
        _ => object,
    }
}

fn color_space_of(pdf: &Document, dict: &Dictionary) -> ColorSpaceKind {
    match dict.get(b"ColorSpace") {
        Ok(object) => color_space_from_object(pdf, object),
        Err(_) => ColorSpaceKind::Unknown,
    }
}

fn color_space_from_object(pdf: &Document, object: &Object) -> ColorSpaceKind {
    match resolve(pdf, object) {
        Object::Name(name) => match name.as_slice() {
            b"DeviceGray" | b"CalGray" | b"G" => ColorSpaceKind::Gray,
            b"DeviceRGB" | b"CalRGB" | b"RGB" => ColorSpaceKind::Rgb,
            b"DeviceCMYK" | b"CMYK" => ColorSpaceKind::Cmyk,
            _ => ColorSpaceKind::Unknown,
        },
        Object::Array(items) => {
            let Some(head) = items.first().map(|item| resolve(pdf, item)) else {
                return ColorSpaceKind::Unknown;
            };
            let Ok(head) = head.as_name() else {
                return ColorSpaceKind::Unknown;
            };

            match head {
                b"ICCBased" => match icc_components(pdf, items.get(1)) {
                    Some(1) => ColorSpaceKind::Gray,
                    Some(3) => ColorSpaceKind::Rgb,
                    Some(4) => ColorSpaceKind::Cmyk,
                    _ => ColorSpaceKind::Unknown,
                },
                b"CalRGB" => ColorSpaceKind::Rgb,
                b"CalGray" => ColorSpaceKind::Gray,
                b"Indexed" | b"I" => indexed_color_space(pdf, items),
                _ => ColorSpaceKind::Unknown,
            }
        }
        _ => ColorSpaceKind::Unknown,
    }
}

fn icc_components(pdf: &Document, object: Option<&Object>) -> Option<i64> {
    let stream = resolve(pdf, object?).as_stream().ok()?;
    stream.dict.get(b"N").ok()?.as_i64().ok()
}

fn indexed_color_space(pdf: &Document, items: &[Object]) -> ColorSpaceKind {
    let Some(base) = items.get(1) else {
        return ColorSpaceKind::Unknown;
    };
    let base = color_space_from_object(pdf, base);
    let Some(base_components) = base.components() else {
        return ColorSpaceKind::Unknown;
    };
    // A palette of palettes is not a thing worth supporting.
    if matches!(base, ColorSpaceKind::Indexed { .. }) {
        return ColorSpaceKind::Unknown;
    }

    let Some(lookup) = items.get(3) else {
        return ColorSpaceKind::Unknown;
    };
    let lookup = match resolve(pdf, lookup) {
        Object::String(bytes, _) => bytes.clone(),
        Object::Stream(stream) => stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone()),
        _ => return ColorSpaceKind::Unknown,
    };

    ColorSpaceKind::Indexed {
        base_components,
        base: Box::new(base),
        lookup,
    }
}

fn samples_to_image(
    pdf: &Document,
    raw: &lopdf::xobject::PdfImage<'_>,
    data: &[u8],
    width: u32,
    height: u32,
) -> Result<DynamicImage, String> {
    // An image mask is a 1-bit stencil, not a picture. Treat it as black-on-white.
    let is_mask = raw
        .origin_dict
        .get(b"ImageMask")
        .and_then(Object::as_bool)
        .unwrap_or(false);

    let bits_per_component = if is_mask {
        1
    } else {
        u32::try_from(raw.bits_per_component.unwrap_or(8))
            .map_err(|_| "invalid bits per component".to_string())?
    };
    if !matches!(bits_per_component, 1 | 2 | 4 | 8 | 16) {
        return Err(format!("{bits_per_component} bits per component is not supported"));
    }

    let kind = if is_mask {
        ColorSpaceKind::Gray
    } else {
        color_space_of(pdf, raw.origin_dict)
    };

    let (kind, components) = match kind.components() {
        Some(components) => (kind, components),
        None => {
            let components = infer_components(data.len(), width, height, bits_per_component)?;
            let inferred = match components {
                1 => ColorSpaceKind::Gray,
                3 => ColorSpaceKind::Rgb,
                _ => ColorSpaceKind::Cmyk,
            };
            (inferred, components)
        }
    };

    let samples = read_samples(data, width, height, components, bits_per_component)?;

    let image = match kind {
        ColorSpaceKind::Gray => {
            let mut pixels = Vec::with_capacity(samples.len());
            for value in &samples {
                pixels.push(scale(*value, bits_per_component));
            }
            // `/ImageMask true` means sample 0 paints, so the stencil is inverted.
            if is_mask {
                for pixel in &mut pixels {
                    *pixel = 255 - *pixel;
                }
            }
            GrayImage::from_raw(width, height, pixels)
                .map(DynamicImage::ImageLuma8)
                .ok_or_else(|| "grayscale buffer size mismatch".to_string())?
        }
        ColorSpaceKind::Rgb => {
            let pixels: Vec<u8> = samples
                .iter()
                .map(|value| scale(*value, bits_per_component))
                .collect();
            RgbImage::from_raw(width, height, pixels)
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| "RGB buffer size mismatch".to_string())?
        }
        ColorSpaceKind::Cmyk => {
            let mut pixels = Vec::with_capacity(samples.len() / 4 * 3);
            for chunk in samples.chunks_exact(4) {
                let [c, m, y, k] = [
                    scale(chunk[0], bits_per_component),
                    scale(chunk[1], bits_per_component),
                    scale(chunk[2], bits_per_component),
                    scale(chunk[3], bits_per_component),
                ];
                let [r, g, b] = cmyk_to_rgb(c, m, y, k);
                pixels.extend_from_slice(&[r, g, b]);
            }
            RgbImage::from_raw(width, height, pixels)
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| "CMYK buffer size mismatch".to_string())?
        }
        ColorSpaceKind::Indexed {
            base_components,
            base,
            lookup,
        } => {
            let mut pixels = Vec::with_capacity(samples.len() * 3);
            for index in &samples {
                let offset = *index as usize * base_components;
                let entry = lookup
                    .get(offset..offset + base_components)
                    .unwrap_or(&[0, 0, 0, 0][..base_components]);
                let [r, g, b] = match (&*base, entry) {
                    (ColorSpaceKind::Gray, [value]) => [*value, *value, *value],
                    (ColorSpaceKind::Rgb, [r, g, b]) => [*r, *g, *b],
                    (ColorSpaceKind::Cmyk, [c, m, y, k]) => cmyk_to_rgb(*c, *m, *y, *k),
                    _ => [0, 0, 0],
                };
                pixels.extend_from_slice(&[r, g, b]);
            }
            RgbImage::from_raw(width, height, pixels)
                .map(DynamicImage::ImageRgb8)
                .ok_or_else(|| "indexed buffer size mismatch".to_string())?
        }
        ColorSpaceKind::Unknown => return Err("unrecognised colour space".to_string()),
    };

    Ok(image)
}

fn infer_components(
    byte_len: usize,
    width: u32,
    height: u32,
    bits_per_component: u32,
) -> Result<usize, String> {
    let pixels = width as usize * height as usize;
    let bits_per_pixel = byte_len
        .checked_mul(8)
        .and_then(|bits| bits.checked_div(pixels.max(1)))
        .unwrap_or(0);

    match bits_per_pixel / bits_per_component as usize {
        1 => Ok(1),
        3 => Ok(3),
        4 => Ok(4),
        _ => Err("colour space is missing and could not be inferred".to_string()),
    }
}

/// Unpack `components` samples per pixel at `bits_per_component`, honouring the
/// per-row byte alignment the PDF spec requires. Values are returned unscaled so
/// palette indices survive intact.
fn read_samples(
    data: &[u8],
    width: u32,
    height: u32,
    components: usize,
    bits_per_component: u32,
) -> Result<Vec<u16>, String> {
    let per_row = width as usize * components;
    let row_bytes = (per_row * bits_per_component as usize).div_ceil(8);
    let needed = row_bytes * height as usize;
    if data.len() < needed {
        return Err(format!(
            "pixel data is truncated ({} of {needed} bytes)",
            data.len()
        ));
    }

    let mut samples = Vec::with_capacity(per_row * height as usize);
    for y in 0..height as usize {
        let row = &data[y * row_bytes..(y + 1) * row_bytes];
        match bits_per_component {
            8 => samples.extend(row[..per_row].iter().map(|byte| u16::from(*byte))),
            16 => samples.extend(
                row[..per_row * 2]
                    .chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]])),
            ),
            bits => {
                let mut offset = 0usize;
                for _ in 0..per_row {
                    samples.push(read_bits(row, offset, bits));
                    offset += bits as usize;
                }
            }
        }
    }

    Ok(samples)
}

fn read_bits(row: &[u8], bit_offset: usize, bits: u32) -> u16 {
    let mut value = 0u16;
    for step in 0..bits as usize {
        let index = bit_offset + step;
        let byte = row.get(index / 8).copied().unwrap_or(0);
        value = (value << 1) | u16::from((byte >> (7 - index % 8)) & 1);
    }
    value
}

fn scale(value: u16, bits_per_component: u32) -> u8 {
    match bits_per_component {
        8 => value as u8,
        16 => (value >> 8) as u8,
        bits => {
            let max = (1u32 << bits) - 1;
            ((u32::from(value) * 255 + max / 2) / max) as u8
        }
    }
}

fn cmyk_to_rgb(c: u8, m: u8, y: u8, k: u8) -> [u8; 3] {
    let ink = |value: u8| 1.0 - f32::from(value) / 255.0;
    let black = ink(k);
    [
        (255.0 * ink(c) * black).round() as u8,
        (255.0 * ink(m) * black).round() as u8,
        (255.0 * ink(y) * black).round() as u8,
    ]
}

/// Fold a `/SMask` alpha channel into the decoded image. Without this, logos and
/// cut-out figures render as opaque boxes.
fn apply_soft_mask(pdf: &Document, dict: &Dictionary, image: DynamicImage) -> DynamicImage {
    let Some(mask) = decode_soft_mask(pdf, dict) else {
        return image;
    };

    let (width, height) = (image.width(), image.height());
    let mask = if mask.width() == width && mask.height() == height {
        mask
    } else {
        image::imageops::resize(
            &mask,
            width,
            height,
            image::imageops::FilterType::Triangle,
        )
    };

    let mut rgba = image.to_rgba8();
    for (pixel, alpha) in rgba.pixels_mut().zip(mask.pixels()) {
        pixel.0[3] = alpha.0[0];
    }
    DynamicImage::ImageRgba8(RgbaImage::from(rgba))
}

fn decode_soft_mask(pdf: &Document, dict: &Dictionary) -> Option<GrayImage> {
    let mask = resolve(pdf, dict.get(b"SMask").ok()?).as_stream().ok()?;
    let width = u32::try_from(mask.dict.get(b"Width").ok()?.as_i64().ok()?).ok()?;
    let height = u32::try_from(mask.dict.get(b"Height").ok()?.as_i64().ok()?).ok()?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return None;
    }

    let filters: Vec<String> = mask
        .filters()
        .ok()?
        .into_iter()
        .map(|filter| String::from_utf8_lossy(filter).to_string())
        .collect();

    if filters.last().map(String::as_str) == Some("DCTDecode") {
        let bytes = predecode(
            &mask.dict,
            &mask.content,
            &filters[..filters.len() - 1],
        )
        .ok()?;
        return image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
            .ok()
            .map(|image| image.to_luma8());
    }

    let bits = u32::try_from(
        mask.dict
            .get(b"BitsPerComponent")
            .ok()
            .and_then(|object| object.as_i64().ok())
            .unwrap_or(8),
    )
    .ok()?;
    if !matches!(bits, 1 | 2 | 4 | 8 | 16) {
        return None;
    }

    let bytes = predecode(&mask.dict, &mask.content, &filters).ok()?;
    let samples = read_samples(&bytes, width, height, 1, bits).ok()?;
    GrayImage::from_raw(
        width,
        height,
        samples.iter().map(|value| scale(*value, bits)).collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_low_bit_depths_to_full_range() {
        assert_eq!(scale(0, 1), 0);
        assert_eq!(scale(1, 1), 255);
        assert_eq!(scale(3, 2), 255);
        assert_eq!(scale(15, 4), 255);
        assert_eq!(scale(200, 8), 200);
        assert_eq!(scale(u16::MAX, 16), 255);
    }

    #[test]
    fn reads_one_bit_samples_with_row_padding() {
        // Two rows of 4 one-bit pixels; each row is padded to a full byte.
        let data = [0b1010_0000u8, 0b0101_0000u8];
        let samples = read_samples(&data, 4, 2, 1, 1).unwrap();
        assert_eq!(samples, vec![1, 0, 1, 0, 0, 1, 0, 1]);
    }

    #[test]
    fn reads_eight_bit_rgb_samples() {
        let data = [1u8, 2, 3, 4, 5, 6];
        let samples = read_samples(&data, 2, 1, 3, 8).unwrap();
        assert_eq!(samples, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn rejects_truncated_pixel_data() {
        assert!(read_samples(&[0u8; 3], 4, 4, 3, 8).is_err());
    }

    #[test]
    fn converts_cmyk_corners() {
        assert_eq!(cmyk_to_rgb(0, 0, 0, 0), [255, 255, 255]);
        assert_eq!(cmyk_to_rgb(0, 0, 0, 255), [0, 0, 0]);
        assert_eq!(cmyk_to_rgb(255, 0, 0, 0), [0, 255, 255]);
    }

    #[test]
    fn infers_component_count_from_data_length() {
        assert_eq!(infer_components(16, 4, 4, 8).unwrap(), 1);
        assert_eq!(infer_components(48, 4, 4, 8).unwrap(), 3);
        assert_eq!(infer_components(64, 4, 4, 8).unwrap(), 4);
        // Trailing padding past the last row is tolerated.
        assert_eq!(infer_components(17, 4, 4, 8).unwrap(), 1);
        // Too little data for even one component per pixel.
        assert!(infer_components(8, 4, 4, 8).is_err());
        // Two components per pixel is not a colour space flerp can name.
        assert!(infer_components(32, 4, 4, 8).is_err());
    }

    #[test]
    fn maps_lines_back_to_their_page() {
        let document = PdfDocument {
            text: String::new(),
            pages: vec![
                PdfPage { number: 1, start_line: 0, line_count: 10, image_count: 0 },
                PdfPage { number: 2, start_line: 10, line_count: 5, image_count: 0 },
                PdfPage { number: 3, start_line: 15, line_count: 7, image_count: 0 },
            ],
            images: Vec::new(),
            skipped: Vec::new(),
            image_error: None,
        };

        assert_eq!(document.page_of_line(0), 0);
        assert_eq!(document.page_of_line(9), 0);
        assert_eq!(document.page_of_line(10), 1);
        assert_eq!(document.page_of_line(14), 1);
        assert_eq!(document.page_of_line(15), 2);
        assert_eq!(document.page_of_line(100), 2);
    }
}
