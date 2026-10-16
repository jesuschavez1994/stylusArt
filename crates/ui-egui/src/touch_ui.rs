//! Touch interface for tablets (StylusArt on Android), in place of the desktop menu bar, options
//! bar, toolbar and dock: large targets; a tool rail whose active tool opens its options (brush
//! width/opacity/hardness with a stroke preview, type font/size/alignment, Patch, Crop and
//! Artboard choices); an Apply/Cancel bar while a crop, transform or text edit is pending; a
//! colour well; a Layers sheet with Remove Background, layer masks and clipping masks; image
//! export (PNG with or without the background, JPG) and an image gallery to open photos; palm
//! rejection. The canvas, dialogs and every action stay the same: each
//! control here runs the engine command the desktop UI runs (`tools.setBrush`, `history.undo`,
//! `layer.*`, `tools.setColors`).
//!
//! On when [`PhotocraftApp::touch`] is `Some`; the "…" menu switches back to the desktop
//! interface.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, PoisonError};

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use photocraft_doc::{Layer, LayerContent};
use serde_json::json;

use crate::PhotocraftApp;
use crate::icons;
use crate::state::Tool;
use crate::theme::Tokens;

const BAR_H: f32 = 60.0;
const RAIL_W: f32 = 84.0;
const BTN: f32 = 58.0;
const ICON: f32 = 28.0;
const BRUSH_POP_W: f32 = 400.0;
const LAYERS_W: f32 = 340.0;
const ROW_H: f32 = 60.0;
/// Largest size the size slider reaches (the engine allows up to `MAX_BRUSH_SIZE`).
const SLIDER_MAX_SIZE: f32 = 1000.0;

/// The tools on the rail, in order: painting first, then selection and the rest.
const TOOLS: &[(Tool, &str)] = &[
    (Tool::Brush, "Pincel"),
    (Tool::Pencil, "Lápiz"),
    (Tool::Eraser, "Borrador"),
    (Tool::Smudge, "Difuminar"),
    (Tool::PaintBucket, "Bote de pintura"),
    (Tool::Gradient, "Degradado"),
    (Tool::Eyedropper, "Cuentagotas"),
    (Tool::Type, "Texto"),
    (Tool::Lasso, "Lazo"),
    (Tool::RectMarquee, "Marco rectangular"),
    (Tool::Move, "Mover"),
    (Tool::Crop, "Recortar"),
    (Tool::Patch, "Parche"),
    (Tool::Artboard, "Mesa de trabajo"),
];

/// Quick colours under the colour wheel.
const SWATCHES: &[[u8; 3]] = &[
    [0, 0, 0],
    [255, 255, 255],
    [128, 128, 128],
    [230, 57, 70],
    [244, 162, 97],
    [233, 196, 106],
    [42, 157, 143],
    [69, 123, 157],
    [29, 53, 87],
    [131, 56, 236],
    [255, 0, 110],
    [111, 78, 55],
];

/// Touch interface state (which sheets are open) and what the platform shell provides.
#[derive(Default)]
pub struct TouchUi {
    pub layers_open: bool,
    pub menu_open: bool,
    pub color_open: bool,
    /// The brush settings popover (width, opacity, hardness, pressure) is open.
    pub brush_open: bool,
    /// A blank canvas was created at start, so the app opens ready to draw.
    pub started: bool,
    /// The active layer's opacity while its slider is dragged: committed as one step on release.
    opacity_drag: Option<(u64, f32)>,
    /// Where the colour popover opens (beside the colour well), in points.
    color_anchor: Option<Pos2>,
    /// Where the brush popover opens (beside the tapped tool), in points.
    brush_anchor: Option<Pos2>,
    /// Folder exports are written to (Android: Pictures/StylusArt). `None`: Export As dialog.
    pub export_dir: Option<PathBuf>,
    /// Folders the image gallery lists (Android: Pictures, DCIM/Camera, Download). Empty: the
    /// Open dialog instead.
    pub gallery_dirs: Vec<PathBuf>,
    /// Asks the platform for permission to read the user's photos (Android runtime permission).
    pub request_media: Option<Arc<dyn Fn() + Send + Sync>>,
    gallery: Option<Gallery>,
    /// The font list's search text.
    font_query: String,
    /// A short message at the bottom of the screen, until the time (seconds) it disappears.
    notice: Option<(String, f64)>,
    /// The colour picker's hue, saturation and value (kept so greys keep their hue).
    hsv: Hsv,
    /// The colour being dragged in the picker, shown at once but set on release.
    picking: Option<Hsv>,
    /// Colours set recently, newest first.
    recent: Vec<Color32>,
    /// The brush library sheet is open, and its search text.
    pub library_open: bool,
    /// The Layers sheet's blend mode list is open.
    blend_open: bool,
    /// The Transform popover is open, and where (beside its rail button).
    pub transform_open: bool,
    transform_anchor: Option<Pos2>,
    brush_query: String,
}

impl TouchUi {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Draw the touch chrome (top bar, tool rail, Layers sheet, popovers). Call before the canvas'
/// central panel, in place of the desktop chrome.
pub fn chrome(app: &mut PhotocraftApp, ui: &mut egui::Ui) {
    let Some(mut st) = app.touch.take() else { return };
    let t = Tokens::get(ui.ctx());
    if !st.started {
        st.started = true;
        // A finger moves a little and rests longer than a mouse click: a tap still counts as one
        // within 18 points and 1.2 s (egui's defaults, 6 points and 0.8 s, drop many real taps).
        ui.ctx().options_mut(|o| {
            o.input_options.max_click_dist = 18.0;
            o.input_options.max_click_duration = 1.2;
        });
        if app.session.active().is_none() {
            new_canvas(app, ui.ctx());
        }
    }
    dismiss_on_outside_tap(&mut st, ui.ctx());
    run_gestures(app, ui.ctx());
    top_bar(app, &mut st, ui, &t);
    tool_rail(app, &mut st, ui, &t);
    if st.layers_open {
        layers_sheet(app, &mut st, ui, &t);
    }
    if st.menu_open {
        menu_popover(app, &mut st, ui.ctx(), &t);
    }
    if st.color_open {
        color_popover(app, &mut st, ui.ctx(), &t);
    }
    if st.brush_open {
        tool_options(app, &mut st, ui.ctx(), &t);
    }
    action_bar(app, ui.ctx(), &t);
    if st.transform_open {
        transform_options(app, &mut st, ui.ctx(), &t);
    }
    if st.library_open {
        brush_library(app, &mut st, ui.ctx(), &t);
    }
    if st.gallery.is_some() {
        gallery_sheet(app, &mut st, ui.ctx(), &t);
    }
    show_notice(&mut st, ui.ctx(), &t);
    // One more frame after any input. A tap's effect (the selected layer, a colour, a tool) is
    // applied while this frame draws, after the widgets that show it; with a mouse the next
    // pointer move redraws at once, but a touch screen sends nothing more, so without this the
    // change would show only on the next touch and the app would seem stuck.
    if ui.ctx().input(|i| !i.events.is_empty()) {
        ui.ctx().request_repaint();
    }
    // "Interfaz de escritorio" in the menu clears the state; otherwise keep it.
    if app.touch_off_requested {
        app.touch_off_requested = false;
    } else {
        app.touch = Some(st);
    }
}

/// A tap on the image outside the open popovers closes them (as on iPad and Android). The tap
/// still does what it does there.
fn dismiss_on_outside_tap(st: &mut TouchUi, ctx: &egui::Context) {
    if !(st.menu_open || st.color_open || st.brush_open || st.transform_open) {
        return;
    }
    let Some(pos) = ctx.input(|i| if i.pointer.any_pressed() { i.pointer.interact_pos() } else { None }) else { return };
    // Only on the image: the bar and rail buttons toggle their own popovers.
    let on_canvas = CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner).is_some_and(|r| r.contains(pos));
    if on_canvas && ctx.layer_id_at(pos).is_none_or(|l| l.order == egui::Order::Background) {
        st.menu_open = false;
        st.color_open = false;
        st.brush_open = false;
        st.transform_open = false;
        st.picking = None;
    }
}

/// A new white canvas the size of the screen, in device pixels.
fn new_canvas(app: &mut PhotocraftApp, ctx: &egui::Context) {
    let px = ctx.content_rect().size() * ctx.pixels_per_point();
    let (w, h) = (px.x.round().clamp(512.0, 8192.0), px.y.round().clamp(512.0, 8192.0));
    run(app, "file.new", json!({ "width": w, "height": h, "background": "white", "name": "Lienzo" }));
}

fn run(app: &mut PhotocraftApp, id: &str, params: serde_json::Value) {
    if let Err(e) = app.run(id, params) {
        log::warn!("{id}: {e}");
        app.ui.status = e;
    }
}

/// A menu item: the same path the desktop menus take (UI and engine commands, dialogs).
fn invoke(app: &mut PhotocraftApp, ctx: &egui::Context, id: &str) {
    if let Err(e) = crate::menus::invoke(app, ctx, id, json!({})) {
        log::warn!("{id}: {e}");
        app.ui.status = e;
    }
}

fn icon_button(ui: &mut egui::Ui, name: &str, selected: bool, tip: &str) -> egui::Response {
    icons::button_with_icon_size(ui, name, BTN, ICON, selected, tip)
}

