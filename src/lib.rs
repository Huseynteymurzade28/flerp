// The binary is a thin wrapper around this crate, so every module lives here
// and is compiled exactly once.
pub mod app;
pub mod app_structs;
pub mod file_utils;
pub mod headless;
pub mod media;
pub mod pdf_doc;
pub mod settings;
pub mod stopwords;
pub mod text_analysis;
pub mod ui_components;
