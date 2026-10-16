//! StylusArt: the PhotoCraft editor as an Android app (NativeActivity + eframe/wgpu).
//!
//! The same `PhotocraftApp` the desktop and web shells run, started from `android_main`, with the
//! touch interface (`photocraft_ui_egui::touch_ui`). Pen pressure arrives through winit's
//! `WindowEvent::Touch` force, which `photocraft_ui_egui::stylus` reads.
//!
//! What this shell adds for Android:
//! - Gestures: the touch observer feeds every touch to `touch_ui`'s gestures (two-finger
//!   pan/zoom/rotate, two- and three-finger taps for undo and redo).
//! - Palm rejection: the patched winit (`vendor/winit`) hands every pointer's tool type to a touch
//!   filter installed here. Pens always pass; a finger (or a palm, which Android reports as one)
//!   is dropped when it lands on the image, as `touch_ui::finger_allowed` decides.
//! - The on-screen keyboard, shown while text is being typed (the Type tool, a text field).
//! - The fonts bundled in the APK (`assets/fonts`, see `fetch-fonts.sh`) besides the system's.
//! - Export to Pictures/StylusArt and a gallery of Pictures, DCIM/Camera and Download to open
//!   photos from (after the photos permission is granted).

#![cfg(target_os = "android")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

use photocraft_engine::Session;
use photocraft_ui_egui::theme::ThemeKind;
use photocraft_ui_egui::touch_ui::TouchUi;
use photocraft_ui_egui::{PhotocraftApp, Services};
use winit::platform::android::activity::AndroidApp;
use winit::platform::android::{TouchDown, TouchSample, TouchTool};

/// Shared storage, as apps see it.
const STORAGE: &str = "/storage/emulated/0";

/// Entry point NativeActivity calls on its own thread.
#[allow(unsafe_code)] // `no_mangle` is required for the activity to find this symbol.
#[unsafe(no_mangle)]
fn android_main(android: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_max_level(log::LevelFilter::Info).with_tag("StylusArt"));
    winit::platform::android::set_touch_filter(keep_touch);
    winit::platform::android::set_touch_observer(observe_touch);
    register_bundled_fonts(&android);
    let options = eframe::NativeOptions { android_app: Some(android.clone()), renderer: eframe::Renderer::Wgpu, ..Default::default() };
    let result = eframe::run_native(
        "StylusArt",
        options,
        Box::new(move |cc| {
            PhotocraftApp::setup_context(&cc.egui_ctx, ThemeKind::Pro);
            let _ = EGUI_CTX.set(cc.egui_ctx.clone());
            let mut app = PhotocraftApp::new(Session::new(), services());
            // The tablet interface (large targets, tool rail, tool options, palm rejection) in
            // place of the desktop menus and panels. The app has no desktop mode.
            let mut touch = TouchUi::new();
            touch.export_dir = Some(PathBuf::from(STORAGE).join("Pictures/StylusArt"));
            touch.gallery_dirs = ["Pictures/StylusArt", "Pictures", "DCIM/Camera", "Pictures/Screenshots", "DCIM/Screenshots", "Download"]
                .iter()
                .map(|d| PathBuf::from(STORAGE).join(d))
                .collect();
            let asker = android.clone();
            touch.request_media = Some(Arc::new(move || request_media_permission(&asker)));
            app.touch = Some(touch);
            app.set_theme(&cc.egui_ctx, ThemeKind::Pro);
            if let Some(rs) = cc.wgpu_render_state.clone() {
                log::info!("wgpu backend {:?} on {}", rs.adapter.get_info().backend, rs.adapter.get_info().name);
                app.set_wgpu(rs);
            }
            Ok(Box::new(Shell { app, android, keyboard: false }))
        }),
    );
    if let Err(e) = result {
        log::error!("StylusArt failed to start: {e}");
    }
}

/// The app plus what only the Android shell does each frame (the soft keyboard).
struct Shell {
    app: PhotocraftApp,
    android: AndroidApp,
    /// The soft keyboard is shown.
    keyboard: bool,
}

