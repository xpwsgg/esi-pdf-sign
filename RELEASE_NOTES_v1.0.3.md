# Release v1.0.3

## 🐛 Bug Fixes

### Fix multi-page PDF signature failure

**Problem**: H5R5-六月.pdf (3 pages) failed to sign with `AnchorNotFound` error, while other 2-page PDFs signed successfully.

**Root Cause**: The code hardcoded anchor search on page 2 (`page_index: 1`), but H5R5's signature field was on page 3 (`page_index: 2`).

**Solution**: Modified `find_anchor_baseline` function to automatically search all pages when anchor is not found on the specified page.

**Impact**:
- ✅ All PDFs now sign successfully regardless of page count
- ✅ Backwards compatible: tries specified page first for performance
- ✅ Tested with H5R12, H5R30, H5R43, H5R5 (3 pages), H5R54
- ✅ 16/16 unit tests passing

## 📝 Changes

- `fix: auto-search all pages when anchor not found on specified page` (54df5da)
- `chore(release): bump to v1.0.3` (e45a1a8)

## 🧪 Testing

- Added regression test: `sign_pdf_finds_anchor_on_any_page_when_page_index_wrong`
- Added debug tool: `find_anchor.rs` example for troubleshooting
- All existing tests continue to pass

## 📦 Files Changed

- `crates/pdf-sign-core/src/anchor.rs` - Enhanced anchor search logic
- `crates/pdf-sign-core/src/lib.rs` - Added regression test
- `crates/pdf-sign-core/examples/find_anchor.rs` - New debug tool
- `CHANGELOG-fixes.md` - Updated changelog

---

**Full Changelog**: https://github.com/xpwsgg/esi-pdf-sign/compare/v1.0.2...v1.0.3
