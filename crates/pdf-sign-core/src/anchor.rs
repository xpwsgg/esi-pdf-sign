//! Locate a visible-ASCII text anchor on a PDF page.
//!
//! The heavy lifting — streaming the page content stream and tracking the text
//! matrix — now lives in [`crate::text_scan`]. This module only layers the
//! anchor-not-found semantics on top of `page_chunks` + `locate`.

use crate::error::SignError;
use crate::text_scan::{locate, page_chunks};
use lopdf::Document;
use std::path::Path;

/// Find anchor baseline position and return (page_index, x, y)
pub(crate) fn find_anchor_baseline(
    doc: &Document,
    pdf_path: &Path,
    page_index: usize,
    anchor_text: &str,
) -> Result<(usize, f64, f64), SignError> {
    let pages = doc.get_pages();
    let total_pages = pages.len();

    // Validate page_index before searching (preserve PageOutOfRange error behavior)
    if page_index >= total_pages {
        return Err(SignError::PageOutOfRange {
            path: pdf_path.to_path_buf(),
            requested: page_index,
            total: total_pages,
        });
    }

    // Strategy: signature anchors are typically on the last page.
    // Search order: last page → specified page → all other pages

    // 1. Try the last page first (most common case for signatures)
    let last_page_index = total_pages.saturating_sub(1);
    if let Ok(chunks) = page_chunks(doc, pdf_path, last_page_index) {
        if let Some((x, y)) = locate(&chunks, anchor_text) {
            return Ok((last_page_index, x, y));
        }
    }

    // 2. Try the specified page (for backwards compatibility)
    if page_index != last_page_index {
        if let Ok(chunks) = page_chunks(doc, pdf_path, page_index) {
            if let Some((x, y)) = locate(&chunks, anchor_text) {
                return Ok((page_index, x, y));
            }
        }
    }

    // 3. Search all remaining pages as fallback
    for idx in 0..total_pages {
        // Skip pages we already checked
        if idx == last_page_index || idx == page_index {
            continue;
        }

        if let Ok(chunks) = page_chunks(doc, pdf_path, idx) {
            if let Some((x, y)) = locate(&chunks, anchor_text) {
                return Ok((idx, x, y));
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
