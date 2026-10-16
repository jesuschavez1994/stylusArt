//! StylusArt: the PhotoCraft editor as an Android app (NativeActivity + eframe/wgpu).
//!
//! The same `PhotocraftApp` the desktop and web shells run, started from `android_main`. Pen
//! pressure already arrives: winit's Android backend reports `MotionEvent` pressure as
//! `WindowEvent::Touch` force, which `photocraft_ui_egui::stylus` reads (the Windows path).
//! Tool type, hover, tilt, buttons and historical samples are not forwarded by winit yet; the
//! pen phase reads them from the `MotionEvent`s and writes them into the app's `StylusFeed`.

#![cfg(target_os = "android")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use winit::platform::android::activity::AndroidApp;
use photocraft_engine::Session;
use photocraft_ui_egui::theme::ThemeKind;
use photocraft_ui_egui::{PhotocraftApp, Services};

/// Entry point NativeActivity calls on its own thread.
#[allow(unsafe_code)] // `no_mangle` is required for the activity to find this symbol.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("StylusArt"));
    let options = eframe::NativeOptions { android_app: Some(app), renderer: eframe::Renderer::Wgpu, ..Default::default() };
    let result = eframe::run_native(
        "StylusArt",
        options,
        Box::new(|cc| {
            PhotocraftApp::setup_context(&cc.egui_ctx, ThemeKind::Pro);
            let mut app = PhotocraftApp::new(Session::new(), services());
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
