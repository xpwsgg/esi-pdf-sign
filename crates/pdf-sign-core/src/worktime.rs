//! Extract service-report worktime from the "QTY" table column.
//!
//! ESI service reports lay out a labour table whose `QTY` column holds the
//! hours for each line item (travel, standard labour, …). The `QTY` header sits
//! at a fixed page-x but a *floating* page-y across reports, so the header must
//! be located dynamically — never by a hard-coded y. Once located, every
//! numeric chunk in that column (below the header, same x) is a worktime value;
//! its row's part-number and description are read from the same baseline-y.

use crate::error::SignError;
use crate::text_scan::{lopdf_err_to_io, page_chunks, Chunk};
use lopdf::Document;
use std::path::{Path, PathBuf};

/// One line item of the worktime table.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RowDetail {
    pub part_number: String,
    pub description: String,
    pub qty: f64,
}

/// Worktime statistics for a single PDF.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PdfWorktime {
    pub rows: Vec<RowDetail>,
    pub total: f64,
}

/// Per-PDF outcome of a batch extraction (mirrors `SignResult`: never panics).
#[derive(Debug)]
pub enum WorktimeResult {
    Ok { input: PathBuf, worktime: PdfWorktime },
    Err { input: PathBuf, error: SignError },
}

/// Extract the worktime table from one PDF.
///
/// Scans pages in order; the first page carrying a `QTY` table wins. Returns
/// [`SignError::WorktimeTableNotFound`] if no page has one. A single
/// unreadable page is skipped rather than failing the whole document.
pub fn extract_worktime(pdf_path: &Path) -> Result<PdfWorktime, SignError> {
    let doc = Document::load(pdf_path).map_err(|e| SignError::PdfLoadFailed {
        path: pdf_path.to_path_buf(),
        source: lopdf_err_to_io(e),
    })?;

    let mut page_nums: Vec<u32> = doc.get_pages().keys().copied().collect();
    page_nums.sort_unstable();

    let mut pages: Vec<Vec<Chunk>> = Vec::with_capacity(page_nums.len());
    for page_num in page_nums {
        let page_index = (page_num - 1) as usize;
        // A single page that fails to decode must not sink the whole report.
        if let Ok(chunks) = page_chunks(&doc, pdf_path, page_index) {
            pages.push(chunks);
        }
    }

    worktime_from_pages(&pages).ok_or(SignError::WorktimeTableNotFound {
        path: pdf_path.to_path_buf(),
    })
}

/// Extract worktime for many PDFs serially.
///
/// Never panics and never returns `Err`; each PDF's outcome is collected
/// independently so one failure does not abort the batch (mirrors `sign_pdfs`).
pub fn extract_worktimes(paths: &[PathBuf]) -> Vec<WorktimeResult> {
    paths
        .iter()
        .map(|input| match extract_worktime(input) {
            Ok(worktime) => WorktimeResult::Ok {
                input: input.clone(),
                worktime,
            },
            Err(error) => WorktimeResult::Err {
                input: input.clone(),
                error,
            },
        })
        .collect()
}

/// First page that yields a worktime table wins.
fn worktime_from_pages(pages: &[Vec<Chunk>]) -> Option<PdfWorktime> {
    pages.iter().find_map(|chunks| worktime_from_page(chunks))
}

/// QTY values must sit within this x-distance of the `QTY` header to count as
/// the same column.
const QTY_COLUMN_X_TOL: f64 = 20.0;
/// Detail cells (part-number / description / qty) share a baseline; the time
/// columns are offset by ~8.5pt, so a tight y tolerance keeps them out.
const ROW_Y_TOL: f64 = 3.0;

