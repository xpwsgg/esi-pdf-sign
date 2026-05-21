//! `SignSpec` describes where to place a signature on a PDF page.
//!
//! All coordinates use the PDF standard coordinate system: origin at the
//! bottom-left of the page, y increases upward, units are PostScript points
//! (1 pt = 1/72 inch).

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct SignSpec {
    pub page_index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
