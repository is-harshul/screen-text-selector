mod capture;
mod format;
mod ocr;

use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, State,
};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{
    Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};
use tauri_plugin_notification::NotificationExt;

/// Holds the current screen capture between the hotkey press and the
/// user committing a selection in the overlay window.
struct AppState {
    capture: Mutex<Option<capture::Capture>>,
}

/// Make the overlay window able to appear over a native-fullscreen app.
///
/// When another app is in macOS native fullscreen it occupies its own Space.
/// A normal window has a "home" Space, so showing it would switch Spaces away
/// from the fullscreen app instead of drawing on top of it. Setting
/// `canJoinAllSpaces` + `fullScreenAuxiliary` and raising the window level lets
/// the overlay render on whichever Space is currently active — including a
/// fullscreen app's Space.
#[cfg(target_os = "macos")]
fn configure_overlay_spaces(overlay: &tauri::WebviewWindow) {
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

    let Ok(ptr) = overlay.ns_window() else { return };
    if ptr.is_null() {
        return;
    }
    // SAFETY: `ns_window()` returns the window's live `NSWindow` for the life of
    // the window; we only touch it on the main thread (Tauri's event loop).
    let ns_window: &NSWindow = unsafe { &*(ptr as *const AnyObject as *const NSWindow) };

    let behavior = NSWindowCollectionBehavior::CanJoinAllSpaces
        | NSWindowCollectionBehavior::FullScreenAuxiliary
        | NSWindowCollectionBehavior::Stationary
        | NSWindowCollectionBehavior::IgnoresCycle;
    unsafe { ns_window.setCollectionBehavior(behavior) };

    // Float above normal windows (and the fullscreen app's content) — pop-up
    // menu level (101) sits above NSMainMenuWindowLevel without being intrusive.
    unsafe { ns_window.setLevel(101) };
}

