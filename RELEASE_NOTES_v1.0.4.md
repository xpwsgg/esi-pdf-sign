# Release v1.0.4

## 🐛 Critical Bug Fix

### Fix incorrect signature placement on PDFs

**Problem**: v1.0.3 fixed the `AnchorNotFound` error for multi-page PDFs, but signatures were placed on the wrong page, resulting in incorrect positioning.

**Root Cause**: The anchor search found the correct anchor text, but the signature overlay was applied to the wrong page because the search didn't return which page the anchor was found on.

**Solution**: Changed anchor search strategy to prioritize the **last page first**, since ESI service report signatures are always on the last page:
1. Try last page first (most common for signatures)
2. Try specified page (backwards compatibility)
3. Try all other pages (fallback)

**Impact**:
- ✅ All PDFs now have signatures on the correct page and position
- ✅ Works for both 2-page PDFs (H5R30, H5R43, H5R54, H5R12) and 3-page PDFs (H5R5)
- ✅ Backwards compatible with existing configurations
- ✅ 16/16 unit tests passing

## 📝 Changes

- `fix: prioritize last page for anchor search (signatures are always on last page)` (b821a0b)
- `chore(release): bump to v1.0.4` (89b2bff)

## 🧪 Testing

Verified all 5 PDFs sign correctly:
- H5R12-六月.pdf ✓
- H5R30-六月.pdf ✓
- H5R43-六月.pdf ✓
- H5R5-六月.pdf ✓ (3 pages)
- H5R54-六月.pdf ✓

## ⚠️ Upgrade Note

**Users on v1.0.3 should upgrade immediately** - v1.0.3 has incorrect signature placement.

---

**Full Changelog**: https://github.com/xpwsgg/esi-pdf-sign/compare/v1.0.3...v1.0.4