fn top_bar(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::top("touch-top")
        .resizable(false)
        .exact_size(BAR_H)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(12, 1)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if icon_button(ui, "ellipsis", st.menu_open, "Menú").clicked() {
                    st.menu_open = !st.menu_open;
                    st.color_open = false;
                }
                let title = app.session.active().map(|d| d.doc.name.clone()).unwrap_or_else(|| "StylusArt".into());
                ui.add_space(8.0);
                ui.label(egui::RichText::new(title).size(20.0).color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, "layers", st.layers_open, "Capas").clicked() {
                        st.layers_open = !st.layers_open;
                    }
                    ui.add_space(6.0);
                    color_well(app, st, ui, t);
                    ui.add_space(12.0);
                    let redo = icon_button(ui, "redo-2", false, "Rehacer");
                    let undo = icon_button(ui, "undo-2", false, "Deshacer");
                    if undo.clicked() {
                        run(app, "history.undo", json!({}));
                    }
                    if redo.clicked() {
                        run(app, "history.redo", json!({}));
                    }
                });
            });
        });
}

fn tool_rail(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::left("touch-tools")
        .resizable(false)
        .exact_size(RAIL_W)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(13, 10)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for &(tool, tip) in TOOLS {
                    let selected = app.ui.tool == tool;
                    let resp = icon_button(ui, icons::tool_icon(tool), selected, tip);
                    if resp.clicked() {
                        if selected {
                            // A second tap on the active tool opens its brush settings (as in
                            // Samsung Notes and Procreate).
                            toggle_brush(st, resp.rect);
                        } else {
                            crate::tool_memory::select(app, tool);
                            crate::paint_mouse::sync_tool_brush(app);
                            st.brush_open = false;
                        }
                    }
                }
                ui.add_space(10.0);
                let resp = icon_button(ui, "scaling", st.transform_open, "Transformar");
                if resp.clicked() {
                    st.transform_open = !st.transform_open;
                    st.transform_anchor = Some(pos2(resp.rect.right() + 22.0, resp.rect.top()));
                    st.brush_open = false;
                    st.color_open = false;
                    st.menu_open = false;
                }
                let resp = icon_button(ui, "sliders-horizontal", st.brush_open, "Ajustes del pincel");
                if resp.clicked() {
                    toggle_brush(st, resp.rect);
                }
            });
        });
}

fn toggle_brush(st: &mut TouchUi, button: Rect) {
    st.brush_open = !st.brush_open;
    st.brush_anchor = Some(pos2(button.right() + 22.0, button.top()));
    st.color_open = false;
    st.menu_open = false;
}

fn fmt_size(s: f32) -> String {
    if s < 10.0 { format!("{s:.1}") } else { format!("{}", s.round()) }
}

fn tool_name(tool: Tool) -> &'static str {
    TOOLS.iter().find(|(t, _)| *t == tool).map_or("Pincel", |(_, n)| n)
}

//// The active tool's options beside the tool rail (a second tap on the tool, or the settings
/// button): brush settings for the painting tools, type settings, Patch, Crop and Artboard
/// choices, and a short how-to for the rest.
fn tool_options(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let tool = app.ui.tool;
    let height = match tool {
        Tool::Type => 700.0,
        Tool::Brush | Tool::Pencil | Tool::Eraser | Tool::Smudge => 720.0,
        _ => 380.0,
    };
    let anchor = st.brush_anchor.unwrap_or(pos2(RAIL_W + 10.0, BAR_H + 10.0));
    let lo = screen.top() + BAR_H + 6.0;
    let top = anchor.y.clamp(lo, (screen.bottom() - height - 8.0).max(lo));
    let mut close = false;
    egui::Area::new(egui::Id::new("touch-brush")).order(egui::Order::Foreground).fixed_pos(pos2(anchor.x, top)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            let w = BRUSH_POP_W - 32.0;
            ui.set_width(w);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tool_name(tool)).size(20.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button_with_icon_size(ui, "x", 40.0, 20.0, false, "Cerrar").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(6.0);
            match tool {
                Tool::Brush | Tool::Pencil | Tool::Eraser | Tool::Smudge => brush_options(app, st, ui, w, t),
                Tool::Type => text_options(app, st, ui, t),
                Tool::Patch => patch_options(app, ui, t),
                Tool::Crop => crop_options(app, ui, t),
                Tool::Artboard => artboard_options(app, st, ui, t),
                other => hint(ui, tool_hint(other), t),
            }
        });
    });
    if close {
        st.brush_open = false;
    }
}

fn hint(ui: &mut egui::Ui, text: &str, t: &Tokens) {
    ui.label(egui::RichText::new(text).size(16.0).color(t.text_dim));
    ui.add_space(8.0);
}

fn tool_hint(tool: Tool) -> &'static str {
    match tool {
        Tool::PaintBucket => "Toca una zona para rellenarla con el color actual.",
        Tool::Gradient => "Arrastra sobre el lienzo para crear un degradado del color actual al de fondo.",
        Tool::Eyedropper => "Toca el lienzo para tomar ese color.",
        Tool::Lasso => "Dibuja alrededor de una zona para seleccionarla. Después puedes pintar solo dentro, moverla o usar el parche.",
        Tool::RectMarquee => "Arrastra para seleccionar un rectángulo.",
        Tool::Move => "Arrastra para mover la capa activa (o la selección).",
        _ => "",
    }
}

/// Brush settings: a live preview of the stroke at its real width, then width, opacity and
/// hardness sliders and the pressure switch (Samsung Notes' pen popover).
fn brush_options(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, w: f32, t: &Tokens) {
    stroke_preview(app, ui, w, t);
    ui.add_space(8.0);
    if big_button(ui, "Cambiar pincel…", t).clicked() {
        st.library_open = true;
        st.brush_open = false;
    }
    ui.add_space(12.0);
    let b = app.session.tools.brush.clone();
    // Width: logarithmic, so small sizes get most of the travel.
    let u = (b.size.max(1.0).ln() / SLIDER_MAX_SIZE.ln()).clamp(0.0, 1.0);
    caption(ui, "Ancho del trazo", &format!("{} px", fmt_size(b.size)), t);
    if let Some(u) = h_slider(ui, u, SliderFill::Accent, t) {
        let s = SLIDER_MAX_SIZE.powf(u).clamp(1.0, SLIDER_MAX_SIZE);
        let s = if s < 10.0 { (s * 10.0).round() / 10.0 } else { s.round() };
        if s != b.size {
            run(app, "tools.setBrush", json!({ "brush": { "size": s }, "coalesce": "touch-size" }));
        }
    }
    ui.add_space(10.0);
    caption(ui, "Opacidad", &format!("{} %", (b.opacity * 100.0).round()), t);
    if let Some(u) = h_slider(ui, b.opacity, SliderFill::Opacity(fg_color(app)), t) {
        let o = (u * 100.0).round().clamp(1.0, 100.0) / 100.0;
        if o != b.opacity {
            run(app, "tools.setBrush", json!({ "brush": { "opacity": o }, "coalesce": "touch-opacity" }));
        }
    }
    ui.add_space(10.0);
    caption(ui, "Dureza", &format!("{} %", (b.hardness * 100.0).round()), t);
    if let Some(u) = h_slider(ui, b.hardness, SliderFill::Accent, t) {
        let h = (u * 100.0).round() / 100.0;
        if h != b.hardness {
            run(app, "tools.setBrush", json!({ "brush": { "hardness": h }, "coalesce": "touch-hardness" }));
        }
    }
    ui.add_space(12.0);
    let mut pressure = b.pressure_size;
    if switch_row(ui, "La presión controla el ancho", &mut pressure, t) {
        run(app, "tools.setBrush", json!({ "brush": { "pressureSize": pressure } }));
    }
    ui.add_space(12.0);
    symmetry_options(app, st, ui, t);
}

const TYPE_MIN_PT: f32 = 6.0;
const TYPE_MAX_PT: f32 = 400.0;

/// Type settings: size, alignment, colour and the font list (bundled and system fonts, with a
/// search field). They apply to the text being edited and to new text.
fn text_options(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let ctx = ui.ctx().clone();
    hint(ui, "Toca el lienzo donde quieras escribir; el teclado aparece solo. Toca Aplicar para terminar.", t);
    let size = app.ui.tool_options.type_size.clamp(TYPE_MIN_PT, TYPE_MAX_PT);
    caption(ui, "Tamaño", &format!("{} pt", size.round()), t);
    let u = ((size / TYPE_MIN_PT).ln() / (TYPE_MAX_PT / TYPE_MIN_PT).ln()).clamp(0.0, 1.0);
    if let Some(u) = h_slider(ui, u, SliderFill::Accent, t) {
        let s = (TYPE_MIN_PT * (TYPE_MAX_PT / TYPE_MIN_PT).powf(u)).round();
        if s != app.ui.tool_options.type_size {
            app.ui.tool_options.type_size = s;
            crate::type_tool::apply(app, &ctx, json!({ "size": s, "metricsAsShown": true }));
        }
    }
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (align, icon, tip) in [("left", "align-left", "Izquierda"), ("center", "align-center", "Centro"), ("right", "align-right", "Derecha")] {
            let on = app.ui.tool_options.type_align == align;
            if icons::button_with_icon_size(ui, icon, 52.0, 24.0, on, tip).clicked() {
                app.ui.tool_options.type_align = align.into();
                crate::type_tool::apply(app, &ctx, json!({ "align": align }));
            }
        }
        ui.add_space(8.0);
        let c = fg_color(app);
        if chip(ui, "Usar el color actual", false, t).clicked() {
            let rgb = [f32::from(c.r()) / 255.0, f32::from(c.g()) / 255.0, f32::from(c.b()) / 255.0];
            crate::type_tool::apply(app, &ctx, json!({ "color": rgb }));
        }
    });
    ui.add_space(10.0);
    ui.label(egui::RichText::new("Tipografía").size(17.0).color(t.text));
    ui.add_space(4.0);
    ui.add(
        egui::TextEdit::singleline(&mut st.font_query)
            .hint_text("Buscar fuente…")
            .desired_width(f32::INFINITY)
            .font(egui::FontId::proportional(18.0))
            .margin(egui::Margin::symmetric(10, 8)),
    );
    ui.add_space(6.0);
    crate::type_tool::warm_fonts();
    let families = crate::type_tool::families();
    let query = st.font_query.to_lowercase();
    let current = app.ui.tool_options.type_font.clone();
    egui::ScrollArea::vertical().min_scrolled_height(320.0).max_height(320.0).auto_shrink([false, false]).show(ui, |ui| {
        let mut shown = 0;
        for fam in families.iter().filter(|f| !f.starts_with('.') && (query.is_empty() || f.to_lowercase().contains(&query))) {
            shown += 1;
            let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 46.0), Sense::click());
            let sel = *fam == current;
            ui.painter().rect_filled(r, CornerRadius::same(10), if sel { t.row_selected } else { Color32::TRANSPARENT });
            ui.painter().text(pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, fam, egui::FontId::proportional(18.0), t.text);
            if resp.clicked() {
                app.ui.tool_options.type_font = fam.clone();
                let styles = crate::type_tool::styles(fam);
                let style = if styles.iter().any(|s| s == "Regular") { "Regular".to_string() } else { styles.first().cloned().unwrap_or_else(|| "Regular".into()) };
                app.ui.tool_options.type_style = style;
                crate::type_tool::apply(app, &ctx, json!({ "font": fam }));
            }
        }
        if shown == 0 {
            ui.label(egui::RichText::new(if families.is_empty() { "Cargando fuentes…" } else { "Ninguna fuente coincide." }).color(t.text_dim));
        }
    });
}

