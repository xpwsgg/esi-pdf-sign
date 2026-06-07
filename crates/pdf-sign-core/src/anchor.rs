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
    // Try the specified page first (for backwards compatibility and performance)
    if let Ok(chunks) = page_chunks(doc, pdf_path, page_index) {
        if let Some(pos) = locate(&chunks, anchor_text) {
            return Ok(pos);
        }
    }

    // If not found on the specified page, search all pages
    // This handles cases where PDFs have varying page counts (e.g., H5R5 has 3 pages vs H5R30 has 2 pages)
    let pages = doc.get_pages();
    let total_pages = pages.len();

    for idx in 0..total_pages {
        // Skip the page we already checked
        if idx == page_index {
            continue;
        }

        if let Ok(chunks) = page_chunks(doc, pdf_path, idx) {
            if let Some(pos) = locate(&chunks, anchor_text) {
                return Ok(pos);
            }
        }
    }

    // Not found on any page
    Err(SignError::AnchorNotFound {
        path: pdf_path.to_path_buf(),
        anchor: anchor_text.to_string(),
        page_index,
    })
}
