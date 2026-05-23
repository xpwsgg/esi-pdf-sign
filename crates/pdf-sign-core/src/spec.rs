//! `SignSpec` describes where to place a signature on a PDF page.
//!
//! Position is **anchor-relative**: at sign time we locate the first occurrence
//! of `anchor_text` on the target page (case-sensitive substring match in the
//! page's content stream), then place the signature image at
//! `(anchor_baseline_x + dx, anchor_baseline_y + dy)` — where `anchor_baseline_*`
//! is the PDF-native coordinate of the first character's text baseline.
//!
//! Coordinates use the PDF standard system: origin at the bottom-left of the
//! page, y increases upward, units are PostScript points (1 pt = 1/72 inch).

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct SignSpec {
    /// 0-based page index where the anchor lives.
    pub page_index: usize,
    /// Substring searched verbatim in the page's content stream.
    pub anchor_text: String,
    /// Horizontal offset from the anchor's first-character x.
    pub dx: f32,
    /// Vertical offset above the anchor's baseline; positive = signature is
    /// drawn higher on the page (PDF y up).
    pub dy: f32,
    /// Signature image width in PDF points.
    pub width: f32,
    /// Signature image height in PDF points.
    pub height: f32,
}