fn patch_options(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    hint(ui, "Rodea la zona a reparar (una mancha, un objeto) y arrástrala hacia una zona limpia: el parche copia la textura y la funde con los bordes.", t);
    ui.horizontal(|ui| {
        for (mode, label) in [("source", "Origen"), ("destination", "Destino")] {
            if chip(ui, label, app.ui.tool_options.patch_mode == mode, t).clicked() {
                app.ui.tool_options.patch_mode = mode.into();
            }
        }
    });
    ui.add_space(10.0);
    let mut aware = app.ui.tool_options.patch_content_aware;
    if switch_row(ui, "Según el contenido", &mut aware, t) {
        app.ui.tool_options.patch_content_aware = aware;
    }
}

fn crop_options(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    hint(ui, "Arrastra los bordes o las esquinas del marco y toca Aplicar. Arrastra dentro del marco para moverlo.", t);
    if big_button(ui, "Restablecer el marco", t).clicked() {
        crate::crop_ui::cancel(app);
        crate::crop_ui::ensure_frame(app);
    }
    ui.add_space(8.0);
    if big_button(ui, "Recortar los bordes transparentes", t).clicked() {
        run(app, "image.trim", json!({ "basedOn": "transparent" }));
    }
}

/// Artboard sizes offered in its options (width, height in pixels; 0: the canvas size).
const ARTBOARDS: &[(&str, f64, f64)] = &[
    ("Igual que el lienzo", 0.0, 0.0),
    ("Cuadrado 1080 × 1080", 1080.0, 1080.0),
    ("Historia 1080 × 1920", 1080.0, 1920.0),
    ("Horizontal 1920 × 1080", 1920.0, 1080.0),
    ("A4 vertical (300 ppp)", 2480.0, 3508.0),
];

fn artboard_options(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    hint(ui, "Arrastra en una zona vacía para dibujar una mesa de trabajo, o crea una de estos tamaños (se coloca a la derecha de la última):", t);
    for &(label, w, h) in ARTBOARDS {
        if big_button(ui, label, t).clicked() {
            let params = if w > 0.0 { json!({ "width": w, "height": h, "name": label }) } else { json!({}) };
            let ctx = ui.ctx().clone();
            match app.run("layer.new.artboard", params) {
                Ok(_) => {
                    invoke(app, &ctx, "view.fitOnScreen");
                    notify(st, &ctx, "Mesa de trabajo creada");
                }
                Err(e) => notify(st, &ctx, &e),
            }
        }
        ui.add_space(6.0);
    }
}

/// A rounded choice button; `on` highlights it.
fn chip(ui: &mut egui::Ui, text: &str, on: bool, t: &Tokens) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(text.to_string(), egui::FontId::proportional(17.0), t.text);
    let (r, resp) = ui.allocate_exact_size(vec2(galley.size().x + 32.0, 44.0), Sense::click());
    ui.painter().rect_filled(r, CornerRadius::same(22), if on { t.accent } else { t.field });
    ui.painter().galley(pos2(r.center().x - galley.size().x / 2.0, r.center().y - galley.size().y / 2.0), galley, t.text);
    resp
}

/// What the Apply / Cancel bar finishes.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pending {
    Text,
    Transform,
    Crop,
}

/// Apply / Cancel, floating under the top bar while a text edit, a transform or a crop is
/// pending (the desktop's Enter and Esc).
fn action_bar(app: &mut PhotocraftApp, ctx: &egui::Context, t: &Tokens) {
    let pending = if app.ui.text_edit.is_some() {
        Pending::Text
    } else if app.ui.transform.is_some() {
        Pending::Transform
    } else if app.ui.tool == Tool::Crop && app.ui.crop_rect.is_some() {
        Pending::Crop
    } else {
        return;
    };
    let screen = ctx.content_rect();
    let size = vec2(320.0, 64.0);
    let at = pos2(screen.center().x - size.x / 2.0, screen.top() + BAR_H + 12.0);
    let mut action = None;
    egui::Area::new(egui::Id::new("touch-apply")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        egui::Frame::NONE
            .fill(t.card)
            .stroke(Stroke::new(1.0, t.card_border))
            .corner_radius(CornerRadius::same(32))
            .inner_margin(egui::Margin::same(8))
            .shadow(egui::Shadow { offset: [0, 4], blur: 18, spread: 0, color: Color32::from_black_alpha(80) })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if pill_button(ui, "x", "Cancelar", t.field, t).clicked() {
                        action = Some(false);
                    }
                    if pill_button(ui, "check", "Aplicar", t.accent, t).clicked() {
                        action = Some(true);
                    }
                });
            });
    });
    match (pending, action) {
        (Pending::Text, Some(true)) => crate::type_tool::commit(app),
        (Pending::Text, Some(false)) => crate::type_tool::cancel(app),
        (Pending::Transform, Some(true)) => crate::transform_tool::commit(app),
        (Pending::Transform, Some(false)) => crate::transform_tool::cancel(app),
        (Pending::Crop, Some(true)) => crate::canvas::commit_crop(app),
        (Pending::Crop, Some(false)) => {
            crate::crop_ui::cancel(app);
            crate::crop_ui::ensure_frame(app);
        }
        (_, None) => {}
    }
}

fn pill_button(ui: &mut egui::Ui, icon: &str, text: &str, fill: Color32, t: &Tokens) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(vec2(148.0, 48.0), Sense::click());
    let fill = if resp.is_pointer_button_down_on() { t.pressed } else { fill };
    ui.painter().rect_filled(r, CornerRadius::same(24), fill);
    icons::paint(ui, Rect::from_center_size(pos2(r.left() + 30.0, r.center().y), Vec2::splat(24.0)), icon, 22.0, t.text);
    ui.painter().text(pos2(r.left() + 50.0, r.center().y), Align2::LEFT_CENTER, text, egui::FontId::proportional(18.0), t.text);
    resp
}

fn notify(st: &mut TouchUi, ctx: &egui::Context, text: &str) {
    let now = ctx.input(|i| i.time);
    st.notice = Some((text.to_string(), now + 3.5));
    ctx.request_repaint();
}

fn show_notice(st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let Some((text, until)) = st.notice.clone() else { return };
    let now = ctx.input(|i| i.time);
    if now > until {
        st.notice = None;
        return;
    }
    ctx.request_repaint_after(std::time::Duration::from_millis(250));
    let max_w = ctx.content_rect().width() * 0.6;
    egui::Area::new(egui::Id::new("touch-notice")).order(egui::Order::Tooltip).anchor(Align2::CENTER_BOTTOM, vec2(0.0, -28.0)).show(ctx, |ui| {
        egui::Frame::NONE
            .fill(t.card)
            .stroke(Stroke::new(1.0, t.card_border))
            .corner_radius(CornerRadius::same(14))
            .inner_margin(egui::Margin::symmetric(18, 12))
            .show(ui, |ui| {
                ui.set_max_width(max_w);
                ui.label(egui::RichText::new(text).size(17.0).color(t.text));
            });
    });
}

/// The brush's real stroke (rendered by the brush engine, as the Brushes panel does) in the
/// foreground colour, on the field colour.
fn stroke_preview(app: &PhotocraftApp, ui: &mut egui::Ui, w: f32, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 100.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(12), t.field);
    let erase = matches!(app.ui.tool, Tool::Eraser | Tool::BackgroundEraser | Tool::MagicEraser);
    let color = if erase { t.text_dim } else { fg_color(app) };
    let ppp = ui.ctx().pixels_per_point();
    let (pw, ph) = ((rect.width() * ppp) as u32, (rect.height() * ppp) as u32);
    let tex = crate::brush_preview::stroke_texture(ui.ctx(), "touch-brush-preview", &app.session.tools.brush, pw, ph, color);
    ui.painter().image(tex.id(), rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
}

/// Painting symmetry for the brush tools: off, or mirrored across the canvas centre.
const SYMMETRY: &[(&str, &str)] = &[("", "No"), ("vertical", "Vertical"), ("horizontal", "Horizontal"), ("dual", "Doble"), ("diagonal", "Diagonal")];

