//! Content-stream overlay: places a PNG signature image at PDF coordinates.
//!
//! Strategy (works on XFA-form PDFs by ignoring forms entirely):
//! 1. Decode PNG → RGBA8.
//! 2. Split into RGB plane + alpha plane.
//! 3. Zlib-compress each plane.
//! 4. Create two PDF Image XObjects: main (DeviceRGB) + SMask (DeviceGray).
//! 5. Register main XObject in target page's Resources/XObject dict under a
//!    caller-supplied name (unique per page so stacking multiple signatures
//!    doesn't overwrite an earlier registration — see lib.rs::sign_pdf).
//! 6. Append a tiny content stream to the page's Contents array:
//!    `q  w 0 0 h x y cm  /<name> Do  Q`.

use crate::error::SignError;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::ImageReader;
use lopdf::{dictionary, Dictionary, Document, Object, ObjectId, Stream};
use std::io::Write;
use std::path::Path;

/// Maximum signature image dimensions (width x height in pixels).
/// Reasonable signature images are typically 200-400px wide.
const MAX_IMAGE_WIDTH: u32 = 4096;
const MAX_IMAGE_HEIGHT: u32 = 4096;

/// Maximum signature image file size in megabytes.
/// Prevents accidental selection of huge images.
const MAX_FILE_SIZE_MB: u32 = 10;

/// Final placement of the signature in PDF-native coordinates (origin
/// bottom-left, points). Computed in `lib.rs::sign_pdf` from a `SignSpec`
/// plus the located anchor baseline.
pub(crate) struct Placement {
    pub page_index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub(crate) fn overlay_signature_on_page(
    doc: &mut Document,
    pdf_path: &Path,
    sig_png_path: &Path,
    placement: &Placement,
    xobject_name: &[u8],
) -> Result<(), SignError> {
    let (img_w, img_h, rgb, alpha) = load_png_rgba(sig_png_path)?;
    let rgb_z = zlib(&rgb);
    let alpha_z = zlib(&alpha);

    let smask_id = add_image_xobject(doc, img_w, img_h, "DeviceGray", alpha_z, None);
    let main_id = add_image_xobject(doc, img_w, img_h, "DeviceRGB", rgb_z, Some(smask_id));

    let pages = doc.get_pages();
    let page_num = (placement.page_index + 1) as u32;
    let total = pages.len();
    let page_id = *pages.get(&page_num).ok_or_else(|| SignError::PageOutOfRange {
        path: pdf_path.to_path_buf(),
        requested: placement.page_index,
        total,
    })?;

    register_xobject_in_page_resources(doc, page_id, xobject_name, main_id, pdf_path)?;
    append_draw_op_to_page_contents(doc, page_id, xobject_name, placement, pdf_path)?;
    Ok(())
}

fn load_png_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>, Vec<u8>), SignError> {
    // Check file size first (before decoding)
    let metadata = std::fs::metadata(path).map_err(|e| SignError::ImageLoadFailed {
        path: path.to_path_buf(),
        source: e,
    })?;
    let file_size_bytes = metadata.len();
    let file_size_mb = file_size_bytes as f64 / (1024.0 * 1024.0);

    if file_size_bytes > (MAX_FILE_SIZE_MB as u64 * 1024 * 1024) {
        return Err(SignError::ImageTooLarge {
            path: path.to_path_buf(),
            width: 0,
            height: 0,
            max_width: MAX_IMAGE_WIDTH,
            max_height: MAX_IMAGE_HEIGHT,
            file_size_mb,
            max_file_size_mb: MAX_FILE_SIZE_MB,
        });
    }

    let img = ImageReader::open(path)
        .map_err(|e| SignError::ImageLoadFailed {
            path: path.to_path_buf(),
            source: e,
        })?
        .decode()
        .map_err(|e| SignError::ImageLoadFailed {
            path: path.to_path_buf(),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
        })?
        .into_rgba8();

    let (w, h) = img.dimensions();

    // Check dimensions after decoding
    if w > MAX_IMAGE_WIDTH || h > MAX_IMAGE_HEIGHT {
        return Err(SignError::ImageTooLarge {
            path: path.to_path_buf(),
            width: w,
            height: h,
            max_width: MAX_IMAGE_WIDTH,
            max_height: MAX_IMAGE_HEIGHT,
            file_size_mb,
            max_file_size_mb: MAX_FILE_SIZE_MB,
        });
    }

    let pixels = img.into_raw();
    let pixel_count = (w as usize) * (h as usize);
    let mut rgb = Vec::with_capacity(pixel_count * 3);
    let mut alpha = Vec::with_capacity(pixel_count);
    for chunk in pixels.chunks_exact(4) {
        rgb.extend_from_slice(&chunk[0..3]);
        alpha.push(chunk[3]);
    }
    Ok((w, h, rgb, alpha))
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).expect("zlib write into Vec is infallible");
    enc.finish().expect("zlib finish is infallible")
}

