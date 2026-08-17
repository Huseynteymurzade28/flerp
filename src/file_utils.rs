use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
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

/// Files worth searching under a directory, plus what the walk could not read.
pub struct SearchableFiles {
    /// Searchable files, sorted, so a directory search reports its matches in
    /// the same order on every run and on every filesystem.
    pub files: Vec<PathBuf>,
    /// Directories the walk was not allowed to look inside, with the reason.
    /// A permission error on one subdirectory should not abandon the search,
    /// but it should not disappear either.
    pub unreadable: Vec<(PathBuf, String)>,
}

pub fn load_file(file_path: &str) -> Result<LoadedFile, Box<dyn Error>> {
    let path = Path::new(file_path);

    match extension_of(path).as_deref() {
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

/// Load only what a search needs: the text, and for a PDF the page table that
/// turns a line number back into a page.
///
/// This is the sweep's counterpart to [`load_file`]. Images are left alone in
/// both senses — none are decoded, and an image file is refused rather than
/// searched, because the text flerp shows for one is a description it wrote
/// itself and matching on that would report hits that are not in the file.
pub fn load_for_search(file_path: &str) -> Result<LoadedFile, Box<dyn Error>> {
    let path = Path::new(file_path);

    match extension_of(path).as_deref() {
        Some("pdf") => {
            let document = pdf_doc::load_text(file_path)?;
            Ok(LoadedFile {
                content: document.text.clone(),
                document: Some(Arc::new(document)),
                media: Vec::new(),
                notice: None,
            })
        }
        Some("png") | Some("jpg") | Some("jpeg") => Err("an image has no text to search".into()),
        _ => Ok(LoadedFile {
            content: fs::read_to_string(file_path)?,
            document: None,
            media: Vec::new(),
            notice: None,
        }),
    }
}

/// Collect every file under `root` that a search could say something about.
///
/// A directory search has to decide for itself what is worth opening, so it
/// uses grep's rule with one addition: skip anything hidden, skip anything that
/// does not look like text, and let PDFs through, because reading those is the
/// reason to point flerp at a folder instead of grep. Images are skipped — the
/// text flerp shows for an image is a description it wrote itself, and matching
/// on that would report hits that are not in the file.
pub fn collect_searchable(root: &Path) -> Result<SearchableFiles, Box<dyn Error>> {
    // Fail loudly for the directory the user actually named. Anything deeper is
    // collected as unreadable instead, since the user did not ask for it by name.
    drop(fs::read_dir(root)?);

    let mut found = SearchableFiles {
        files: Vec::new(),
        unreadable: Vec::new(),
    };
    let mut pending = vec![root.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                found.unreadable.push((directory, error.to_string()));
                continue;
            }
        };

        for entry in entries.flatten() {
            if is_hidden(&entry.file_name()) {
                continue;
            }

            // `file_type` reports on the link itself rather than its target, so
            // a symlink pointing at an ancestor cannot send the walk in circles.
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let path = entry.path();

            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() && is_searchable(&path) {
                found.files.push(path);
            }
        }
    }

    found.files.sort();
    found.unreadable.sort();
    Ok(found)
}

fn extension_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
}

/// Dotfiles are skipped for the same reason grep skips them: a walk that
/// descends into `.git` spends its time on object files nobody searched for.
fn is_hidden(name: &OsStr) -> bool {
    name.to_string_lossy().starts_with('.')
}

fn is_searchable(path: &Path) -> bool {
    match extension_of(path).as_deref() {
        Some("pdf") => true,
        Some("png") | Some("jpg") | Some("jpeg") => false,
        _ => looks_like_text(path),
    }
}

/// Whether a file's opening bytes look like text rather than a binary blob.
///
/// A NUL byte is the signal grep uses, and testing a short prefix costs one
/// read, where loading the whole file would mean pulling a multi-megabyte
/// binary into memory only to have the UTF-8 check throw it away.
fn looks_like_text(path: &Path) -> bool {
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };

    let mut sample = [0u8; 8192];
    let Ok(read) = file.read(&mut sample) else {
        return false;
    };
    let sample = &sample[..read];

    if sample.contains(&0) {
        return false;
    }

    match std::str::from_utf8(sample) {
        Ok(_) => true,
        // A multi-byte character can straddle the end of the sample. That is the
        // sample's fault, not the file's, and `error_len` is `None` for exactly
        // that case.
        Err(error) => error.error_len().is_none(),
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