fn symmetry_options(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let current = app.session.active().and_then(|d| d.symmetry.as_ref()).and_then(|s| s.preset()).map(|p| p.mode.id().to_string()).unwrap_or_default();
    ui.label(egui::RichText::new("Simetría").size(17.0).color(t.text));
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for &(mode, label) in SYMMETRY {
            if chip(ui, label, current == mode, t).clicked() && current != mode {
                let r = if mode.is_empty() { app.run("paint.symmetryDisable", json!({})) } else { app.run("paint.setSymmetry", json!({ "mode": mode })) };
                if let Err(e) = r {
                    notify(st, ui.ctx(), &e);
                }
            }
        }
    });
}

/// The brush presets (built-in and imported) with their real strokes; a tap picks one.
fn brush_library(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let width = 560.0_f32.min(screen.width() - RAIL_W - 40.0);
    let rect = Rect::from_min_size(pos2(RAIL_W + 12.0, screen.top() + BAR_H + 10.0), vec2(width, screen.height() - BAR_H - 26.0));
    let mut close = false;
    let mut pick: Option<String> = None;
    let color = fg_color(app);
    egui::Area::new(egui::Id::new("touch-brush-library")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            ui.set_width(rect.width() - 32.0);
            ui.set_height(rect.height() - 32.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Pinceles").size(22.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button_with_icon_size(ui, "x", 44.0, 22.0, false, "Cerrar").clicked() {
                        close = true;
                    }
                });
            });
            ui.add(
                egui::TextEdit::singleline(&mut st.brush_query)
                    .hint_text("Buscar pincel…")
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::proportional(18.0))
                    .margin(egui::Margin::symmetric(10, 8)),
            );
            ui.add_space(8.0);
            let query = st.brush_query.to_lowercase();
            let presets: Vec<(usize, String, String)> = app
                .session
                .tools
                .presets
                .iter()
                .enumerate()
                .filter(|(_, p)| query.is_empty() || p.name.to_lowercase().contains(&query) || p.group.to_lowercase().contains(&query))
                .map(|(i, p)| (i, p.name.clone(), p.group.clone()))
                .collect();
            if presets.is_empty() {
                hint(ui, "No hay pinceles que coincidan.", t);
                return;
            }
            let row_h = 92.0;
            let ppp = ui.ctx().pixels_per_point();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, presets.len(), |ui, range| {
                for (i, name, group) in presets.get(range).unwrap_or_default() {
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h - 6.0), Sense::click());
                    ui.painter().rect_filled(r, CornerRadius::same(12), if resp.is_pointer_button_down_on() { t.pressed } else { t.field });
                    let stroke_r = Rect::from_min_max(pos2(r.left() + 10.0, r.top() + 6.0), pos2(r.right() - 10.0, r.bottom() - 26.0));
                    if let Some(p) = app.session.tools.presets.get(*i) {
                        let tex = crate::brush_preview::stroke_texture(ui.ctx(), &format!("touch-lib-{i}"), &p.brush, (stroke_r.width() * ppp) as u32, (stroke_r.height() * ppp) as u32, color);
                        ui.painter().image(tex.id(), stroke_r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    ui.painter().text(pos2(r.left() + 14.0, r.bottom() - 13.0), Align2::LEFT_CENTER, name, egui::FontId::proportional(16.0), t.text);
                    if !group.is_empty() {
                        ui.painter().text(pos2(r.right() - 14.0, r.bottom() - 13.0), Align2::RIGHT_CENTER, group, egui::FontId::proportional(13.0), t.text_dim);
                    }
                    if resp.clicked() {
                        pick = Some(name.clone());
                    }
                    ui.add_space(6.0);
                }
            });
        });
    });
    if let Some(name) = pick {
        match app.run("tools.setBrush", json!({ "preset": name })) {
            Ok(_) => {
                notify(st, ctx, &format!("Pincel: {name}"));
                close = true;
            }
            Err(e) => notify(st, ctx, &e),
        }
    }
    if close {
        st.library_open = false;
    }
}

fn caption(ui: &mut egui::Ui, label: &str, value: &str, t: &Tokens) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(17.0).color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(value).size(17.0).color(t.text));
        });
    });
    ui.add_space(4.0);
}

#[derive(Clone, Copy)]
enum SliderFill {
    Accent,
    /// Checkerboard fading into the colour, for opacity.
    Opacity(Color32),
}

/// A wide pill slider (0..1) with a round white thumb; touching anywhere on it sets the value.
/// Returns the value under the finger or pen while touched.
fn h_slider(ui: &mut egui::Ui, value: f32, fill: SliderFill, t: &Tokens) -> Option<f32> {
    const H: f32 = 44.0;
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, H), Sense::click_and_drag());
    let p = ui.painter_at(rect);
    let r = CornerRadius::same((H / 2.0) as u8);
    let v = value.clamp(0.0, 1.0);
    let x = rect.left() + H / 2.0 + (rect.width() - H) * v;
    match fill {
        SliderFill::Accent => {
            p.rect_filled(rect, r, t.field_border);
            p.rect_filled(Rect::from_min_max(rect.min, pos2(x + H / 2.0, rect.bottom())), r, t.accent);
        }
        SliderFill::Opacity(c) => {
            let (light, dark) = (Color32::from_gray(96), Color32::from_gray(64));
            p.rect_filled(rect, r, dark);
            // Checkerboard inside the straight part of the pill.
            let inner = rect.shrink2(vec2(H / 2.0, 0.0));
            let cell = H / 4.0;
            let (cols, rows) = ((inner.width() / cell).ceil() as usize, 4usize);
            for row in 0..rows {
                for col in 0..cols {
                    if (row + col) % 2 == 0 {
                        let min = pos2(inner.left() + col as f32 * cell, inner.top() + row as f32 * cell);
                        let cr = Rect::from_min_max(min, pos2((min.x + cell).min(inner.right()), (min.y + cell).min(inner.bottom())));
                        p.rect_filled(cr, 0.0, light);
                    }
                }
            }
            p.circle_filled(pos2(rect.right() - H / 2.0, rect.center().y), H / 2.0, c);
            let mut mesh = egui::Mesh::default();
            let c0 = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 0);
            mesh.colored_vertex(inner.left_top(), c0);
            mesh.colored_vertex(inner.right_top(), c);
            mesh.colored_vertex(inner.right_bottom(), c);
            mesh.colored_vertex(inner.left_bottom(), c0);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            p.add(mesh);
        }
    }
    p.circle_filled(pos2(x, rect.center().y), H / 2.0 - 5.0, Color32::WHITE);
    p.circle_filled(pos2(x, rect.center().y), 4.0, Color32::from_gray(60));
    let active = resp.is_pointer_button_down_on() || resp.dragged();
    let pos = resp.interact_pointer_pos().filter(|_| active)?;
    Some(((pos.x - rect.left() - H / 2.0) / (rect.width() - H)).clamp(0.0, 1.0))
}

/// A label with an on/off switch; returns true when toggled.
fn switch_row(ui: &mut egui::Ui, label: &str, on: &mut bool, t: &Tokens) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(label).size(17.0).color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, resp) = ui.allocate_exact_size(vec2(60.0, 34.0), Sense::click());
            let p = ui.painter();
            p.rect_filled(rect, CornerRadius::same(17), if *on { t.accent } else { t.field_border });
            let x = if *on { rect.right() - 17.0 } else { rect.left() + 17.0 };
            p.circle_filled(pos2(x, rect.center().y), 13.0, Color32::WHITE);
            if resp.clicked() {
                *on = !*on;
                changed = true;
            }
        });
    });
    changed
}

// ---------------------------------------------------------------------------------------------
// Palm rejection. The Android shell's touch filter (patched winit) asks [`finger_allowed`] for
// every finger that goes down: on the image it is dropped, so a resting palm or knuckle never
// paints or drags the stroke across the canvas. Fingers still work everywhere else (tool rail,
// bars, popovers, dialogs), and Menu › Dibujar con el dedo lets them paint.
// ---------------------------------------------------------------------------------------------

/// Where the image is on screen (points), from the last frame.
static CANVAS_RECT: Mutex<Option<Rect>> = Mutex::new(None);

/// Menu › Dibujar con el dedo: fingers paint on the canvas too (no palm rejection).
pub static FINGER_PAINT: AtomicBool = AtomicBool::new(false);

/// The canvas reports its rectangle each frame (touch interface only).
pub(crate) fn note_canvas(rect: Rect) {
    *CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner) = Some(rect);
}

/// Whether a finger going down at `pos` (points) reaches the app: not on the image (unless
/// finger painting is on), unless a window, popover or dialog covers that spot.
pub fn finger_allowed(ctx: &egui::Context, pos: Pos2) -> bool {
    if FINGER_PAINT.load(Ordering::Relaxed) {
        return true;
    }
    let canvas = *CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(canvas) = canvas else { return true };
    if !canvas.contains(pos) {
        return true;
    }
    // Panels and the canvas live on the background layer; anything above it is UI.
    ctx.layer_id_at(pos).is_some_and(|l| l.order != egui::Order::Background)
}

fn fg_color(app: &PhotocraftApp) -> Color32 {
    let c = app.session.tools.foreground;
    let to8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(to8(c[0]), to8(c[1]), to8(c[2]))
}

/// The colour the UI shows as the foreground: the one being dragged in the picker (not sent to
/// the engine yet), else the engine's.
fn shown_fg(app: &PhotocraftApp, st: &TouchUi) -> Color32 {
    st.picking.map_or_else(|| fg_color(app), |hsv| hsv_to_color(hsv))
}