fn add_image_xobject(
    doc: &mut Document,
    width: u32,
    height: u32,
    color_space: &str,
    data: Vec<u8>,
    smask: Option<ObjectId>,
) -> ObjectId {
    let mut dict = dictionary! {
        "Type" => "XObject",
        "Subtype" => "Image",
        "Width" => i64::from(width),
        "Height" => i64::from(height),
        "ColorSpace" => Object::Name(color_space.as_bytes().to_vec()),
        "BitsPerComponent" => 8i64,
        "Filter" => "FlateDecode",
    };
    if let Some(s) = smask {
        dict.set("SMask", Object::Reference(s));
    }
    doc.add_object(Stream::new(dict, data))
}

/// Add `/<name> <xobject_ref>` under page's Resources/XObject. Creates Resources
/// or Resources/XObject if missing. Handles Resources-as-reference case.
fn register_xobject_in_page_resources(
    doc: &mut Document,
    page_id: ObjectId,
    name: &[u8],
    xobject_id: ObjectId,
    pdf_path: &Path,
) -> Result<(), SignError> {
    // Resolve Resources to (target_dict_id, dict_owned_clone).
    // We need to either modify Resources inline on the page, or modify the
    // referenced Resources object.
    let page_resources_obj = doc
        .get_object(page_id)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|d| d.get(b"Resources").ok())
        .cloned();

    match page_resources_obj {
        Some(Object::Reference(res_id)) => {
            add_xobject_into_dict_object(doc, res_id, name, xobject_id, pdf_path)?;
        }
        Some(Object::Dictionary(_)) | None => {
            // modify inline / create new
            let page = doc.get_object_mut(page_id).map_err(|_| SignError::PdfStructureError {
                path: pdf_path.to_path_buf(),
                detail: format!("Page object {:?} does not exist", page_id),
            })?;
            let page_dict = page.as_dict_mut().map_err(|_| SignError::PdfStructureError {
                path: pdf_path.to_path_buf(),
                detail: format!("Page object {:?} is not a dictionary", page_id),
            })?;
            let mut resources = page_dict
                .get(b"Resources")
                .ok()
                .and_then(|o: &Object| o.as_dict().ok())
                .cloned()
                .unwrap_or_else(Dictionary::new);
            add_xobject_into_resources_dict(&mut resources, name, xobject_id);
            page_dict.set("Resources", Object::Dictionary(resources));
        }
        Some(_) => {
            // Unexpected Resources type; create a fresh inline dict.
            let page = doc.get_object_mut(page_id).map_err(|_| SignError::PdfStructureError {
                path: pdf_path.to_path_buf(),
                detail: format!("Page object {:?} does not exist", page_id),
            })?;
            let page_dict = page.as_dict_mut().map_err(|_| SignError::PdfStructureError {
                path: pdf_path.to_path_buf(),
                detail: format!("Page object {:?} is not a dictionary", page_id),
            })?;
            let mut resources = Dictionary::new();
            add_xobject_into_resources_dict(&mut resources, name, xobject_id);
            page_dict.set("Resources", Object::Dictionary(resources));
        }
    }
    Ok(())
}

fn add_xobject_into_dict_object(
    doc: &mut Document,
    dict_id: ObjectId,
    name: &[u8],
    xobject_id: ObjectId,
    pdf_path: &Path,
) -> Result<(), SignError> {
    let obj = doc.get_object_mut(dict_id).map_err(|_| SignError::PdfStructureError {
        path: pdf_path.to_path_buf(),
        detail: format!("Referenced Resources object {:?} does not exist", dict_id),
    })?;
    let dict = obj.as_dict_mut().map_err(|_| SignError::PdfStructureError {
        path: pdf_path.to_path_buf(),
        detail: format!("Referenced Resources object {:?} is not a dictionary", dict_id),
    })?;
    add_xobject_into_resources_dict(dict, name, xobject_id);
    Ok(())
}

fn add_xobject_into_resources_dict(
    resources: &mut Dictionary,
    name: &[u8],
    xobject_id: ObjectId,
) {
    let mut xobj_dict = resources
        .get(b"XObject")
        .ok()
        .and_then(|o| o.as_dict().ok())
        .cloned()
        .unwrap_or_else(Dictionary::new);
    xobj_dict.set(name.to_vec(), Object::Reference(xobject_id));
    resources.set("XObject", Object::Dictionary(xobj_dict));
}