/// Hide the overlay window cleanly, exiting macOS simple-fullscreen first.
#[tauri::command]
fn close_overlay(app: AppHandle) -> Result<(), String> {
    if let Some(overlay) = app.get_webview_window("overlay") {
        #[cfg(target_os = "macos")]
        let _ = overlay.set_simple_fullscreen(false);
        overlay.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Returns the (width, height) of the active capture in physical pixels
/// so the frontend can compute scale factors for HiDPI displays.
#[tauri::command]
fn get_capture_dimensions(state: State<AppState>) -> Result<(u32, u32), String> {
    let cap = state.capture.lock().map_err(|e| e.to_string())?;
    let cap = cap.as_ref().ok_or("no active capture")?;
    Ok((cap.width, cap.height))
}

/// Crop the active capture to the given region, run OCR, and copy the
/// result to the clipboard. Hides the overlay before showing the
/// notification so the toast is visible against the user's actual desktop.
#[tauri::command]
async fn process_selection(
    app: AppHandle,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    format: String,
) -> Result<String, String> {
    // Pull the capture out of state on the main thread, then hand it
    // to a blocking task for OCR (CPU-heavy).
    let cropped = {
        let state: State<AppState> = app.state();
        let cap = state.capture.lock().map_err(|e| e.to_string())?;
        let cap = cap.as_ref().ok_or("no active capture")?;
        let c = capture::crop(cap, x, y, w, h);
        // Debug: write full capture + cropped region to /tmp for inspection.
        let _ = cap.image.save("/tmp/text-extractor-capture.png");
        let _ = c.save("/tmp/text-extractor-crop.png");
        log::warn!(
            "selection: x={x} y={y} w={w} h={h} | capture: {}x{}",
            cap.width, cap.height
        );
        c
    };

    // Hide the overlay BEFORE OCR finishes so the notification (and any
    // visual feedback) appears against the user's real desktop.
    if let Some(overlay) = app.get_webview_window("overlay") {
        #[cfg(target_os = "macos")]
        let _ = overlay.set_simple_fullscreen(false);
        let _ = overlay.hide();
    }

    let text = tokio::task::spawn_blocking(move || ocr::extract_text(&cropped))
        .await
        .map_err(|e| format!("ocr task panicked: {e}"))??;

    if text.is_empty() {
        let _ = app
            .notification()
            .builder()
            .title("Text Extractor")
            .body("No text detected in selection")
            .show();
    } else {
        match format.as_str() {
            "markdown" => {
                let md = crate::format::to_markdown(&text);
                app.clipboard()
                    .write_text(md)
                    .map_err(|e| format!("clipboard: {e}"))?;
            }
            "html" => {
                let html = crate::format::to_html(&text);
                app.clipboard()
                    .write_html(html, Some(text.clone()))
                    .map_err(|e| format!("clipboard: {e}"))?;
            }
            _ => {
                app.clipboard()
                    .write_text(text.clone())
                    .map_err(|e| format!("clipboard: {e}"))?;
            }
        }
        let preview: String = text.chars().take(80).collect();
        let body = if text.chars().count() > 80 {
            format!("{preview}…")
        } else {
            preview
        };
        let _ = app
            .notification()
            .builder()
            .title("Copied to clipboard")
            .body(body)
            .show();
    }

    Ok(text)
}

// Check if the app has Screen Recording permission (macOS only).
#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

fn has_screen_recording_permission() -> bool {
    #[cfg(target_os = "macos")]
    {
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

/// Trigger a screen capture and reveal the overlay window.
/// Called from both the global hotkey handler and the tray menu.
fn trigger_capture(app: &AppHandle) {
    let app = app.clone();

    // Grab the cursor position now so we capture the monitor the user is
    // actually looking at. `None` -> fall back to the primary monitor.
    let cursor = app
        .cursor_position()
        .ok()
        .map(|p| (p.x as i32, p.y as i32));

    tauri::async_runtime::spawn(async move {
        // xcap is blocking — run it on a blocking thread
        let cap_result = tokio::task::spawn_blocking(move || match cursor {
            Some((x, y)) => capture::capture_monitor_at(x, y),
            None => capture::capture_primary_monitor(),
        })
        .await;

        let cap = match cap_result {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                log::error!("capture failed: {e}");
                let _ = app
                    .notification()
                    .builder()
                    .title("Capture failed")
                    .body(format!("Couldn't capture screen: {e}"))
                    .show();
                return;
            }
            Err(e) => {
                log::error!("capture task panicked: {e}");
                return;
            }
        };

        // Stash the capture in state
        if let Some(state) = app.try_state::<AppState>() {
            if let Ok(mut slot) = state.capture.lock() {
                *slot = Some(cap);
            }
        }

        // Reveal the overlay sized to the primary monitor.
        // We avoid `fullscreen: true` because on macOS that triggers native
        // fullscreen (which animates into a new Space). Sizing to monitor
        // bounds + simple_fullscreen keeps the overlay in the current Space.
        if let Some(overlay) = app.get_webview_window("overlay") {
            // Position on the monitor under the cursor (same one we captured),
            // falling back to primary if the cursor position was unavailable.
            let target_monitor = match cursor {
                Some((x, y)) => overlay
                    .monitor_from_point(x as f64, y as f64)
                    .ok()
                    .flatten()
                    .or_else(|| overlay.primary_monitor().ok().flatten()),
                None => overlay.primary_monitor().ok().flatten(),
            };
            if let Some(monitor) = target_monitor {
                let pos = monitor.position();
                let size = monitor.size();
                let _ = overlay.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
                let _ = overlay.set_size(tauri::PhysicalSize::new(size.width, size.height));
            }
            // Allow the overlay onto a fullscreen app's Space BEFORE showing,
            // otherwise showing it would switch Spaces to the overlay's home.
            #[cfg(target_os = "macos")]
            configure_overlay_spaces(&overlay);
            #[cfg(target_os = "macos")]
            let _ = overlay.set_simple_fullscreen(true);
            let _ = overlay.show();
            let _ = overlay.set_focus();
            // Tell the frontend the capture is ready so it can fetch dimensions
            let _ = app.emit("capture-ready", ());
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            capture: Mutex::new(None),
        })
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    let target = Shortcut::new(
                        Some(Modifiers::CONTROL | Modifiers::SHIFT),
                        Code::KeyT,
                    );
                    if shortcut == &target {
                        trigger_capture(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Tray-resident: hide from dock and remove the auto-generated
            // menu bar app entry (otherwise we get a duplicate icon next to
            // the real tray icon on macOS).
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // --- Global hotkey ----------------------------------------------------
            // Note: on macOS this maps to Cmd+Shift+T because Tauri normalises
            // CONTROL → Cmd on Apple platforms.
            let shortcut = Shortcut::new(
                Some(Modifiers::CONTROL | Modifiers::SHIFT),
                Code::KeyT,
            );
            app.global_shortcut().register(shortcut)?;

            // --- Tray icon --------------------------------------------------------
            let capture_item = MenuItem::with_id(
                app,
                "capture",
                "Capture text  (Ctrl+Shift+T)",
                true,
                None::<&str>,
            )?;
            let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            let autostart_label = if autostart_enabled {
                "✓ Launch at Login"
            } else {
                "  Launch at Login"
            };
            let autostart_item = MenuItem::with_id(
                app,
                "autostart",
                autostart_label,
                true,
                None::<&str>,
            )?;
            let perm_granted = has_screen_recording_permission();
            let perm_label = if perm_granted {
                "✓ Screen Recording"
            } else {
                "⚠ Screen Recording — Click to enable"
            };
            let perm_item = MenuItem::with_id(app, "screen_perm", perm_label, true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let separator2 = PredefinedMenuItem::separator(app)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&capture_item, &separator, &perm_item, &autostart_item, &separator2, &quit_item])?;

            let _tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .tooltip("Text Extractor")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "capture" => trigger_capture(app),
                    "screen_perm" => {
                        if !has_screen_recording_permission() {
                            let _ = std::process::Command::new("open")
                                .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture")
                                .spawn();
                        }
                    }
                    "autostart" => {
                        let al = app.autolaunch();
                        let enabled = al.is_enabled().unwrap_or(false);
                        if enabled {
                            let _ = al.disable();
                        } else {
                            let _ = al.enable();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    // Left-click the tray icon also triggers a capture
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        trigger_capture(tray.app_handle());
                    }
                })
                .build(app)?;

            // --- Autostart: enable by default on first run -----------------------
            let autostart = app.autolaunch();
            if !autostart.is_enabled().unwrap_or(false) {
                let _ = autostart.enable();
            }

            // --- Make sure overlay starts hidden ---------------------------------
            if let Some(overlay) = app.get_webview_window("overlay") {
                let _ = overlay.hide();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_capture_dimensions,
            process_selection,
            close_overlay
        ])
        .build(tauri::generate_context!())
        .expect("error building tauri application")
        // Prevent the app from quitting when all windows are closed —
        // this is a tray-resident utility.
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                // Allow explicit quits (from the tray menu calling app.exit)
                // but ignore window-close-driven exits.
                let _ = api;
            }
        });
}
