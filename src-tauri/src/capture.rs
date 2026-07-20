use image::{DynamicImage, RgbaImage};
use xcap::Monitor;

/// A screen capture stored in memory along with the monitor's geometry.
pub struct Capture {
    pub image: DynamicImage,
    pub width: u32,
    pub height: u32,
}

/// Capture the monitor containing the point `(x, y)` (physical pixels).
///
/// On multi-monitor setups we capture the display under the cursor so the
/// overlay appears where the user is actually looking. Falls back to the
/// primary monitor (then the first enumerated) if no monitor contains the point.
pub fn capture_monitor_at(x: i32, y: i32) -> Result<Capture, String> {
    let monitors = Monitor::all().map_err(|e| format!("enumerate monitors: {e}"))?;

    let monitor = Monitor::from_point(x, y)
        .ok()
        .or_else(|| monitors.iter().find(|m| m.is_primary().unwrap_or(false)).cloned())
        .or_else(|| monitors.first().cloned())
        .ok_or_else(|| "no monitors detected".to_string())?;

    capture_monitor(&monitor)
}

/// Capture the primary monitor as a `DynamicImage`.
///
/// Fallback used when the cursor position is unavailable.
pub fn capture_primary_monitor() -> Result<Capture, String> {
    let monitors = Monitor::all().map_err(|e| format!("enumerate monitors: {e}"))?;

    let monitor = monitors
        .iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .or_else(|| monitors.first())
        .ok_or_else(|| "no monitors detected".to_string())?
        .clone();

    capture_monitor(&monitor)
}

/// Capture a single monitor into memory.
fn capture_monitor(monitor: &Monitor) -> Result<Capture, String> {
    let buffer: RgbaImage = monitor
        .capture_image()
        .map_err(|e| format!("capture: {e}"))?;

    // Use the actual captured image's dimensions (physical pixels).
    // xcap's `monitor.width()/height()` return logical points, which mismatches
    // the physical-pixel image on HiDPI displays and breaks coord scaling.
    let width = buffer.width();
    let height = buffer.height();

    Ok(Capture {
        image: DynamicImage::ImageRgba8(buffer),
        width,
        height,
    })
}

/// Crop a capture to the given region (in physical pixels).
/// Coordinates are clamped to the image bounds defensively.
pub fn crop(capture: &Capture, x: u32, y: u32, w: u32, h: u32) -> DynamicImage {
    let max_w = capture.width.saturating_sub(x);
    let max_h = capture.height.saturating_sub(y);
    let w = w.min(max_w).max(1);
    let h = h.min(max_h).max(1);
    capture.image.crop_imm(x, y, w, h)
}