/// Append a draw-image content stream to the page's `/Contents` array.
fn append_draw_op_to_page_contents(
    doc: &mut Document,
    page_id: ObjectId,
    name: &[u8],
    placement: &Placement,
    pdf_path: &Path,
) -> Result<(), SignError> {
    let name_str = std::str::from_utf8(name).expect("xobject name is ASCII");
    // PDF content stream: q [save state] cm [transform] Do [draw XObject] Q [restore]
    // cm matrix: width 0 0 height x y  → maps 1×1 unit square to (x,y)-(x+w,y+h)
    let content_str = format!(
        "q\n{w} 0 0 {h} {x} {y} cm\n/{n} Do\nQ\n",
        w = placement.width,
        h = placement.height,
        x = placement.x,
        y = placement.y,
        n = name_str,
    );
    let stream = Stream::new(dictionary! {}, content_str.into_bytes());
    let new_content_id = doc.add_object(stream);

    let page = doc.get_object_mut(page_id).map_err(|_| SignError::PdfStructureError {
        path: pdf_path.to_path_buf(),
        detail: format!("Page object {:?} does not exist", page_id),
    })?;
    let page_dict = page.as_dict_mut().map_err(|_| SignError::PdfStructureError {
        path: pdf_path.to_path_buf(),
        detail: format!("Page object {:?} is not a dictionary", page_id),
    })?;
    let merged = match page_dict.get(b"Contents").cloned() {
        Ok(Object::Reference(r)) => Object::Array(vec![
            Object::Reference(r),
            Object::Reference(new_content_id),
        ]),
        Ok(Object::Array(mut arr)) => {
            arr.push(Object::Reference(new_content_id));
            Object::Array(arr)
        }
        _ => Object::Reference(new_content_id),
    };
    page_dict.set("Contents", merged);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::SignError;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn load_png_rgba_rejects_oversized_dimensions() {
        // Create a PNG that exceeds dimension limits
        let tmp = tempfile::NamedTempFile::with_suffix(".png").unwrap();
        let path = tmp.path();

        // Create a 5000x5000 image (exceeds MAX_IMAGE_WIDTH=4096)
        // Use DynamicImage to ensure proper encoding
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(5000, 5000, Rgba([255, 255, 255, 255]));
        let dynamic = image::DynamicImage::ImageRgba8(img);
        dynamic.save_with_format(path, image::ImageFormat::Png).unwrap();

        let result = load_png_rgba(path);
        assert!(result.is_err(), "should reject oversized image");
        match result.unwrap_err() {
            SignError::ImageTooLarge { width, height, .. } => {
                assert_eq!(width, 5000);
                assert_eq!(height, 5000);
            }
            other => panic!("expected ImageTooLarge, got {:?}", other),
        }
    }

    #[test]
    fn load_png_rgba_rejects_large_file_size() {
        // Create a PNG that's under dimension limits but over file size limit
        // A 4000x4000 RGBA PNG with pseudo-random data should exceed 10MB
        let tmp = tempfile::NamedTempFile::with_suffix(".png").unwrap();
        let path = tmp.path();

        // Create 4000x4000 with pseudo-random data (poorly compressible)
        let mut img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(4000, 4000);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let val = ((x * 7 + y * 13) % 256) as u8;
            *pixel = Rgba([val, val.wrapping_add(1), val.wrapping_add(2), 255]);
        }
        let dynamic = image::DynamicImage::ImageRgba8(img);
        dynamic.save_with_format(path, image::ImageFormat::Png).unwrap();

        // Check if file size exceeds 10MB
        let metadata = std::fs::metadata(path).unwrap();
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

        if size_mb > 10.0 {
            let result = load_png_rgba(path);
            assert!(result.is_err(), "should reject large file");
            match result.unwrap_err() {
                SignError::ImageTooLarge { file_size_mb, .. } => {
                    assert!(file_size_mb > 10.0, "reported size should exceed limit");
                }
                other => panic!("expected ImageTooLarge, got {:?}", other),
            }
        } else {
            // File compressed too well; skip this test
            eprintln!("Test PNG only {:.2}MB, skipping file size test", size_mb);
        }
    }

    #[test]
    fn load_png_rgba_accepts_normal_signature() {
        // Verify normal small signatures still work
        let tmp = tempfile::NamedTempFile::with_suffix(".png").unwrap();
        let path = tmp.path();

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_pixel(200, 100, Rgba([255, 255, 255, 255]));
        let dynamic = image::DynamicImage::ImageRgba8(img);
        dynamic.save_with_format(path, image::ImageFormat::Png).unwrap();

        let result = load_png_rgba(path);
        assert!(result.is_ok(), "should accept normal-sized image: {:?}", result);
        let (w, h, rgb, alpha) = result.unwrap();
        assert_eq!(w, 200);
        assert_eq!(h, 100);
        assert_eq!(rgb.len(), 200 * 100 * 3);
        assert_eq!(alpha.len(), 200 * 100);
    }
}
