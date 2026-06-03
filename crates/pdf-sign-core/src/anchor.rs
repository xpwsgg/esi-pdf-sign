//! Locate a visible-ASCII text anchor on a PDF page.
//!
//! The heavy lifting — streaming the page content stream and tracking the text
//! matrix — now lives in [`crate::text_scan`]. This module only layers the
//! anchor-not-found semantics on top of `page_chunks` + `locate`.

use crate::error::SignError;
use crate::text_scan::{locate, page_chunks};
use lopdf::Document;
use std::path::Path;

pub(crate) fn find_anchor_baseline(
    doc: &Document,
    pdf_path: &Path,
    page_index: usize,
    anchor_text: &str,
) -> Result<(f64, f64), SignError> {
    let chunks = page_chunks(doc, pdf_path, page_index)?;
    locate(&chunks, anchor_text).ok_or_else(|| SignError::AnchorNotFound {
        path: pdf_path.to_path_buf(),
        anchor: anchor_text.to_string(),
        page_index,
    })
}
