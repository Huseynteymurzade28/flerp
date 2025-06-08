use std::error::Error;
use std::fs;
use std::path::Path;

pub fn read_file_content(file_path: &str) -> Result<String, Box<dyn Error>> {
    if Path::new(file_path).extension().and_then(|s| s.to_str()) == Some("pdf") {
        // Assuming pdf_extract crate is a dependency and accessible
        Ok(pdf_extract::extract_text(file_path)?)
    } else {
        Ok(fs::read_to_string(file_path)?)
    }
}
