use image::{DynamicImage, ImageFormat};
#[cfg(not(target_os = "macos"))]
use image::{GenericImageView, imageops::FilterType};
use std::io::Cursor;

pub fn extract_text(image: &DynamicImage) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    return extract_text_vision(image);

    #[cfg(not(target_os = "macos"))]
    return extract_text_tesseract(image);
}

// ── macOS: Vision framework ───────────────────────────────────────────────────

#[cfg(target_os = "macos")]
extern "C" {
    fn vision_ocr_from_png_bytes(
        ptr: *const u8,
        len: usize,
        out_ptr: *mut *mut std::ffi::c_char,
    ) -> usize;
    fn vision_ocr_free(ptr: *mut std::ffi::c_char);
}

#[cfg(target_os = "macos")]
fn extract_text_vision(image: &DynamicImage) -> Result<String, String> {
    let bytes = encode_png(image)?;

    let mut raw_ptr: *mut std::ffi::c_char = std::ptr::null_mut();
    let _len = unsafe {
        vision_ocr_from_png_bytes(bytes.as_ptr(), bytes.len(), &mut raw_ptr)
    };

    if raw_ptr.is_null() {
        return Err("Vision OCR returned null — image decode failed".into());
    }

    let text = unsafe {
        let s = std::ffi::CStr::from_ptr(raw_ptr)
            .to_string_lossy()
            .into_owned();
        vision_ocr_free(raw_ptr);
        s
    };

    Ok(text.trim().to_string())
}

// ── non-macOS: Tesseract fallback ─────────────────────────────────────────────

#[cfg(not(target_os = "macos"))]
fn extract_text_tesseract(image: &DynamicImage) -> Result<String, String> {
    use tesseract::Tesseract;

    let processed = preprocess_for_tesseract(image);
    let bytes = encode_png(&processed)?;

    let text = Tesseract::new(None, Some("eng"))
        .map_err(|e| format!("tesseract init: {e}"))?
        .set_image_from_mem(&bytes)
        .map_err(|e| format!("set image: {e}"))?
        .get_text()
        .map_err(|e| format!("recognize: {e}"))?;

    Ok(text.trim().replace('\u{000C}', "").to_string())
}

#[cfg(not(target_os = "macos"))]
fn preprocess_for_tesseract(image: &DynamicImage) -> DynamicImage {
    let (w, h) = image.dimensions();
    let min_dim = w.min(h);
    let scale: f32 = if min_dim < 200 { 3.0 } else if min_dim < 400 { 2.0 } else { 1.0 };

    let scaled = if (scale - 1.0_f32).abs() > f32::EPSILON {
        image.resize((w as f32 * scale) as u32, (h as f32 * scale) as u32, FilterType::Lanczos3)
    } else {
        image.clone()
    };

    DynamicImage::ImageLuma8(scaled.to_luma8())
}

// ── shared ────────────────────────────────────────────────────────────────────

fn encode_png(image: &DynamicImage) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::new());
    image
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| format!("encode png: {e}"))?;
    Ok(buf.into_inner())
}
