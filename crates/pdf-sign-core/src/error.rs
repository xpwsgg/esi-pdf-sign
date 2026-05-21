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
}
