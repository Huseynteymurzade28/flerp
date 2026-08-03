use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::media::MediaItem;
use crate::pdf_doc::{self, PdfDocument};

/// Everything flerp learned from a file in one load.
pub struct LoadedFile {
    pub content: String,
    pub document: Option<Arc<PdfDocument>>,
    pub media: Vec<MediaItem>,
    pub notice: Option<String>,
}

pub fn load_file(file_path: &str) -> Result<LoadedFile, Box<dyn Error>> {
    let path = Path::new(file_path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("pdf") => load_pdf(file_path),
        Some("png") | Some("jpg") | Some("jpeg") => load_image(path),
        _ => Ok(LoadedFile {
            content: fs::read_to_string(file_path)?,
            document: None,
            media: Vec::new(),
            notice: None,
        }),
    }
}

fn load_pdf(file_path: &str) -> Result<LoadedFile, Box<dyn Error>> {
    let document = pdf_doc::load(file_path)?;

    let media = document
        .images
        .iter()
        .enumerate()
        .map(|(index, asset)| MediaItem {
            key: format!("{file_path}#{index}"),
            title: format!("Page {} · image {}", asset.page, index + 1),
            detail: format!(
                "{}x{} · {} · {}",
                asset.width, asset.height, asset.encoding, asset.color_space
            ),
            page: Some(asset.page),
            image: asset.image.clone(),
        })
        .collect();

    let notice = match (&document.image_error, document.skipped.len()) {
        (Some(error), _) => Some(format!("Embedded images unavailable: {error}")),
        (None, 0) => None,
        (None, count) => Some(format!(
            "{count} embedded image(s) could not be decoded; see the Media tab."
        )),
    };

    Ok(LoadedFile {
        content: document.text.clone(),
        document: Some(Arc::new(document)),
        media,
        notice,
    })
}

fn load_image(path: &Path) -> Result<LoadedFile, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    let format = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_uppercase())
        .unwrap_or_else(|| "IMAGE".to_string());
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown");

    let image = image::ImageReader::open(path)?.decode()?;
    let (width, height) = (image.width(), image.height());

    let content = format!(
        "Image file loaded.\n\nName: {name}\nFormat: {format}\nDimensions: {width}x{height}\nSize: {} bytes\n\nOpen the Media tab to view the image itself.",
        metadata.len()
    );

    Ok(LoadedFile {
        content,
        document: None,
        media: vec![MediaItem {
            key: path.to_string_lossy().to_string(),
            title: name.to_string(),
            detail: format!("{width}x{height} · {format}"),
            page: None,
            image,
        }],
        notice: None,
    })
}