impl eframe::App for Shell {
    fn on_exit(&mut self) {
        self.app.on_exit();
    }

    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.logic(ctx, frame);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        self.app.raw_input_hook(ctx, raw_input);
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.ui(ui, frame);
        // Show the keyboard while text is typed: an inline type edit or a focused text field.
        let wants = self.app.ui.text_edit.is_some() || ui.ctx().egui_wants_keyboard_input();
        if wants != self.keyboard {
            self.keyboard = wants;
            if wants {
                self.android.show_soft_input(true);
            } else {
                self.android.hide_soft_input(false);
            }
        }
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

/// Every touch (also the fingers the filter keeps off the image) goes to the touch interface's
/// gestures: two-finger pan/zoom/rotate, two- and three-finger taps for undo and redo.
fn observe_touch(s: &TouchSample) {
    use photocraft_ui_egui::touch_ui::{TouchEvent, TouchPhase, feed_touch};
    let Some(ctx) = EGUI_CTX.get() else { return };
    let ppp = ctx.pixels_per_point().max(0.1);
    let phase = match s.phase {
        winit::event::TouchPhase::Started => TouchPhase::Start,
        winit::event::TouchPhase::Moved => TouchPhase::Move,
        winit::event::TouchPhase::Ended => TouchPhase::End,
        winit::event::TouchPhase::Cancelled => TouchPhase::Cancel,
    };
    feed_touch(TouchEvent {
        id: s.pointer_id,
        pen: matches!(s.tool, TouchTool::Stylus | TouchTool::Eraser),
        phase,
        pos: egui::pos2(s.x / ppp, s.y / ppp),
        size: s.touch_major / ppp,
        dropped: s.dropped,
    });
    if s.dropped {
        ctx.request_repaint();
    }
}

/// Register the fonts in the APK's `assets/fonts` with the type engine.
fn register_bundled_fonts(android: &AndroidApp) {
    let assets = android.asset_manager();
    let Some(dir) = assets.open_dir(c"fonts") else {
        log::info!("no bundled fonts");
        return;
    };
    let mut count = 0;
    let mut engine = photocraft_text::shared().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    for name in dir {
        let file = name.to_string_lossy().to_string();
        let lower = file.to_lowercase();
        if !(lower.ends_with(".ttf") || lower.ends_with(".otf")) {
            continue;
        }
        let Ok(path) = std::ffi::CString::new(format!("fonts/{file}")) else { continue };
        let Some(mut asset) = assets.open(&path) else { continue };
        match asset.buffer() {
            Ok(bytes) => {
                let families = engine.fonts.register_font_data(bytes.to_vec());
                count += families.len();
            }
            Err(e) => log::warn!("font {file}: {e}"),
        }
    }
    log::info!("registered {count} bundled font families");
}

/// Ask for permission to read the user's photos (READ_MEDIA_IMAGES on Android 13+, storage before
/// that). Android shows its dialog once; when already granted this does nothing visible.
fn request_media_permission(android: &AndroidApp) {
    if let Err(e) = request_permissions(android, &["android.permission.READ_MEDIA_IMAGES", "android.permission.READ_EXTERNAL_STORAGE"]) {
        log::warn!("photos permission request failed: {e}");
    }
}

/// `Activity.requestPermissions(names, 1)` through JNI.
#[allow(unsafe_code)] // JNI handles from the activity; see the SAFETY notes.
fn request_permissions(android: &AndroidApp, names: &[&str]) -> Result<(), jni::errors::Error> {
    // SAFETY: `vm_as_ptr` is the process' JavaVM, valid for the app's lifetime.
    let vm = unsafe { jni::JavaVM::from_raw(android.vm_as_ptr().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    // SAFETY: `activity_as_ptr` is a global reference to the running Activity, valid while the
    // app runs; it is only borrowed here (never deleted).
    let activity = unsafe { jni::objects::JObject::from_raw(android.activity_as_ptr().cast()) };
    let first = env.new_string(names.first().copied().unwrap_or_default())?;
    let array = env.new_object_array(names.len() as i32, "java/lang/String", &first)?;
    for (i, name) in names.iter().enumerate().skip(1) {
        let s = env.new_string(name)?;
        env.set_object_array_element(&array, i as i32, &s)?;
    }
    env.call_method(&activity, "requestPermissions", "([Ljava/lang/String;I)V", &[(&array).into(), 1i32.into()])?;
    Ok(())
}

/// Decoding and encoding work in memory as on the web; opened files arrive through the inbox
/// (the gallery reads them).
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
        inbox: Some(Arc::default()),
        ..Default::default()
    }
}
