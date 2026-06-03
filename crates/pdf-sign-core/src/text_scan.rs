//! Shared PDF content-stream text scanner.
//!
//! Streams a page's content stream while tracking the text matrix, emitting
//! each visible-ASCII text run with its PDF-native `(x, y)` baseline (origin at
//! the page's bottom-left, in points).
//!
//! Only bytes in `0x20..=0x7E` are treated as characters; any chunk containing
//! a byte outside that range is dropped wholesale. This is intentional: it
//! makes the scanner safe on PDFs that mix English labels with CJK CIDFonts
//! (the `pdf-extract` crate panics on unsupported CMaps like `UniGB-UTF16-H`).
//!
//! Extracted verbatim from `anchor.rs` so both anchor location and worktime
//! (QTY column) extraction share one scanner — behaviour is byte-for-byte
//! identical to the original private implementation.

use crate::error::SignError;
use lopdf::{content::Content, Document, Object};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
pub(crate) struct Mat([f64; 6]);

impl Mat {
    fn identity() -> Self {
        Mat([1., 0., 0., 1., 0., 0.])
    }
    /// Pre-multiply by translation matrix `T(tx, ty)`: result = T * self.
    fn translate(&mut self, tx: f64, ty: f64) {
        let m = self.0;
        self.0[4] = m[4] + tx * m[0] + ty * m[2];
        self.0[5] = m[5] + tx * m[1] + ty * m[3];
    }
}

pub(crate) struct Chunk {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) text: String,
}

/// Read, decode and scan one page's content stream into ASCII text chunks.
///
/// Inherits the page-range and content-stream error mapping that
/// `find_anchor_baseline` used to own, so anchor location and worktime
/// extraction share identical boundary handling.
pub(crate) fn page_chunks(
    doc: &Document,
    pdf_path: &Path,
    page_index: usize,
) -> Result<Vec<Chunk>, SignError> {
    let pages = doc.get_pages();
    let page_num = (page_index + 1) as u32;
    let total = pages.len();
    let page_id = *pages.get(&page_num).ok_or_else(|| SignError::PageOutOfRange {
        path: pdf_path.to_path_buf(),
        requested: page_index,
        total,
    })?;

    let content_bytes = doc
        .get_page_content(page_id)
        .map_err(|e| content_err(pdf_path, page_index, e))?;
    let content =
        Content::decode(&content_bytes).map_err(|e| content_err(pdf_path, page_index, e))?;
    Ok(scan_ascii_chunks(&content))
}

pub(crate) fn content_err(path: &Path, page_index: usize, e: lopdf::Error) -> SignError {
    let source = match e {
        lopdf::Error::IO(io) => io,
        other => std::io::Error::new(std::io::ErrorKind::InvalidData, other.to_string()),
    };
    SignError::ContentStreamRead {
        path: PathBuf::from(path),
        page_index,
        source,
    }
}

/// Map a lopdf load error onto the `io::Error` carried by
/// `SignError::PdfLoadFailed`.
pub(crate) fn lopdf_err_to_io(e: lopdf::Error) -> std::io::Error {
    match e {
        lopdf::Error::IO(io) => io,
        other => std::io::Error::new(std::io::ErrorKind::InvalidData, other.to_string()),
    }
}

pub(crate) fn scan_ascii_chunks(content: &Content) -> Vec<Chunk> {
    let mut tm = Mat::identity();
    let mut tlm = Mat::identity();
    let mut leading = 0.0_f64;
    let mut in_text = false;
    let mut out: Vec<Chunk> = Vec::new();

    for op in &content.operations {
        match op.operator.as_str() {
            "BT" => {
                tm = Mat::identity();
                tlm = Mat::identity();
                in_text = true;
            }
            "ET" => {
                in_text = false;
            }
            "Tm" if op.operands.len() == 6 => {
                let m = [
                    num(&op.operands[0]),
                    num(&op.operands[1]),
                    num(&op.operands[2]),
                    num(&op.operands[3]),
                    num(&op.operands[4]),
                    num(&op.operands[5]),
                ];
                tm = Mat(m);
                tlm = Mat(m);
            }
            "Td" if op.operands.len() == 2 => {
                tlm.translate(num(&op.operands[0]), num(&op.operands[1]));
                tm = tlm;
            }
            "TD" if op.operands.len() == 2 => {
                let ty = num(&op.operands[1]);
                leading = -ty;
                tlm.translate(num(&op.operands[0]), ty);
                tm = tlm;
            }
            "T*" => {
                tlm.translate(0., -leading);
                tm = tlm;
            }
            "TL" if op.operands.len() == 1 => {
                leading = num(&op.operands[0]);
            }
            "Tj" if in_text => push_str(&op.operands, &tm, &mut out),
            "TJ" if in_text => push_array(&op.operands, &tm, &mut out),
            "'" if in_text => {
                tlm.translate(0., -leading);
                tm = tlm;
                push_str(&op.operands, &tm, &mut out);
            }
            "\"" if in_text && op.operands.len() == 3 => {
                tlm.translate(0., -leading);
                tm = tlm;
                push_str(&op.operands[2..], &tm, &mut out);
            }
            _ => {}
        }
    }
    out
}

fn num(o: &Object) -> f64 {
    match o {
        Object::Integer(i) => *i as f64,
        Object::Real(r) => *r as f64,
        _ => 0.,
    }
}

fn ascii_filter(bytes: &[u8]) -> Option<String> {
    let mut s = String::with_capacity(bytes.len());
    for &b in bytes {
        if (0x20..=0x7E).contains(&b) {
            s.push(b as char);
        } else {
            return None;
        }
    }
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn push_str(operands: &[Object], tm: &Mat, out: &mut Vec<Chunk>) {
    if let Some(Object::String(bytes, _)) = operands.first() {
        if let Some(text) = ascii_filter(bytes) {
            out.push(Chunk {
                x: tm.0[4],
                y: tm.0[5],
                text,
            });
        }
    }
}

fn push_array(operands: &[Object], tm: &Mat, out: &mut Vec<Chunk>) {
    let Some(Object::Array(arr)) = operands.first() else {
        return;
    };
    let mut buf = Vec::new();
    for el in arr {
        if let Object::String(b, _) = el {
            buf.extend_from_slice(b);
        }
    }
    if let Some(text) = ascii_filter(&buf) {
        out.push(Chunk {
            x: tm.0[4],
            y: tm.0[5],
            text,
        });
    }
}

pub(crate) fn locate(chunks: &[Chunk], needle: &str) -> Option<(f64, f64)> {
    // Fast path: the anchor lives entirely inside a single Tj/TJ chunk.
    for c in chunks {
        if c.text.contains(needle) {
            return Some((c.x, c.y));
        }
    }
    // Fallback: anchor split across consecutive Tj calls. We join with an
    // unprintable separator so substring matches cannot straddle chunk
    // boundaries silently — needle is plain ASCII and never contains \x1f.
    let mut joined = String::new();
    let mut offs: Vec<(usize, &Chunk)> = Vec::new();
    for c in chunks {
        offs.push((joined.len(), c));
        joined.push_str(&c.text);
        joined.push('\x1f');
    }
    let pos = joined.find(needle)?;
    offs.iter()
        .rev()
        .find(|(off, _)| *off <= pos)
        .map(|(_, c)| (c.x, c.y))
}