fn set_fg(app: &mut PhotocraftApp, st: &mut TouchUi, c: Color32) {
    if c == fg_color(app) {
        return;
    }
    run(app, "tools.setColors", json!({ "foreground": format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()) }));
    st.recent.retain(|r| *r != c);
    st.recent.insert(0, c);
    st.recent.truncate(RECENT_MAX);
}

const RECENT_MAX: usize = 10;

fn color_well(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let (outer, resp) = ui.allocate_exact_size(Vec2::splat(BTN), Sense::click());
    let c = outer.center();
    let p = ui.painter();
    p.circle_filled(c, BTN / 2.0 - 2.0, shown_fg(app, st));
    p.circle_stroke(c, BTN / 2.0 - 2.0, Stroke::new(3.0, if st.color_open { t.accent } else { t.text }));
    if resp.clicked() {
        st.color_open = !st.color_open;
        st.menu_open = false;
        st.brush_open = false;
        st.color_anchor = Some(pos2(outer.right(), outer.bottom() + 6.0));
        st.hsv = color_to_hsv(fg_color(app), st.hsv);
    }
}

/// HSV in 0..1, kept by the picker so the hue survives greys and black (an RGB round trip would
/// lose it and make the picker jump).
type Hsv = [f32; 3];

fn hsv_to_color([h, s, v]: Hsv) -> Color32 {
    let h6 = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let to8 = |f: f32| ((f + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(to8(r), to8(g), to8(b))
}

/// `c` as HSV, keeping `prev`'s hue (and saturation) where `c` doesn't define them.
fn color_to_hsv(c: Color32, prev: Hsv) -> Hsv {
    let (r, g, b) = (f32::from(c.r()) / 255.0, f32::from(c.g()) / 255.0, f32::from(c.b()) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let v = max;
    let s = if max > 0.0 { d / max } else { prev[1] };
    let h = if d <= f32::EPSILON {
        prev[0]
    } else if max == r {
        ((g - b) / d).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    [h, s, v]
}

/// A touch colour picker: a large saturation/brightness square and a hue bar, both dragged with
/// the pen or a finger; the old and new colours side by side; recent colours and swatches. The
/// colour shows everywhere at once while dragging and is set (one command) on release.
fn color_popover(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let size = vec2(380.0, 560.0);
    // Below the colour well, right-aligned with it.
    let anchor = st.color_anchor.unwrap_or(pos2(screen.right() - 80.0, BAR_H + 6.0));
    let left = (anchor.x - size.x).max(screen.left() + 8.0);
    let mut close = false;
    let mut commit: Option<Color32> = None;
    egui::Area::new(egui::Id::new("touch-color")).order(egui::Order::Foreground).fixed_pos(pos2(left, anchor.y)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            let w = size.x - 32.0;
            ui.set_width(w);
            let mut hsv = st.picking.unwrap_or(st.hsv);
            let mut dragging = false;

            // Saturation (x) / brightness (y) square in the current hue.
            let (sq, resp) = ui.allocate_exact_size(vec2(w, 230.0), Sense::click_and_drag());
            let p = ui.painter_at(sq);
            let mut mesh = egui::Mesh::default();
            let n = 24u32;
            for j in 0..=n {
                for i in 0..=n {
                    let (fs, fv) = (i as f32 / n as f32, 1.0 - j as f32 / n as f32);
                    let pos = pos2(sq.left() + sq.width() * i as f32 / n as f32, sq.top() + sq.height() * j as f32 / n as f32);
                    mesh.colored_vertex(pos, hsv_to_color([hsv[0], fs, fv]));
                }
            }
            for j in 0..n {
                for i in 0..n {
                    let a = j * (n + 1) + i;
                    mesh.add_triangle(a, a + 1, a + n + 1);
                    mesh.add_triangle(a + 1, a + n + 2, a + n + 1);
                }
            }
            p.add(mesh);
            p.rect_stroke(sq, CornerRadius::same(10), Stroke::new(1.0, t.card_border), StrokeKind::Inside);
            if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() || resp.dragged()) {
                hsv[1] = ((pt.x - sq.left()) / sq.width()).clamp(0.0, 1.0);
                hsv[2] = 1.0 - ((pt.y - sq.top()) / sq.height()).clamp(0.0, 1.0);
                dragging = true;
            }
            let knob = pos2(sq.left() + sq.width() * hsv[1], sq.top() + sq.height() * (1.0 - hsv[2]));
            // Unclipped, so the knob shows whole at the square's edges.
            ui.painter().circle_stroke(knob, 13.0, Stroke::new(3.0, Color32::WHITE));
            ui.painter().circle_stroke(knob, 15.0, Stroke::new(1.5, Color32::from_black_alpha(140)));
            ui.add_space(14.0);

            // Hue bar.
            let (bar, resp) = ui.allocate_exact_size(vec2(w, 40.0), Sense::click_and_drag());
            let p = ui.painter_at(bar);
            let mut mesh = egui::Mesh::default();
            let steps = 36u32;
            for i in 0..=steps {
                let f = i as f32 / steps as f32;
                let c = hsv_to_color([f, 1.0, 1.0]);
                let x = bar.left() + bar.width() * f;
                mesh.colored_vertex(pos2(x, bar.top()), c);
                mesh.colored_vertex(pos2(x, bar.bottom()), c);
                if i > 0 {
                    let k = 2 * i;
                    mesh.add_triangle(k - 2, k - 1, k);
                    mesh.add_triangle(k - 1, k + 1, k);
                }
            }
            p.add(mesh);
            if let Some(pt) = resp.interact_pointer_pos().filter(|_| resp.is_pointer_button_down_on() || resp.dragged()) {
                hsv[0] = ((pt.x - bar.left()) / bar.width()).clamp(0.0, 0.9999);
                dragging = true;
            }
            let hx = bar.left() + bar.width() * hsv[0];
            let thumb = Rect::from_center_size(pos2(hx, bar.center().y), vec2(14.0, bar.height() + 8.0));
            p.rect_filled(thumb, CornerRadius::same(7), hsv_to_color([hsv[0], 1.0, 1.0]));
            p.rect_stroke(thumb, CornerRadius::same(7), Stroke::new(3.0, Color32::WHITE), StrokeKind::Outside);
            ui.add_space(14.0);

            // Old | new.
            let (pr, _) = ui.allocate_exact_size(vec2(w, 44.0), Sense::hover());
            let half = pr.width() / 2.0;
            let old_r = Rect::from_min_size(pr.min, vec2(half, pr.height()));
            let new_r = Rect::from_min_size(pos2(pr.left() + half, pr.top()), vec2(half, pr.height()));
            ui.painter().rect_filled(old_r, CornerRadius { nw: 10, sw: 10, ne: 0, se: 0 }, fg_color(app));
            ui.painter().rect_filled(new_r, CornerRadius { nw: 0, sw: 0, ne: 10, se: 10 }, hsv_to_color(hsv));
            let hex = hsv_to_color(hsv);
            ui.painter().text(
                new_r.center(),
                Align2::CENTER_CENTER,
                format!("#{:02X}{:02X}{:02X}", hex.r(), hex.g(), hex.b()),
                egui::FontId::monospace(15.0),
                if hsv[2] > 0.6 && hsv[1] < 0.6 { Color32::BLACK } else { Color32::WHITE },
            );
            ui.add_space(12.0);

            if dragging {
                st.picking = Some(hsv);
                ui.ctx().request_repaint();
            } else if let Some(h) = st.picking.take() {
                // Released: set the colour once.
                st.hsv = h;
                commit = Some(hsv_to_color(h));
            }

            // Recent colours, then the swatches.
            let mut swatch_row = |ui: &mut egui::Ui, colors: &[Color32], label: &str| {
                if colors.is_empty() {
                    return;
                }
                ui.label(egui::RichText::new(label).size(14.0).color(t.text_dim));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    for &col in colors {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(42.0), Sense::click());
                        ui.painter().rect_filled(r, CornerRadius::same(21), col);
                        ui.painter().rect_stroke(r, CornerRadius::same(21), Stroke::new(1.0, t.card_border), StrokeKind::Inside);
                        if resp.clicked() {
                            commit = Some(col);
                        }
                    }
                });
                ui.add_space(8.0);
            };
            let recent = st.recent.clone();
            swatch_row(ui, &recent, "Recientes");
            let swatches: Vec<Color32> = SWATCHES.iter().map(|s| Color32::from_rgb(s[0], s[1], s[2])).collect();
            swatch_row(ui, &swatches, "Muestras");
            if big_button(ui, "Listo", t).clicked() {
                close = true;
            }
        });
    });
    if let Some(c) = commit {
        set_fg(app, st, c);
        st.hsv = color_to_hsv(c, st.hsv);
    }
    if close {
        st.color_open = false;
        st.picking = None;
    }
}
fn popover_frame(t: &Tokens) -> egui::Frame {
    egui::Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.card_border))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(egui::Margin::same(16))
        .shadow(egui::Shadow { offset: [0, 6], blur: 24, spread: 0, color: Color32::from_black_alpha(90) })
}

fn big_button(ui: &mut egui::Ui, text: &str, t: &Tokens) -> egui::Response {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 50.0), Sense::click());
    let fill = if resp.is_pointer_button_down_on() { t.pressed } else { t.field };
    ui.painter().rect_filled(r, CornerRadius::same(12), fill);
    ui.painter().text(r.center(), Align2::CENTER_CENTER, text, egui::FontId::proportional(19.0), t.text);
    resp
}

