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

/// Maximum reasonable signature dimensions in PDF points.
/// Standard A4 page is ~595x842 points.
const MAX_SIGNATURE_DIMENSION: f32 = 600.0;

/// Maximum reasonable offset in PDF points (can be negative or positive).
const MAX_OFFSET: f32 = 1000.0;

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

impl SignSpec {
    /// Validate that all parameters are within reasonable bounds.
    ///
    /// Returns an error message if validation fails, or `Ok(())` if all checks pass.
    pub fn validate(&self) -> Result<(), String> {
        // Anchor text must not be empty
        if self.anchor_text.trim().is_empty() {
            return Err("anchor_text cannot be empty".to_string());
        }

        // Width and height must be positive, finite, and reasonable
        if !self.width.is_finite() || self.width <= 0.0 {
            return Err(format!("width must be positive and finite, got {}", self.width));
        }
        if !self.height.is_finite() || self.height <= 0.0 {
            return Err(format!("height must be positive and finite, got {}", self.height));
        }
        if self.width > MAX_SIGNATURE_DIMENSION {
            return Err(format!(
                "width {} exceeds maximum {}",
                self.width, MAX_SIGNATURE_DIMENSION
            ));
        }
        if self.height > MAX_SIGNATURE_DIMENSION {
            return Err(format!(
                "height {} exceeds maximum {}",
                self.height, MAX_SIGNATURE_DIMENSION
            ));
        }

        // Offsets must be finite and within reasonable bounds
        if !self.dx.is_finite() || self.dx.abs() > MAX_OFFSET {
            return Err(format!(
                "dx must be finite and within ±{}, got {}",
                MAX_OFFSET, self.dx
            ));
        }
        if !self.dy.is_finite() || self.dy.abs() > MAX_OFFSET {
            return Err(format!(
                "dy must be finite and within ±{}, got {}",
                MAX_OFFSET, self.dy
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_spec() -> SignSpec {
        SignSpec {
            page_index: 0,
            anchor_text: "Test Anchor".to_string(),
            dx: 10.0,
            dy: 20.0,
            width: 100.0,
            height: 50.0,
        }
    }

    #[test]
    fn validate_accepts_normal_spec() {
        let spec = valid_spec();
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn validate_rejects_empty_anchor_text() {
        let mut spec = valid_spec();
        spec.anchor_text = "".to_string();
        let err = spec.validate().unwrap_err();
        assert!(err.contains("anchor_text cannot be empty"));
    }

    #[test]
    fn validate_rejects_whitespace_only_anchor() {
        let mut spec = valid_spec();
        spec.anchor_text = "   ".to_string();
        let err = spec.validate().unwrap_err();
        assert!(err.contains("anchor_text cannot be empty"));
    }

    #[test]
    fn validate_rejects_zero_width() {
        let mut spec = valid_spec();
        spec.width = 0.0;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("width must be positive"));
    }

    #[test]
    fn validate_rejects_negative_width() {
        let mut spec = valid_spec();
        spec.width = -10.0;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("width must be positive"));
    }

    #[test]
    fn validate_rejects_infinite_width() {
        let mut spec = valid_spec();
        spec.width = f32::INFINITY;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("width must be positive and finite"));
    }

    #[test]
    fn validate_rejects_nan_width() {
        let mut spec = valid_spec();
        spec.width = f32::NAN;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("width must be positive and finite"));
    }

    #[test]
    fn validate_rejects_oversized_width() {
        let mut spec = valid_spec();
        spec.width = 700.0; // exceeds MAX_SIGNATURE_DIMENSION=600
        let err = spec.validate().unwrap_err();
        assert!(err.contains("width") && err.contains("exceeds maximum"));
    }

    #[test]
    fn validate_rejects_zero_height() {
        let mut spec = valid_spec();
        spec.height = 0.0;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("height must be positive"));
    }

    #[test]
    fn validate_rejects_negative_height() {
        let mut spec = valid_spec();
        spec.height = -10.0;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("height must be positive"));
    }

    #[test]
    fn validate_rejects_oversized_height() {
        let mut spec = valid_spec();
        spec.height = 700.0;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("height") && err.contains("exceeds maximum"));
    }

    #[test]
    fn validate_accepts_negative_offsets() {
        let mut spec = valid_spec();
        spec.dx = -50.0;
        spec.dy = -100.0;
        assert!(spec.validate().is_ok(), "negative offsets should be valid");
    }

    #[test]
    fn validate_rejects_excessive_dx() {
        let mut spec = valid_spec();
        spec.dx = 1500.0; // exceeds MAX_OFFSET=1000
        let err = spec.validate().unwrap_err();
        assert!(err.contains("dx") && err.contains("within"));
    }

    #[test]
    fn validate_rejects_excessive_negative_dx() {
        let mut spec = valid_spec();
        spec.dx = -1500.0;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("dx") && err.contains("within"));
    }

    #[test]
    fn validate_rejects_infinite_dx() {
        let mut spec = valid_spec();
        spec.dx = f32::INFINITY;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("dx") && err.contains("finite"));
    }

    #[test]
    fn validate_rejects_excessive_dy() {
        let mut spec = valid_spec();
        spec.dy = 1500.0;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("dy") && err.contains("within"));
    }

    #[test]
    fn validate_rejects_nan_dy() {
        let mut spec = valid_spec();
        spec.dy = f32::NAN;
        let err = spec.validate().unwrap_err();
        assert!(err.contains("dy") && err.contains("finite"));
    }

    #[test]
    fn validate_accepts_boundary_values() {
        let mut spec = valid_spec();
        spec.width = MAX_SIGNATURE_DIMENSION; // exactly at limit
        spec.height = MAX_SIGNATURE_DIMENSION;
        spec.dx = MAX_OFFSET;
        spec.dy = -MAX_OFFSET;
        assert!(spec.validate().is_ok(), "boundary values should be valid");
    }
}
