//! StylusArt: the PhotoCraft editor as an Android app (NativeActivity + eframe/wgpu).
//!
//! The same `PhotocraftApp` the desktop and web shells run, started from `android_main`. Pen
//! pressure already arrives: winit's Android backend reports `MotionEvent` pressure as
//! `WindowEvent::Touch` force, which `photocraft_ui_egui::stylus` reads (the Windows path).
//!
//! Palm rejection: the patched winit (`vendor/winit`) hands every pointer's tool type to a touch
//! filter installed here. Pens always pass; a finger (or a palm, which Android reports as one)
//! is dropped when it lands on the image, as `touch_ui::finger_allowed` decides.
//! Hover, tilt, pen buttons and historical samples are still not forwarded by winit.

#![cfg(target_os = "android")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::sync::OnceLock;

use winit::platform::android::activity::AndroidApp;
use winit::platform::android::{TouchDown, TouchTool};
use photocraft_engine::Session;
use photocraft_ui_egui::theme::ThemeKind;
use photocraft_ui_egui::{PhotocraftApp, Services};

/// Entry point NativeActivity calls on its own thread.
#[allow(unsafe_code)] // `no_mangle` is required for the activity to find this symbol.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("StylusArt"));
    winit::platform::android::set_touch_filter(keep_touch);
    let options = eframe::NativeOptions { android_app: Some(app), renderer: eframe::Renderer::Wgpu, ..Default::default() };
    let result = eframe::run_native(
        "StylusArt",
        options,
        Box::new(|cc| {
            PhotocraftApp::setup_context(&cc.egui_ctx, ThemeKind::Pro);
            let _ = EGUI_CTX.set(cc.egui_ctx.clone());
            let mut app = PhotocraftApp::new(Session::new(), services());
            // The tablet interface (large targets, tool rail, brush popover, palm rejection) in
            // place of the desktop menus and panels; its menu can switch back.
            app.touch = Some(photocraft_ui_egui::touch_ui::TouchUi::new());
            app.set_theme(&cc.egui_ctx, ThemeKind::Pro);
            if let Some(rs) = cc.wgpu_render_state.clone() {
                log::info!("wgpu backend {:?} on {}", rs.adapter.get_info().backend, rs.adapter.get_info().name);
                app.set_wgpu(rs);
            }
            Ok(Box::new(app))
        }),
    );
    if let Err(e) = result {
        log::error!("StylusArt failed to start: {e}");
    }
}

/// The UI context, for the touch filter (set once the app is created).
static EGUI_CTX: OnceLock<egui::Context> = OnceLock::new();

/// The touch filter (palm rejection): pens, erasers and mice always reach the app; a finger is
/// kept unless it lands on the image (the touch interface's rule).
fn keep_touch(down: &TouchDown) -> bool {
    match down.tool {
        TouchTool::Stylus | TouchTool::Eraser | TouchTool::Mouse => true,
        TouchTool::Finger | TouchTool::Unknown => {
            let Some(ctx) = EGUI_CTX.get() else { return true };
            let ppp = ctx.pixels_per_point().max(0.1);
            photocraft_ui_egui::touch_ui::finger_allowed(ctx, egui::pos2(down.x / ppp, down.y / ppp))
        }
    }
}

/// Decoding and encoding work in memory as on the web; file dialogs (Storage Access Framework)
/// and preferences storage come with the files phase.
fn services() -> Services {
    Services {
        import: Some(Box::new(|name: &str, bytes: &[u8], max_svg_group_depth: usize| {
            photocraft_io::import_with_svg_group_depth(name, bytes, max_svg_group_depth).map(|r| (r.document, r.warnings)).map_err(|e| e.to_string())
        })),
        export: Some(Box::new(|doc: &photocraft_doc::Document, path: &str, settings: &photocraft_ui_egui::ExportSettings| {
            photocraft_io::export(doc, path, &settings.io_options()).map(|r| (r.bytes, r.warnings)).map_err(|e| e.to_string())
        })),
        encode_png: Some(Box::new(|w, h, rgba| {
            let img = photocraft_codecs::Image::from_u8(w, h, photocraft_codecs::ChannelLayout::Rgba, rgba.to_vec()).map_err(|e| e.to_string())?;
            photocraft_codecs::encode(&img, photocraft_codecs::Format::Png, &photocraft_codecs::EncodeOptions::default()).map_err(|e| e.to_string())
        })),
        ..Default::default()
    }
}