fn menu_popover(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let mut close = false;
    egui::Area::new(egui::Id::new("touch-menu")).order(egui::Order::Foreground).fixed_pos(pos2(12.0, BAR_H + 6.0)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            ui.set_width(320.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            if big_button(ui, "Nuevo lienzo", t).clicked() {
                new_canvas(app, ctx);
                close = true;
            }
            if big_button(ui, "Abrir imagen…", t).clicked() {
                open_images(app, st, ctx);
                close = true;
            }
            ui.add_space(4.0);
            if big_button(ui, "Exportar PNG", t).clicked() {
                export_image(app, st, ctx, "png", false);
                close = true;
            }
            if big_button(ui, "Exportar PNG sin fondo", t).clicked() {
                export_image(app, st, ctx, "png", true);
                close = true;
            }
            if big_button(ui, "Exportar JPG", t).clicked() {
                export_image(app, st, ctx, "jpg", false);
                close = true;
            }
            ui.add_space(4.0);
            if big_button(ui, "Ajustar a la pantalla", t).clicked() {
                invoke(app, ctx, "view.fitOnScreen");
                close = true;
            }
            let finger = FINGER_PAINT.load(Ordering::Relaxed);
            if big_button(ui, if finger { "Dibujar con el dedo: Sí" } else { "Dibujar con el dedo: No" }, t).clicked() {
                FINGER_PAINT.store(!finger, Ordering::Relaxed);
            }
            if big_button(ui, "Interfaz de escritorio", t).clicked() {
                app.touch_off_requested = true;
                close = true;
            }
        });
    });
    if close {
        st.menu_open = false;
    }
}

// ---------------------------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------------------------

/// Export the active document as `ext` (png, jpg) into the export folder, named after it with a
/// time stamp. `transparent`: without the Background layer, so a PNG keeps only what is drawn
/// (after Quitar fondo, the subject alone). Without an export folder, the Export As dialog.
fn export_image(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, ext: &str, transparent: bool) {
    let Some(dir) = st.export_dir.clone() else {
        invoke(app, ctx, "file.export.exportAs");
        return;
    };
    let Some(d) = app.session.active() else {
        notify(st, ctx, "No hay ningún documento abierto");
        return;
    };
    let mut doc = (*d.doc).clone();
    if transparent && let Some(bg) = doc.background_mut() {
        bg.visible = false;
    }
    let stem: String = doc.name.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect();
    let stem = stem.split('.').next().filter(|s| !s.is_empty()).unwrap_or("StylusArt").to_string();
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let suffix = if transparent { "-sin-fondo" } else { "" };
    let path = dir.join(format!("{stem}{suffix}-{stamp}.{ext}"));
    let Some(export) = app.services.export.as_ref() else {
        notify(st, ctx, "Exportar no está disponible");
        return;
    };
    let settings = crate::ExportSettings { jpeg_quality: Some(92), ..Default::default() };
    let result = std::fs::create_dir_all(&dir)
        .map_err(|e| e.to_string())
        .and_then(|()| export(&doc, &path.to_string_lossy(), &settings))
        .and_then(|(bytes, _)| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
    match result {
        Ok(()) => {
            let shown = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            notify(st, ctx, &format!("Guardado en Imágenes/StylusArt: {shown}"));
        }
        Err(e) => notify(st, ctx, &format!("No se pudo exportar: {e}")),
    }
}

// ---------------------------------------------------------------------------------------------
// Image gallery (open photos)
// ---------------------------------------------------------------------------------------------

/// Image files the gallery lists, newest first, and their thumbnails as a worker decodes them.
struct Gallery {
    items: Vec<(PathBuf, String)>,
    thumbs: Receiver<(usize, u32, u32, Vec<u8>)>,
    textures: HashMap<usize, egui::TextureHandle>,
}

const GALLERY_MAX: usize = 240;
const THUMB_PX: u32 = 256;

fn open_images(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context) {
    if st.gallery_dirs.is_empty() {
        invoke(app, ctx, "file.open");
        return;
    }
    if let Some(ask) = &st.request_media {
        ask();
    }
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for dir in &st.gallery_dirs {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            let ext = p.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "psd" | "pcraft") {
                let modified = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                files.push((modified, p));
            }
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.truncate(GALLERY_MAX);
    let items: Vec<(PathBuf, String)> = files.into_iter().map(|(_, p)| {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        (p, name)
    }).collect();
    let (tx, rx) = channel();
    let paths: Vec<PathBuf> = items.iter().map(|(p, _)| p.clone()).collect();
    let repaint = ctx.clone();
    let _ = std::thread::Builder::new().name("stylusart-thumbs".into()).spawn(move || {
        for (i, p) in paths.into_iter().enumerate() {
            let Some((w, h, rgba)) = thumbnail(&p) else { continue };
            if tx.send((i, w, h, rgba)).is_err() {
                return;
            }
            repaint.request_repaint();
        }
    });
    st.gallery = Some(Gallery { items, thumbs: rx, textures: HashMap::new() });
}

/// A small RGBA preview of an image file (box-filtered to at most `THUMB_PX` on a side).
fn thumbnail(path: &std::path::Path) -> Option<(u32, u32, Vec<u8>)> {
    let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
    if !matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    let img = photocraft_codecs::decode(&bytes).ok()?;
    let (w, h) = (img.width().max(1), img.height().max(1));
    let rgba = img.to_rgba8();
    let step = w.max(h).div_ceil(THUMB_PX).max(1);
    let (tw, th) = (w.div_ceil(step), h.div_ceil(step));
    let mut out = vec![0u8; (tw * th * 4) as usize];
    for ty in 0..th {
        for tx in 0..tw {
            let mut acc = [0u32; 4];
            let mut n = 0u32;
            for y in (ty * step)..((ty + 1) * step).min(h) {
                for x in (tx * step)..((tx + 1) * step).min(w) {
                    let i = ((y * w + x) * 4) as usize;
                    if let Some(px) = rgba.get(i..i + 4) {
                        for (a, v) in acc.iter_mut().zip(px) {
                            *a += u32::from(*v);
                        }
                        n += 1;
                    }
                }
            }
            let o = ((ty * tw + tx) * 4) as usize;
            if let (Some(dst), true) = (out.get_mut(o..o + 4), n > 0) {
                for (d, a) in dst.iter_mut().zip(acc) {
                    *d = (a / n) as u8;
                }
            }
        }
    }
    Some((tw, th, out))
}

fn gallery_sheet(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let Some(g) = st.gallery.as_mut() else { return };
    while let Ok((i, w, h, rgba)) = g.thumbs.try_recv() {
        let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
        g.textures.insert(i, ctx.load_texture(format!("stylusart-thumb-{i}"), img, egui::TextureOptions::LINEAR));
    }
    let screen = ctx.content_rect();
    let rect = screen.shrink2(vec2(screen.width() * 0.08, 0.0)).with_min_y(screen.top() + BAR_H + 12.0).with_max_y(screen.bottom() - 16.0);
    let mut close = false;
    let mut open: Option<usize> = None;
    egui::Area::new(egui::Id::new("touch-gallery")).order(egui::Order::Foreground).fixed_pos(rect.min).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            ui.set_width(rect.width() - 32.0);
            ui.set_height(rect.height() - 32.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Abrir imagen").size(22.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button_with_icon_size(ui, "x", 44.0, 22.0, false, "Cerrar").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(8.0);
            if g.items.is_empty() {
                hint(ui, "No hay imágenes en Imágenes, Cámara ni Descargas, o falta el permiso para verlas. Acepta el permiso de fotos y vuelve a abrir esta ventana.", t);
                return;
            }
            let tile = 180.0;
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                    for (i, (_, name)) in g.items.iter().enumerate() {
                        let (r, resp) = ui.allocate_exact_size(vec2(tile, tile + 28.0), Sense::click());
                        let img_r = Rect::from_min_size(r.min, Vec2::splat(tile));
                        ui.painter().rect_filled(img_r, CornerRadius::same(10), t.field);
                        if let Some(tex) = g.textures.get(&i) {
                            let s = tex.size_vec2();
                            let k = (tile / s.x).min(tile / s.y);
                            let fit = Rect::from_center_size(img_r.center(), s * k);
                            ui.painter().image(tex.id(), fit, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        let label: String = name.chars().take(22).collect();
                        ui.painter().text(pos2(r.left() + 4.0, img_r.bottom() + 14.0), Align2::LEFT_CENTER, label, egui::FontId::proportional(14.0), t.text_dim);
                        if resp.clicked() {
                            open = Some(i);
                        }
                    }
                });
            });
        });
    });
    if let Some(i) = open
        && let Some((path, name)) = g.items.get(i).cloned()
    {
        match std::fs::read(&path) {
            Ok(bytes) => match app.services.inbox.as_ref() {
                Some(inbox) => {
                    inbox.lock().unwrap_or_else(PoisonError::into_inner).push((name, bytes));
                    close = true;
                }
                None => notify(st, ctx, "Abrir archivos no está disponible"),
            },
            Err(e) => notify(st, ctx, &format!("No se pudo leer {name}: {e}")),
        }
    }
    if close {
        st.gallery = None;
    }
}

// ---------------------------------------------------------------------------------------------
// Layers sheet
// ---------------------------------------------------------------------------------------------

/// One row of the Layers sheet.
struct Row {
    id: u64,
    name: String,
    visible: bool,
    opacity: f32,
    depth: usize,
    mask: bool,
    clipped: bool,
    group: bool,
    artboard: bool,
}

/// The layer tree topmost first, groups (and artboards) followed by their contents, indented.
fn rows(layers: &[Layer], depth: usize, out: &mut Vec<Row>) {
    for l in layers.iter().rev() {
        let (group, artboard) = match &l.content {
            LayerContent::Group(g) => (true, g.artboard.is_some()),
            _ => (false, false),
        };
        out.push(Row {
            id: l.id.0,
            name: l.name.clone(),
            visible: l.visible,
            opacity: l.opacity,
            depth,
            mask: l.mask.is_some(),
            clipped: l.clipped,
            group,
            artboard,
        });
        if let Some(children) = l.children() {
            rows(children, depth + 1, out);
        }
    }
}

