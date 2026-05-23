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

    register_xobject_in_page_resources(doc, page_id, xobject_name, main_id);
    append_draw_op_to_page_contents(doc, page_id, xobject_name, placement);
    Ok(())
}

fn load_png_rgba(path: &Path) -> Result<(u32, u32, Vec<u8>, Vec<u8>), SignError> {
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
) {
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
            add_xobject_into_dict_object(doc, res_id, name, xobject_id);
        }
        Some(Object::Dictionary(_)) | None => {
            // modify inline / create new
            let page = doc.get_object_mut(page_id).expect("page exists");
            let page_dict = page.as_dict_mut().expect("page is dict");
            let mut resources = page_dict
                .get(b"Resources")
                .ok()
                .and_then(|o| o.as_dict().ok())
                .cloned()
                .unwrap_or_else(Dictionary::new);
            add_xobject_into_resources_dict(&mut resources, name, xobject_id);
            page_dict.set("Resources", Object::Dictionary(resources));
        }
        Some(_) => {
            // Unexpected Resources type; create a fresh inline dict.
            let page = doc.get_object_mut(page_id).expect("page exists");
            let page_dict = page.as_dict_mut().expect("page is dict");
            let mut resources = Dictionary::new();
            add_xobject_into_resources_dict(&mut resources, name, xobject_id);
            page_dict.set("Resources", Object::Dictionary(resources));
        }
    }
}

fn add_xobject_into_dict_object(
    doc: &mut Document,
    dict_id: ObjectId,
    name: &[u8],
    xobject_id: ObjectId,
) {
    let obj = doc.get_object_mut(dict_id).expect("resources exists");
    let dict = obj.as_dict_mut().expect("resources is dict");
    add_xobject_into_resources_dict(dict, name, xobject_id);
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
) {
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

    let page = doc.get_object_mut(page_id).expect("page exists");
    let page_dict = page.as_dict_mut().expect("page is dict");
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
}
