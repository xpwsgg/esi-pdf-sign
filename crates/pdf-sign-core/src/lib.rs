//! pdf-sign-core: PDF visual signature overlay library.
//!
//! Public API:
//! - [`sign_pdf`]: sign one PDF, return new path or fail with [`SignError`].
//! - [`sign_pdfs`]: sign many PDFs serially, return a [`Vec<SignResult>`] —
//!   never panics, single failure does not stop the batch.
//!
//! Works on XFA-form PDFs by operating on page content streams only.

pub mod error;
mod anchor;
mod overlay;
pub mod spec;

pub use error::SignError;
pub use spec::SignSpec;

use lopdf::Document;
use std::path::{Path, PathBuf};

/// Outcome of signing a single PDF inside a batch.
#[derive(Debug)]
pub enum SignResult {
    Ok { input: PathBuf, output: PathBuf },
    Err { input: PathBuf, error: SignError },
}

/// Sign one PDF by overlaying a signature PNG at a position computed from
/// the anchor text in `spec`.
///
/// Steps: load PDF → locate `spec.anchor_text` baseline on `spec.page_index`
/// → place the signature image at `(anchor_x + dx, anchor_y + dy)` → save to
/// the `signed/` sub-directory next to the input PDF (file name preserved,
/// so `/foo/bar.pdf` → `/foo/signed/bar.pdf`).
pub fn sign_pdf(
    pdf_path: &Path,
    signature_png_path: &Path,
    spec: &SignSpec,
) -> Result<PathBuf, SignError> {
    let mut doc = Document::load(pdf_path).map_err(|e| SignError::PdfLoadFailed {
        path: pdf_path.to_path_buf(),
        source: lopdf_err_to_io(e),
    })?;

    let (ax, ay) = anchor::find_anchor_baseline(&doc, pdf_path, spec.page_index, &spec.anchor_text)?;
    let placement = overlay::Placement {
        page_index: spec.page_index,
        x: ax as f32 + spec.dx,
        y: ay as f32 + spec.dy,
        width: spec.width,
        height: spec.height,
    };
    overlay::overlay_signature_on_page(&mut doc, pdf_path, signature_png_path, &placement)?;

    let output_path = make_signed_path(pdf_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SignError::OutputWriteFailed {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    doc.save(&output_path).map_err(|e| SignError::OutputWriteFailed {
        path: output_path.clone(),
        source: e,
    })?;
    Ok(output_path)
}

/// Sign many PDFs serially with the same signature image and spec.
///
/// Per design §2.2 flow constraints: this function **never panics** and
/// **never returns Err**; each PDF's outcome is collected independently
/// so a single failure does not abort the batch. Caller iterates the
/// result vec to render success/failure UI.
pub fn sign_pdfs(
    pdf_paths: &[PathBuf],
    signature_png_path: &Path,
    spec: &SignSpec,
) -> Vec<SignResult> {
    pdf_paths
        .iter()
        .map(|input| match sign_pdf(input, signature_png_path, spec) {
            Ok(output) => SignResult::Ok {
                input: input.clone(),
                output,
            },
            Err(error) => SignResult::Err {
                input: input.clone(),
                error,
            },
        })
        .collect()
}

fn lopdf_err_to_io(e: lopdf::Error) -> std::io::Error {
    match e {
        lopdf::Error::IO(io) => io,
        other => std::io::Error::new(std::io::ErrorKind::InvalidData, other.to_string()),
    }
}

/// `/dir/foo.ext` -> `/dir/signed/foo.ext`. File name (with extension) is
/// preserved verbatim; only the parent gets an extra `signed/` segment.
/// Falls back to the literal "output.pdf" if `input` has no file_name.
fn make_signed_path(input: &Path) -> PathBuf {
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let file_name = input
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_else(|| std::ffi::OsString::from("output.pdf"));
    parent.join("signed").join(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("CARGO_MANIFEST_DIR has two parents")
            .to_path_buf()
    }

    fn default_spec() -> SignSpec {
        SignSpec {
            page_index: 1,
            anchor_text: "ESI Engineer's Signature".to_string(),
            // Locked to the H5P9 layout: anchor baseline lives at PDF-y ≈ 100.166
            // on page 2, signature bottom sits at PDF-y ≈ 122.8 — so dy = 22.634
            // raises the box that far above the baseline.
            dx: 0.0,
            dy: 22.634,
            width: 106.7,
            height: 40.0,
        }
    }

    #[test]
    fn make_signed_path_places_into_signed_subdir() {
        let p = PathBuf::from("/tmp/H5P9-foo.pdf");
        assert_eq!(
            make_signed_path(&p),
            PathBuf::from("/tmp/signed/H5P9-foo.pdf")
        );
    }

    #[test]
    fn make_signed_path_keeps_filename_without_extension() {
        let p = PathBuf::from("/tmp/H5P9-foo");
        assert_eq!(make_signed_path(&p), PathBuf::from("/tmp/signed/H5P9-foo"));
    }

    #[test]
    fn sign_pdf_overlays_real_h5p9() {
        let root = workspace_root();
        let input = root.join("H5P9-\u{4e94}\u{6708}.pdf");
        let sig = root.join("fixtures/zhang-xiang.png");
        if !input.exists() || !sig.exists() {
            eprintln!("test inputs not found, skipping (pdf={input:?}, sig={sig:?})");
            return;
        }

        // Anchor-relative: signature box sits at (anchor_x + 0, anchor_baseline + 22.634).
        // For H5P9 page 2 that resolves to PDF-y ≈ 122.8, matching the cell layout
        // between the "Customer acknowledges" row (PDF y≈179) and the
        // "ESI Engineer's Signature" label (baseline PDF y≈100.2).
        let spec = default_spec();
        let out = sign_pdf(&input, &sig, &spec).expect("overlay succeeds");
        assert!(out.exists(), "output file should exist at {out:?}");
        let in_size = std::fs::metadata(&input).unwrap().len();
        let out_size = std::fs::metadata(&out).unwrap().len();
        assert!(
            out_size > in_size,
            "output ({out_size}B) should be larger than input ({in_size}B) due to image embed"
        );
        // 不清理: 留着给 Step 8 端到端肉眼验证
    }

    // ---- Error path tests (design §3 S5–S8 backing) ----

    #[test]
    fn sign_pdf_returns_pdf_load_failed_on_missing_file() {
        let missing = PathBuf::from("/tmp/this-pdf-does-not-exist-9f3e2c.pdf");
        let sig = workspace_root().join("fixtures/zhang-xiang.png");
        let err = sign_pdf(&missing, &sig, &default_spec()).expect_err("missing PDF should fail");
        assert!(
            matches!(err, SignError::PdfLoadFailed { .. }),
            "expected PdfLoadFailed, got {err:?}"
        );
    }

    #[test]
    fn sign_pdf_returns_page_out_of_range() {
        let root = workspace_root();
        let input = root.join("H5P9-\u{4e94}\u{6708}.pdf");
        let sig = root.join("fixtures/zhang-xiang.png");
        if !input.exists() || !sig.exists() {
            eprintln!("test inputs not found, skipping");
            return;
        }
        let spec = SignSpec {
            page_index: 99, // far beyond 2-page PDF
            ..default_spec()
        };
        let err = sign_pdf(&input, &sig, &spec).expect_err("page 99 should fail");
        assert!(
            matches!(
                err,
                SignError::PageOutOfRange {
                    requested: 99,
                    total: 2,
                    ..
                }
            ),
            "expected PageOutOfRange{{99/2}}, got {err:?}"
        );
    }

    #[test]
    fn sign_pdf_returns_image_load_failed_on_missing_png() {
        let root = workspace_root();
        let input = root.join("H5P9-\u{4e94}\u{6708}.pdf");
        if !input.exists() {
            eprintln!("test PDF not found, skipping");
            return;
        }
        let bad_sig = PathBuf::from("/tmp/this-png-does-not-exist-7a91d4.png");
        let err = sign_pdf(&input, &bad_sig, &default_spec())
            .expect_err("missing PNG should fail");
        assert!(
            matches!(err, SignError::ImageLoadFailed { .. }),
            "expected ImageLoadFailed, got {err:?}"
        );
    }

    #[test]
    fn sign_pdf_returns_output_write_failed_on_readonly_dir() {
        // Strategy: copy the input PDF into a read-only directory, then sign
        // it — `sign_pdf` will try to `mkdir <tmp>/signed` and fail on the
        // read-only parent, which we map to OutputWriteFailed.
        let root = workspace_root();
        let input_master = root.join("H5P9-\u{4e94}\u{6708}.pdf");
        let sig = root.join("fixtures/zhang-xiang.png");
        if !input_master.exists() || !sig.exists() {
            eprintln!("test inputs not found, skipping");
            return;
        }

        let tmp = std::env::temp_dir().join("pdf-sign-core-ro-test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let pdf_in_tmp = tmp.join("input.pdf");
        std::fs::copy(&input_master, &pdf_in_tmp).unwrap();

        // chmod 0o555 (read+exec, no write) on the directory
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(&tmp).unwrap().permissions();
        perm.set_mode(0o555);
        std::fs::set_permissions(&tmp, perm.clone()).unwrap();

        let res = sign_pdf(&pdf_in_tmp, &sig, &default_spec());

        // Restore permissions before assertion so cleanup never gets stuck.
        let mut restore = std::fs::metadata(&tmp).unwrap().permissions();
        restore.set_mode(0o755);
        let _ = std::fs::set_permissions(&tmp, restore);
        let _ = std::fs::remove_dir_all(&tmp);

        let err = res.expect_err("read-only dir should fail to write output");
        assert!(
            matches!(err, SignError::OutputWriteFailed { .. }),
            "expected OutputWriteFailed, got {err:?}"
        );
    }

    #[test]
    fn sign_pdf_returns_anchor_not_found() {
        let root = workspace_root();
        let input = root.join("H5P9-\u{4e94}\u{6708}.pdf");
        let sig = root.join("fixtures/zhang-xiang.png");
        if !input.exists() || !sig.exists() {
            eprintln!("test inputs not found, skipping");
            return;
        }
        let spec = SignSpec {
            anchor_text: "NoSuchAnchorXYZ".to_string(),
            ..default_spec()
        };
        let err = sign_pdf(&input, &sig, &spec).expect_err("bogus anchor should fail");
        assert!(
            matches!(
                &err,
                SignError::AnchorNotFound { anchor, page_index: 1, .. }
                    if anchor == "NoSuchAnchorXYZ"
            ),
            "expected AnchorNotFound, got {err:?}"
        );
    }

    // ---- sign_pdfs batch behavior (design §2.2 流程级约束) ----

    #[test]
    fn sign_pdfs_never_aborts_on_single_failure() {
        let root = workspace_root();
        let good = root.join("H5P9-\u{4e94}\u{6708}.pdf");
        let sig = root.join("fixtures/zhang-xiang.png");
        if !good.exists() || !sig.exists() {
            eprintln!("test inputs not found, skipping");
            return;
        }
        let bad = PathBuf::from("/tmp/missing-pdf-batch-test-3c5e.pdf");
        let inputs = vec![good.clone(), bad.clone(), good.clone()];

        let results = sign_pdfs(&inputs, &sig, &default_spec());
        assert_eq!(results.len(), 3, "should produce one result per input");
        assert!(matches!(results[0], SignResult::Ok { .. }), "first should succeed");
        assert!(
            matches!(
                &results[1],
                SignResult::Err { error: SignError::PdfLoadFailed { .. }, .. }
            ),
            "second should fail with PdfLoadFailed, got {:?}",
            results[1]
        );
        assert!(matches!(results[2], SignResult::Ok { .. }), "third should succeed after middle failure");
    }
}
