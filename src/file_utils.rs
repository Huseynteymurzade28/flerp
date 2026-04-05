use std::error::Error;
use std::fs;
use std::path::Path;

pub fn read_file_content(file_path: &str) -> Result<String, Box<dyn Error>> {
    let path = Path::new(file_path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());

    match extension.as_deref() {
        Some("pdf") => Ok(pdf_extract::extract_text(file_path)?),
        Some("png") | Some("jpg") | Some("jpeg") => read_image_summary(path),
        _ => Ok(fs::read_to_string(file_path)?),
    }
}

fn read_image_summary(path: &Path) -> Result<String, Box<dyn Error>> {
    let metadata = fs::metadata(path)?;
    let format = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_uppercase())
        .unwrap_or_else(|| "IMAGE".to_string());
    let dimensions = image::image_dimensions(path)?;

    Ok(format!(
        "Image file loaded for viewing.\n\nName: {}\nFormat: {}\nDimensions: {}x{}\nSize: {} bytes\n\nTerminal image rendering is not available, so flerp shows image metadata instead.",
        path.file_name().and_then(|value| value.to_str()).unwrap_or("unknown"),
        format,
        dimensions.0,
        dimensions.1,
        metadata.len()
    ))
}