fn layers_sheet(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::right("touch-layers")
        .resizable(false)
        .exact_size(LAYERS_W)
        .frame(egui::Frame::NONE.fill(t.dock).inner_margin(egui::Margin::same(12)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Capas").size(22.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, "trash", false, "Eliminar capa").clicked() {
                        run(app, "layer.delete", json!({}));
                    }
                    if icon_button(ui, "copy", false, "Duplicar capa").clicked() {
                        run(app, "layer.duplicate", json!({}));
                    }
                    if icon_button(ui, "plus", false, "Nueva capa").clicked() {
                        run(app, "layer.new.layer", json!({}));
                    }
                });
            });
            ui.add_space(8.0);
            let Some(d) = app.session.active() else {
                ui.label("Sin documento");
                return;
            };
            let active = d.active_layer.map(|l| l.0);
            let mut list = Vec::new();
            rows(&d.doc.layers, 0, &mut list);
            let current = list.iter().find(|r| Some(r.id) == active).map(|r| (r.id, r.opacity, r.mask, r.clipped));
            let blend = active.and_then(|id| d.doc.layer(photocraft_doc::LayerId(id))).map(|l| l.blend);
            let ctx = ui.ctx().clone();
            // Actions on the active layer.
            if let Some((id, _, mask, clipped)) = current {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                    if chip(ui, "Quitar fondo", false, t).clicked() {
                        remove_background(app, st, &ctx);
                    }
                    if chip(ui, if mask { "Quitar máscara" } else { "Añadir máscara" }, mask, t).clicked() {
                        let r = app.run(if mask { "layer.layerMask.delete" } else { "layer.layerMask.revealAll" }, json!({}));
                        if let Err(e) = r {
                            notify(st, &ctx, &e);
                        }
                    }
                    if chip(ui, "Máscara de recorte", clipped, t).clicked()
                        && let Err(e) = app.run("layer.setProps", json!({ "layer": id, "clipped": !clipped }))
                    {
                        notify(st, &ctx, &e);
                    }
                });
                ui.add_space(8.0);
                if let Some(mode) = blend {
                    blend_picker(app, st, ui, id, mode, t);
                    ui.add_space(8.0);
                }
            }
            egui::ScrollArea::vertical().max_height((ui.available_height() - 90.0).max(120.0)).show(ui, |ui| {
                for r in &list {
                    layer_row(app, ui, r, Some(r.id) == active, t);
                }
            });
            if let Some((id, op, _, _)) = current {
                ui.add_space(10.0);
                ui.label(egui::RichText::new("Opacidad de la capa").size(16.0).color(t.text_dim));
                let mut v = st.opacity_drag.filter(|(l, _)| *l == id).map_or(op, |(_, v)| v) * 100.0;
                ui.spacing_mut().slider_width = ui.available_width() - 70.0;
                ui.spacing_mut().interact_size.y = 40.0;
                let resp = ui.add(egui::Slider::new(&mut v, 0.0..=100.0).suffix(" %").max_decimals(0));
                if resp.changed() {
                    st.opacity_drag = Some((id, v / 100.0));
                }
                if let Some((l, val)) = st.opacity_drag
                    && !resp.dragged()
                    && !resp.has_focus()
                {
                    st.opacity_drag = None;
                    if (val - op).abs() > f32::EPSILON {
                        run(app, "layer.setProps", json!({ "layer": l, "opacity": val }));
                    }
                }
            }
        });
}

/// Quitar fondo: Select Subject on the active layer, then a layer mask from that selection (the
/// Background layer becomes a normal layer first, as masks need one). Hidden pixels stay under
/// the mask, so painting the mask black or white fixes the edge.
fn remove_background(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context) {
    let Some(d) = app.session.active() else { return };
    let Some(layer) = d.active_layer else {
        notify(st, ctx, "Elige una capa primero");
        return;
    };
    let is_bg = d.doc.is_background(layer);
    let has_mask = d.doc.layer(layer).is_some_and(|l| l.mask.is_some());
    if is_bg && let Err(e) = app.run("layer.new.layerFromBackground", json!({})) {
        notify(st, ctx, &format!("No se pudo convertir el fondo: {e}"));
        return;
    }
    if has_mask {
        let _ = app.run("layer.layerMask.delete", json!({}));
    }
    if let Err(e) = app.run("select.subject", json!({ "sampleAllLayers": false })) {
        notify(st, ctx, &format!("No se encontró un sujeto: {e}"));
        return;
    }
    match app.run("layer.layerMask.revealSelection", json!({})) {
        Ok(_) => {
            let _ = app.run("select.deselect", json!({}));
            notify(st, ctx, "Fondo quitado. Exporta con \"Exportar PNG sin fondo\".");
        }
        Err(e) => notify(st, ctx, &format!("No se pudo crear la máscara: {e}")),
    }
}

fn layer_row(app: &mut PhotocraftApp, ui: &mut egui::Ui, row: &Row, selected: bool, t: &Tokens) {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click());
    let p = ui.painter();
    let indent = row.depth as f32 * 18.0;
    let body = r.with_min_x(r.left() + indent);
    p.rect_filled(body, CornerRadius::same(12), if selected { t.row_selected } else { t.card });
    let eye = Rect::from_min_size(body.min, vec2(ROW_H, ROW_H));
    icons::paint(ui, eye, if row.visible { "eye" } else { "eye-off" }, 26.0, if row.visible { t.text } else { t.text_faint });
    let mut x = eye.right() + 2.0;
    let badge = |name: &str, x: f32| icons::paint(ui, Rect::from_center_size(pos2(x + 11.0, r.center().y), Vec2::splat(22.0)), name, 18.0, t.text_dim);
    if row.clipped {
        badge("clip-below", x);
        x += 24.0;
    }
    if row.artboard {
        badge("artboard", x);
        x += 24.0;
    } else if row.group {
        badge("folder", x);
        x += 24.0;
    }
    let name_w = (body.right() - x - (if row.mask { 34.0 } else { 8.0 })).max(20.0);
    let galley = ui.painter().layout(row.name.clone(), egui::FontId::proportional(17.0), t.text, name_w);
    ui.painter().galley(pos2(x + 2.0, r.center().y - galley.size().y / 2.0), galley, t.text);
    if row.mask {
        icons::paint(ui, Rect::from_center_size(pos2(body.right() - 20.0, r.center().y), Vec2::splat(24.0)), "layer-mask", 20.0, t.text_dim);
    }
    if resp.clicked() {
        let tap = resp.interact_pointer_pos().unwrap_or(Pos2::ZERO);
        if eye.contains(tap) {
            run(app, "layer.setProps", json!({ "layer": row.id, "visible": !row.visible }));
        } else if !selected {
            run(app, "layer.select", json!({ "layer": row.id }));
        }
    }
    ui.add_space(6.0);
}

// ---------------------------------------------------------------------------------------------
// Canvas gestures. With palm rejection, fingers on the image never reach egui; the platform
// shell feeds every touch to [`feed_touch`] instead and these gestures read them:
// - two fingers moving: pan, pinch zoom and rotate the view around the fingers;
// - a quick two-finger tap: undo; a quick three-finger tap: redo.
// A contact as large as a palm, and any finger that lands while the pen is in use (or was a
// moment ago), is ignored, so a resting hand neither paints nor moves the canvas.
// ---------------------------------------------------------------------------------------------

/// What happened to a touch point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// One touch event from the platform, in points.
#[derive(Clone, Copy, Debug)]
pub struct TouchEvent {
    pub id: u64,
    /// A pen (or its eraser end) rather than a finger.
    pub pen: bool,
    pub phase: TouchPhase,
    pub pos: Pos2,
    /// The contact's major axis, in points.
    pub size: f32,
    /// The platform kept it from the app's input (a finger on the image).
    pub dropped: bool,
}

static TOUCH_QUEUE: Mutex<Vec<(TouchEvent, std::time::Instant)>> = Mutex::new(Vec::new());

/// The platform shell reports every touch here (any thread); the touch interface reads them on
/// the next frame.
pub fn feed_touch(e: TouchEvent) {
    let mut q = TOUCH_QUEUE.lock().unwrap_or_else(PoisonError::into_inner);
    if q.len() < 4096 {
        q.push((e, std::time::Instant::now()));
    }
}

/// A contact this wide (points, about 18 mm on the Pad 7) is a palm or the side of a hand.
const PALM_SIZE: f32 = 120.0;
/// A finger that lands this soon after the pen is ignored: it is the hand holding the pen.
const PEN_GRACE: f32 = 0.35;
/// A tap must end this soon and move this little.
const TAP_SECS: f32 = 0.35;
const TAP_SLOP: f32 = 24.0;

#[derive(Default)]
struct Gestures {
    /// Fingers on the image that count: id -> (where it landed, where it is).
    fingers: Vec<(u64, Pos2, Pos2)>,
    /// Ignored contacts (palms, the hand holding the pen) until they lift.
    ignored: Vec<u64>,
    /// Pens touching or hovering.
    pens: Vec<u64>,
    last_pen: Option<std::time::Instant>,
    /// The current multi-finger gesture: when it started, most fingers seen, whether it moved.
    started: Option<std::time::Instant>,
    max_fingers: usize,
    moved: bool,
    /// The two-finger frame the view was last moved from: centroid, distance, angle.
    last: Option<(Pos2, f32, f32)>,
}

static GESTURES: Mutex<Option<Gestures>> = Mutex::new(None);

