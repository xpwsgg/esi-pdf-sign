//! Error types for the PDF signing pipeline.
//!
//! Each variant corresponds to one failure mode named in design §2.1.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignError {
    #[error("Failed to load PDF at {path}: {source}")]
    PdfLoadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Page index {requested} out of range (PDF has {total} pages) at {path}")]
    PageOutOfRange {
        path: PathBuf,
        requested: usize,
        total: usize,
    },

    #[error("Failed to load signature image at {path}: {source}")]
    ImageLoadFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write output PDF at {path}: {source}")]
    OutputWriteFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to read page {page_index} content stream at {path}: {source}")]
    ContentStreamRead {
        path: PathBuf,
        page_index: usize,
        #[source]
        source: std::io::Error,
    },

    #[error("Anchor text {anchor:?} not found on page {page_index} at {path}")]
    AnchorNotFound {
        path: PathBuf,
        anchor: String,
        page_index: usize,
    },

    #[error("No QTY worktime table found in any page at {path}")]
    WorktimeTableNotFound { path: PathBuf },

    #[error("No QTY worktime table found in any page at {path} (page decode errors: {decode_errors})")]
    WorktimeTableNotFoundWithDecodeErrors { path: PathBuf, decode_errors: String },

    #[error("PDF structure error at {path}: {detail}")]
    PdfStructureError { path: PathBuf, detail: String },
}