fn worktime_from_page(chunks: &[Chunk]) -> Option<PdfWorktime> {
    // 1. Locate the QTY header dynamically (fixed x, floating y across reports).
    let qty = chunks.iter().find(|c| c.text.trim() == "QTY")?;
    let (qty_x, qty_y) = (qty.x, qty.y);

    // 2. Each numeric chunk below the header and in the same column is a
    //    worktime value. Keep its baseline-y so rows can be ordered top-down.
    let mut rows: Vec<(f64, RowDetail)> = Vec::new();
    for c in chunks {
        if c.y >= qty_y - 1.0 {
            continue; // header row and everything above it
        }
        if (c.x - qty_x).abs() >= QTY_COLUMN_X_TOL {
            continue; // a different column
        }
        let Ok(qty_val) = c.text.trim().parse::<f64>() else {
            continue; // not a number — skip
        };

        // Text cells sharing this row's baseline-y, left of the QTY column.
        let mut left: Vec<&Chunk> = chunks
            .iter()
            .filter(|o| (o.y - c.y).abs() < ROW_Y_TOL && o.x < qty_x - 1.0)
            .filter(|o| !o.text.trim().is_empty())
            .collect();
        left.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());

        // Closest-to-QTY cell is the Description; the one left of it is the
        // Part Number. The leftmost Date cell (if any) is ignored.
        let description = left
            .last()
            .map(|c| c.text.trim().to_string())
            .unwrap_or_default();
        let part_number = if left.len() >= 2 {
            left[left.len() - 2].text.trim().to_string()
        } else {
            String::new()
        };

        rows.push((
            c.y,
            RowDetail {
                part_number,
                description,
                qty: qty_val,
            },
        ));
    }

    if rows.is_empty() {
        return None; // QTY header present but no values — not a real table
    }

    // Order rows top-to-bottom (descending page-y).
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let rows: Vec<RowDetail> = rows.into_iter().map(|(_, r)| r).collect();
    let total = rows.iter().map(|r| r.qty).sum();

    Some(PdfWorktime { rows, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(x: f64, y: f64, text: &str) -> Chunk {
        Chunk {
            x,
            y,
            text: text.to_string(),
        }
    }

    /// Mirrors the real H5P9 page-2 layout (verified via the qty_spike example):
    /// QTY header at x≈439, two rows whose part/description/qty share a
    /// baseline-y, plus time columns offset ~8.5pt in y.
    fn synthetic_page() -> Vec<Chunk> {
        vec![
            chunk(439.20, 295.49, "QTY"),
            chunk(285.67, 295.47, "Description"),
            // row 1 @ y≈258.5
            chunk(19.42, 258.42, "May 8, 2026"),
            chunk(111.17, 258.52, "SERVICE_TRAVEL"),
            chunk(223.98, 258.52, "SERVICE FSE TRAVEL TIME"),
            chunk(443.56, 258.52, "2.0"),
            chunk(671.56, 267.05, "7:00 AM"),
            // row 2 @ y≈213.8
            chunk(19.42, 213.69, "May 8, 2026"),
            chunk(108.12, 213.79, "STANDARD_LABOR"),
            chunk(223.98, 213.79, "SERVICE FSE STANDARD LABOR"),
            chunk(443.56, 213.79, "6.0"),
            chunk(671.56, 222.31, "9:00 AM"),
        ]
    }

    #[test]
    fn extracts_total_and_rows_from_qty_column() {
        let wt = worktime_from_page(&synthetic_page()).expect("QTY table should be found");
        assert_eq!(wt.rows.len(), 2, "two labour rows");
        assert!(
            (wt.total - 8.0).abs() < 1e-9,
            "total should be 8.0, got {}",
            wt.total
        );
        // Rows ordered top-to-bottom (descending page-y).
        assert!((wt.rows[0].qty - 2.0).abs() < 1e-9);
        assert_eq!(wt.rows[0].part_number, "SERVICE_TRAVEL");
        assert_eq!(wt.rows[0].description, "SERVICE FSE TRAVEL TIME");
        assert!((wt.rows[1].qty - 6.0).abs() < 1e-9);
        assert_eq!(wt.rows[1].part_number, "STANDARD_LABOR");
        assert_eq!(wt.rows[1].description, "SERVICE FSE STANDARD LABOR");
    }

    #[test]
    fn returns_none_when_no_qty_header() {
        let chunks = vec![chunk(100.0, 200.0, "Hello"), chunk(200.0, 200.0, "World")];
        assert!(worktime_from_page(&chunks).is_none());
        assert!(worktime_from_pages(&[chunks]).is_none());
    }

    #[test]
    fn returns_none_when_qty_header_but_no_values() {
        let chunks = vec![
            chunk(439.20, 295.49, "QTY"),
            chunk(285.67, 295.47, "Description"),
        ];
        assert!(worktime_from_page(&chunks).is_none());
    }

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("CARGO_MANIFEST_DIR has two parents")
            .to_path_buf()
    }

    #[test]
    fn extract_worktime_real_h5p9_totals_8() {
        let input = workspace_root().join("H5P9-\u{4e94}\u{6708}.pdf");
        if !input.exists() {
            eprintln!("H5P9 fixture not found, skipping: {input:?}");
            return;
        }
        let wt = extract_worktime(&input).expect("worktime extracted from H5P9");
        assert!(
            (wt.total - 8.0).abs() < 1e-9,
            "H5P9 total should be 8.0, got {}",
            wt.total
        );
        assert_eq!(wt.rows.len(), 2, "H5P9 has two labour rows");
    }

    #[test]
    fn extract_worktime_missing_file_is_pdf_load_failed() {
        let missing = PathBuf::from("/tmp/no-such-worktime-pdf-7c3e2b.pdf");
        let err = extract_worktime(&missing).expect_err("missing file should fail");
        assert!(
            matches!(err, SignError::PdfLoadFailed { .. }),
            "expected PdfLoadFailed, got {err:?}"
        );
    }
}
