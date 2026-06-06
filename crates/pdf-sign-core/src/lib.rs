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
mod text_scan;
pub mod worktime;

pub use error::SignError;
pub use spec::SignSpec;
pub use worktime::{extract_worktime, extract_worktimes, PdfWorktime, RowDetail, WorktimeResult};

use lopdf::Document;
use std::path::{Path, PathBuf};

/// Outcome of signing a single PDF inside a batch.
#[derive(Debug)]
pub enum SignResult {
    Ok { input: PathBuf, output: PathBuf },
    Err { input: PathBuf, error: SignError },
}

/// Sign one PDF by overlaying one or more signature PNGs, each placed
/// relative to its own anchor text.
///
/// `signatures` is a slice of `(spec, png_path)` pairs — every pair stamps
/// one signature on the document. They are applied in order onto the same
/// in-memory `Document`, then the result is saved once to the `signed/`
/// sub-directory next to the input PDF (file name preserved).
///
/// An empty slice produces a copy of the input PDF in `signed/` (no
/// stamping). Callers that require at least one signature should check
/// before invoking.
pub fn sign_pdf(
    pdf_path: &Path,
    signatures: &[(&SignSpec, &Path)],
) -> Result<PathBuf, SignError> {
    let mut doc = Document::load(pdf_path).map_err(|e| SignError::PdfLoadFailed {
        path: pdf_path.to_path_buf(),
        source: text_scan::lopdf_err_to_io(e),
    })?;

    for (i, (spec, sig_png)) in signatures.iter().enumerate() {
        let (ax, ay) = anchor::find_anchor_baseline(
            &doc,
            pdf_path,
            spec.page_index,
            &spec.anchor_text,
        )?;
        let placement = overlay::Placement {
            page_index: spec.page_index,
            x: ax as f32 + spec.dx,
            y: ay as f32 + spec.dy,
            width: spec.width,
            height: spec.height,
        };
        // Each signature needs its own XObject name; reusing one name across
        // overlays would let the second registration overwrite the first in
        // the page's Resources/XObject dict so both `Do` ops would render the
        // last image.
        let xobject_name = format!("SigImg{i}");
        overlay::overlay_signature_on_page(
            &mut doc,
            pdf_path,
            sig_png,
            &placement,
            xobject_name.as_bytes(),
        )?;
    }

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

/// Sign many PDFs serially with the same signature set.
///
/// Per design §2.2 flow constraints: this function **never panics** and
/// **never returns Err**; each PDF's outcome is collected independently
/// so a single failure does not abort the batch.
pub fn sign_pdfs(
    pdf_paths: &[PathBuf],
    signatures: &[(&SignSpec, &Path)],
) -> Vec<SignResult> {
    pdf_paths
        .iter()
        .map(|input| match sign_pdf(input, signatures) {
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
        let out = sign_pdf(&input, &[(&spec, &sig)]).expect("overlay succeeds");
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
        let err = sign_pdf(&missing, &[(&default_spec(), &sig)]).expect_err("missing PDF should fail");
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
        let err = sign_pdf(&input, &[(&spec, &sig)]).expect_err("page 99 should fail");
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
        let err = sign_pdf(&input, &[(&default_spec(), &bad_sig)])
            .expect_err("missing PNG should fail");
        assert!(
            matches!(err, SignError::ImageLoadFailed { .. }),
            "expected ImageLoadFailed, got {err:?}"
        );
    }

    #[test]
    #[cfg(unix)]
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

        let tmp = tempfile::tempdir().unwrap();
        let tmp_path = tmp.path();
        let pdf_in_tmp = tmp_path.join("input.pdf");
        std::fs::copy(&input_master, &pdf_in_tmp).unwrap();

        // chmod 0o555 (read+exec, no write) on the directory
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(tmp_path).unwrap().permissions();
        perm.set_mode(0o555);
        std::fs::set_permissions(tmp_path, perm.clone()).unwrap();

        let res = sign_pdf(&pdf_in_tmp, &[(&default_spec(), &sig)]);

        // Restore permissions before assertion so cleanup never gets stuck.
        let mut restore = std::fs::metadata(tmp_path).unwrap().permissions();
        restore.set_mode(0o755);
        let _ = std::fs::set_permissions(tmp_path, restore);

        let err = res.expect_err("read-only dir should fail to write output");
        assert!(
            matches!(err, SignError::OutputWriteFailed { .. }),
            "expected OutputWriteFailed, got {err:?}"
        );
        // tmp is automatically cleaned up when dropped
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
        let err = sign_pdf(&input, &[(&spec, &sig)]).expect_err("bogus anchor should fail");
        assert!(
            matches!(
                &err,
                SignError::AnchorNotFound { anchor, page_index: 1, .. }
                    if anchor == "NoSuchAnchorXYZ"
            ),
            "expected AnchorNotFound, got {err:?}"
        );
    }

    #[test]
    fn sign_pdf_signs_engineer_and_customer_slots() {
        let root = workspace_root();
        let input = root.join("H5P9-\u{4e94}\u{6708}.pdf");
        let sig = root.join("fixtures/zhang-xiang.png");
        if !input.exists() || !sig.exists() {
            eprintln!("test inputs not found, skipping");
            return;
        }
        let engineer = default_spec();
        let customer = SignSpec {
            anchor_text: "Authorised Customer's Signature".to_string(),
            ..default_spec()
        };
        let out =
            sign_pdf(&input, &[(&engineer, &sig), (&customer, &sig)]).expect("dual sign succeeds");
        assert!(out.exists(), "output exists at {out:?}");

        // Regression guard: each signature must register a distinct XObject
        // name. Reload and inspect page 2's content streams — both
        // `/SigImg0 Do` and `/SigImg1 Do` should appear, otherwise the
        // second overlay overwrote the first and both rendered the same
        // image.
        let doc = lopdf::Document::load(&out).expect("reload signed pdf");
        let pages = doc.get_pages();
        let page_id = *pages.get(&2).expect("page 2 exists");
        let bytes = doc.get_page_content(page_id).expect("page content");
        let stream = String::from_utf8_lossy(&bytes);
        assert!(
            stream.contains("/SigImg0 Do"),
            "page 2 content should reference /SigImg0, got: {stream}"
        );
        assert!(
            stream.contains("/SigImg1 Do"),
            "page 2 content should reference /SigImg1, got: {stream}"
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

        let results = sign_pdfs(&inputs, &[(&default_spec(), &sig)]);
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