/// Read this frame's touches and apply the gestures (view changes, undo, redo).
fn run_gestures(app: &mut PhotocraftApp, ctx: &egui::Context) {
    let events = std::mem::take(&mut *TOUCH_QUEUE.lock().unwrap_or_else(PoisonError::into_inner));
    if events.is_empty() {
        return;
    }
    let mut guard = GESTURES.lock().unwrap_or_else(PoisonError::into_inner);
    let g = guard.get_or_insert_with(Gestures::default);
    let mut undo_redo: Option<&str> = None;
    for (e, at) in events {
        if e.pen {
            g.last_pen = Some(at);
            match e.phase {
                TouchPhase::Start | TouchPhase::Move => {
                    if !g.pens.contains(&e.id) {
                        g.pens.push(e.id);
                    }
                }
                TouchPhase::End | TouchPhase::Cancel => g.pens.retain(|p| *p != e.id),
            }
            continue;
        }
        // Fingers the app receives itself (on the bars, panels, popovers) are not gestures.
        if !e.dropped {
            continue;
        }
        match e.phase {
            TouchPhase::Start => {
                let pen_busy = !g.pens.is_empty() || g.last_pen.is_some_and(|t| at.duration_since(t).as_secs_f32() < PEN_GRACE);
                if e.size >= PALM_SIZE || pen_busy || g.ignored.contains(&e.id) {
                    g.ignored.push(e.id);
                    continue;
                }
                g.fingers.retain(|f| f.0 != e.id);
                g.fingers.push((e.id, e.pos, e.pos));
                if g.fingers.len() == 1 {
                    g.started = Some(at);
                    g.max_fingers = 1;
                    g.moved = false;
                }
                g.max_fingers = g.max_fingers.max(g.fingers.len());
                g.last = None;
            }
            TouchPhase::Move => {
                if e.size >= PALM_SIZE && !g.ignored.contains(&e.id) {
                    // It grew into a palm: drop it from the gesture.
                    g.ignored.push(e.id);
                    g.fingers.retain(|f| f.0 != e.id);
                    g.last = None;
                    continue;
                }
                if let Some(f) = g.fingers.iter_mut().find(|f| f.0 == e.id) {
                    f.2 = e.pos;
                    if (f.2 - f.1).length() > TAP_SLOP {
                        g.moved = true;
                    }
                }
            }
            TouchPhase::End | TouchPhase::Cancel => {
                if g.ignored.contains(&e.id) {
                    g.ignored.retain(|i| *i != e.id);
                    continue;
                }
                let was = g.fingers.len();
                g.fingers.retain(|f| f.0 != e.id);
                g.last = None;
                if was > 0 && g.fingers.is_empty() {
                    let quick = g.started.is_some_and(|s| at.duration_since(s).as_secs_f32() < TAP_SECS);
                    if quick && !g.moved && e.phase == TouchPhase::End {
                        undo_redo = match g.max_fingers {
                            2 => Some("history.undo"),
                            3 => Some("history.redo"),
                            _ => None,
                        };
                    }
                    g.started = None;
                }
            }
        }
    }
    // Two fingers: move the view from the last two-finger frame to this one.
    let frame = (g.fingers.len() == 2).then(|| {
        let (a, b) = (g.fingers[0].2, g.fingers[1].2);
        let d = b - a;
        (pos2((a.x + b.x) / 2.0, (a.y + b.y) / 2.0), d.length().max(1.0), d.y.atan2(d.x))
    });
    if let (Some(now), Some(prev)) = (frame, g.last) {
        let started_moving = g.moved || (now.0 - prev.0).length() > 2.0 || (now.1 / prev.1 - 1.0).abs() > 0.02;
        if started_moving {
            g.moved = true;
            move_view(app, ctx, prev, now);
        }
    }
    g.last = frame;
    drop(guard);
    if let Some(id) = undo_redo {
        run(app, id, json!({}));
    }
    ctx.request_repaint();
}

/// Pan, zoom and rotate the active view so the document point under the fingers' previous
/// centroid ends under the new one, scaled by their spread and turned by their angle.
fn move_view(app: &mut PhotocraftApp, ctx: &egui::Context, prev: (Pos2, f32, f32), now: (Pos2, f32, f32)) {
    let Some(i) = app.session.active_index() else { return };
    let Some(rect) = *CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner) else { return };
    let size = app.session.documents().get(i).map_or([0, 0], |st| [st.doc.size.width, st.doc.size.height]);
    let flip = app.ui.view.flip_horizontal;
    let ppp = ctx.pixels_per_point().max(0.1);
    let Some(view) = app.ui.views.get_mut(i) else { return };
    let xf = |zoom: f32, center: [f32; 2], rotation: f32| crate::canvas::ViewXform { rect, zoom: zoom / ppp, center, flip, rotation, aspect: 1.0 };
    let anchor = xf(view.zoom, view.center, view.rotation).to_doc(prev.0);
    let zoom = crate::zoom_levels::clamp(view.zoom * (now.1 / prev.1), size);
    let mut turn = (now.2 - prev.2).to_degrees();
    if turn > 180.0 {
        turn -= 360.0;
    } else if turn < -180.0 {
        turn += 360.0;
    }
    let rotation = crate::rotate_view::wrap_deg(view.rotation + if flip { -turn } else { turn });
    let under = xf(zoom, view.center, rotation).to_doc(now.0);
    view.center = [view.center[0] + (anchor[0] - under[0]) as f32, view.center[1] + (anchor[1] - under[1]) as f32];
    view.zoom = zoom;
    view.rotation = rotation;
    view.fit_pending = false;
    view.fill_pending = false;
}

/// Layer blend modes offered on the Layers sheet: (engine name, label).
const BLEND_MODES: &[(&str, &str)] = &[
    ("Normal", "Normal"),
    ("Multiply", "Multiplicar"),
    ("Screen", "Trama"),
    ("Overlay", "Superponer"),
    ("SoftLight", "Luz suave"),
    ("HardLight", "Luz fuerte"),
    ("ColorDodge", "Sobreexponer color"),
    ("ColorBurn", "Subexponer color"),
    ("LinearDodge", "Sobreexposición lineal (añadir)"),
    ("Darken", "Oscurecer"),
    ("Lighten", "Aclarar"),
    ("Difference", "Diferencia"),
    ("Hue", "Tono"),
    ("Saturation", "Saturación"),
    ("Color", "Color"),
    ("Luminosity", "Luminosidad"),
];

fn blend_label(mode: photocraft_color::BlendMode) -> String {
    let id = format!("{mode:?}");
    BLEND_MODES.iter().find(|(n, _)| *n == id).map_or_else(|| mode.label().to_string(), |(_, l)| (*l).to_string())
}

/// The active layer's blend mode: a row that opens the list of modes.
fn blend_picker(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, layer: u64, current: photocraft_color::BlendMode, t: &Tokens) {
    let label = format!("Fusión: {}", blend_label(current));
    if chip(ui, &label, st.blend_open, t).clicked() {
        st.blend_open = !st.blend_open;
    }
    if !st.blend_open {
        return;
    }
    let id = format!("{current:?}");
    ui.add_space(6.0);
    egui::Frame::NONE.fill(t.card).corner_radius(CornerRadius::same(12)).inner_margin(egui::Margin::same(6)).show(ui, |ui| {
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for &(name, label) in BLEND_MODES {
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::click());
                let on = name == id;
                ui.painter().rect_filled(r, CornerRadius::same(8), if on { t.row_selected } else { Color32::TRANSPARENT });
                ui.painter().text(pos2(r.left() + 12.0, r.center().y), Align2::LEFT_CENTER, label, egui::FontId::proportional(17.0), t.text);
                if resp.clicked() {
                    if !on && let Err(e) = app.run("layer.setProps", json!({ "layer": layer, "blend": name })) {
                        notify(st, ui.ctx(), &e);
                    }
                    st.blend_open = false;
                }
            }
        });
    });
}

/// Transform the active layer (or the selection): free transform with handles, or one-tap flips
/// and quarter turns.
fn transform_options(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let anchor = st.transform_anchor.unwrap_or(pos2(RAIL_W + 10.0, BAR_H + 10.0));
    let height = 420.0;
    let lo = screen.top() + BAR_H + 6.0;
    let top = anchor.y.clamp(lo, (screen.bottom() - height - 8.0).max(lo));
    let mut close = false;
    egui::Area::new(egui::Id::new("touch-transform")).order(egui::Order::Foreground).fixed_pos(pos2(anchor.x, top)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            ui.set_width(BRUSH_POP_W - 32.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Transformar").size(20.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button_with_icon_size(ui, "x", 40.0, 20.0, false, "Cerrar").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(6.0);
            hint(ui, "Transforma la capa activa (o la selección). Con Transformación libre, arrastra las esquinas con el lápiz para escalar, fuera de ellas para rotar, y toca Aplicar.", t);
            ui.spacing_mut().item_spacing.y = 8.0;
            if big_button(ui, "Transformación libre", t).clicked() {
                invoke(app, ctx, "edit.freeTransform");
                close = true;
            }
            for (id, label) in [
                ("edit.transform.flipHorizontal", "Voltear horizontal"),
                ("edit.transform.flipVertical", "Voltear vertical"),
                ("edit.transform.rotate90Cw", "Rotar 90° a la derecha"),
                ("edit.transform.rotate90Ccw", "Rotar 90° a la izquierda"),
            ] {
                if big_button(ui, label, t).clicked()
                    && let Err(e) = app.run(id, json!({}))
                {
                    notify(st, ctx, &e);
                }
            }
        });
    });
    if close {
        st.transform_open = false;
    }
}
