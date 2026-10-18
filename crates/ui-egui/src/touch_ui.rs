//! Touch interface for tablets (StylusArt on Android), in place of the desktop menu bar, options
//! bar, toolbar and dock. The layout (top bar, tool rail with a properties strip, taskbar with
//! layer thumbnails, popovers) is in `fresco`; this module holds the state, the tool options, the
//! colour picker, export and the gallery, palm rejection, gestures, selection dragging and shape
//! snapping. The canvas, dialogs and every action stay the same: each
//! control here runs the engine command the desktop UI runs (`tools.setBrush`, `history.undo`,
//! `layer.*`, `tools.setColors`).
//!
//! On when [`PhotocraftApp::touch`] is `Some`. StylusArt (the Android app) runs only this
//! interface: there is no way to switch to the desktop one.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex, PoisonError};

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
use photocraft_doc::{Layer, LayerContent};
use serde_json::json;

use crate::PhotocraftApp;
use crate::icons;

mod fresco;
use fresco::{RAIL, TOP as BAR_TOP};
use crate::state::Tool;
use crate::theme::Tokens;

const BAR_H: f32 = 60.0;
const BRUSH_POP_W: f32 = 400.0;
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
    /// The rail's Lápiz is active (the Brush tool with its graphite brush), and the brushes the
    /// Lápiz and the Pincel keep while the other one is in use.
    pencil_mode: bool,
    pencil_brush: Option<photocraft_engine::paint::brush::BrushSettings>,
    brush_brush: Option<photocraft_engine::paint::brush::BrushSettings>,
    /// The Eraser got its own brush (hard, full opacity) the first time it was picked.
    eraser_ready: bool,
    /// A drag inside the selection switched to the Move tool; the selection tool to return to.
    spring_back: Option<Tool>,
    /// The Layers sheet's blend mode list is open.
    blend_open: bool,
    /// The Transform popover is open, and where (beside its rail button).
    pub transform_open: bool,
    brush_query: String,
    /// Top bar popovers: Inicio, Compartir, Ajustes, Ayuda; focus mode hides the side bars.
    pub home_open: bool,
    pub share_open: bool,
    pub settings_open: bool,
    pub help_open: bool,
    focus: bool,
    /// Properties strip popovers (size, smoothing) and where they open.
    pub size_open: bool,
    pub smooth_open: bool,
    dock_anchor: Option<Pos2>,
    /// Taskbar popovers.
    pub layer_props_open: bool,
    pub layer_more_open: bool,
    pub precision_open: bool,
    /// The variant each rail group shows (the one used last).
    variants: HashMap<&'static str, Tool>,
    /// The brush library's tab ("★" favourites, "" all, else a preset group) and favourites.
    library_tab: String,
    favorites: HashSet<String>,
    /// Where the favourite brushes are saved (the app's data folder); `None`: not saved.
    pub favorites_path: Option<PathBuf>,
    /// The tool the touch shortcut replaced with the eraser while held.
    shortcut_prev: Option<Tool>,
}

impl TouchUi {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Height (points) the system navigation bar covers at the bottom of the window, as the platform
/// shell reports it.
static SAFE_BOTTOM: AtomicU32 = AtomicU32::new(0);

/// The platform shell reports how much of the bottom edge the system bars cover (points).
pub fn set_safe_bottom(points: f32) {
    SAFE_BOTTOM.store(points.max(0.0).to_bits(), Ordering::Relaxed);
}

/// Room to leave at the bottom: the system bar, and never less than a comfortable margin above
/// the gesture area.
fn safe_bottom() -> f32 {
    f32::from_bits(SAFE_BOTTOM.load(Ordering::Relaxed)).clamp(28.0, 160.0)
}

/// Height a popover's content may take below `ui`'s top before the safe bottom; it scrolls past it.
fn popover_room(ui: &egui::Ui) -> f32 {
    (ui.ctx().content_rect().bottom() - safe_bottom() - ui.max_rect().top() - 24.0).max(160.0)
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
        fresco::load_favorites(&mut st);
    }
    dismiss_on_outside_tap(&mut st, ui.ctx());
    run_gestures(app, ui.ctx());
    drag_selection(app, &mut st, ui.ctx());
    snap_shape(app, &mut st, ui.ctx());
    let ctx = ui.ctx().clone();
    fresco::top_bar(app, &mut st, ui, &t);
    if !st.focus {
        fresco::tool_rail(app, &mut st, ui, &t);
        fresco::task_bar(app, &mut st, ui, &t);
        if st.layers_open {
            fresco::layer_thumbs(app, ui, &t);
        }
    }
    fresco::touch_shortcut(app, &mut st, &ctx, &t);
    if st.home_open {
        fresco::home_menu(app, &mut st, &ctx, &t);
    }
    if st.share_open {
        fresco::share_menu(app, &mut st, &ctx, &t);
    }
    if st.settings_open {
        fresco::settings_panel(app, &mut st, &ctx, &t);
    }
    if st.help_open {
        fresco::help_popover(&mut st, &ctx, &t);
    }
    if st.color_open {
        color_popover(app, &mut st, &ctx, &t);
    }
    if st.brush_open {
        tool_options(app, &mut st, &ctx, &t);
    }
    if st.size_open {
        fresco::size_popover(app, &mut st, &ctx, &t);
    }
    if st.smooth_open {
        fresco::smooth_popover(app, &mut st, &ctx, &t);
    }
    if st.library_open {
        fresco::brush_library(app, &mut st, &ctx, &t);
    }
    if st.layer_props_open {
        fresco::layer_props(app, &mut st, &ctx, &t);
    }
    if st.layer_more_open {
        fresco::layer_more(app, &mut st, &ctx, &t);
    }
    if st.precision_open {
        fresco::precision_popover(app, &mut st, &ctx, &t);
    }
    action_bar(app, &ctx, &t);
    if st.gallery.is_some() {
        gallery_sheet(app, &mut st, &ctx, &t);
    }
    show_notice(&mut st, ui.ctx(), &t);
    // One more frame after any input. A tap's effect (the selected layer, a colour, a tool) is
    // applied while this frame draws, after the widgets that show it; with a mouse the next
    // pointer move redraws at once, but a touch screen sends nothing more, so without this the
    // change would show only on the next touch and the app would seem stuck.
    if ui.ctx().input(|i| !i.events.is_empty()) {
        ui.ctx().request_repaint();
    }
    app.touch = Some(st);
}

/// A tap on the image outside the open popovers closes them (as on iPad and Android). The tap
/// still does what it does there.
fn dismiss_on_outside_tap(st: &mut TouchUi, ctx: &egui::Context) {
    let any = st.home_open || st.share_open || st.settings_open || st.help_open || st.color_open || st.brush_open || st.size_open || st.smooth_open || st.library_open || st.layer_props_open || st.layer_more_open || st.precision_open;
    if !any {
        return;
    }
    let Some(pos) = ctx.input(|i| if i.pointer.any_pressed() { i.pointer.interact_pos() } else { None }) else { return };
    // Only on the image: the bar and rail buttons toggle their own popovers.
    let on_canvas = CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner).is_some_and(|r| r.contains(pos));
    if on_canvas && ctx.layer_id_at(pos).is_none_or(|l| l.order == egui::Order::Background) {
        st.opacity_drag = None;
        fresco::close_popovers(st);
    }
}

/// A new white canvas the size of the screen, in device pixels.
fn new_canvas(app: &mut PhotocraftApp, ctx: &egui::Context) {
    let px = ctx.content_rect().size() * ctx.pixels_per_point();
    let (w, h) = (px.x.round().clamp(512.0, 8192.0), px.y.round().clamp(512.0, 8192.0));
    run(app, "file.new", json!({ "width": w, "height": h, "background": "white", "name": "Lienzo" }));
    // Draw on a layer above the white Background, as in Procreate: erasing then shows the white,
    // and the layer's opacity can change (the Background has none; changing it would turn it
    // into a normal layer and the eraser would cut transparent holes).
    run(app, "layer.new.layer", json!({ "name": "Capa 1" }));
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

/// The rail's Lápiz is the Brush tool with a graphite brush of its own (the Pencil tool paints
/// aliased, hard pixels, which on a tablet look just like the Brush); Pincel is the Brush tool with
/// its own brush.
fn rail_selected(app: &PhotocraftApp, st: &TouchUi, entry: Tool) -> bool {
    match entry {
        Tool::Pencil => app.ui.tool == Tool::Brush && st.pencil_mode,
        Tool::Brush => app.ui.tool == Tool::Brush && !st.pencil_mode,
        other => app.ui.tool == other,
    }
}

/// The name the active tool shows (Lápiz for the pencil mode of the Brush).
fn active_tool_name(app: &PhotocraftApp, st: &TouchUi) -> &'static str {
    if app.ui.tool == Tool::Brush && st.pencil_mode { "Lápiz" } else { fresco::variant_name(app.ui.tool).unwrap_or_else(|| tool_name(app.ui.tool)) }
}

/// Put `b` in as the live brush, keeping the session's colours and erase flag.
fn set_live_brush(app: &mut PhotocraftApp, mut b: photocraft_engine::paint::brush::BrushSettings) {
    let cur = &app.session.tools.brush;
    b.color = cur.color;
    b.background = cur.background;
    b.color_cmyk = cur.color_cmyk;
    b.erase = cur.erase;
    app.session.tools.brush = b;
    app.session.tools.brush_changed();
}

fn pick_rail_tool(app: &mut PhotocraftApp, st: &mut TouchUi, entry: Tool) {
    let want_pencil = entry == Tool::Pencil;
    let target = if want_pencil { Tool::Brush } else { entry };
    let in_pencil = st.pencil_mode && app.ui.tool == Tool::Brush;
    // Leaving the pencil: keep its brush and bring the Brush's own back first, so the tool switch
    // saves the Brush's (not the pencil's) settings.
    if in_pencil && !want_pencil {
        st.pencil_brush = Some(app.session.tools.brush.clone());
        if let Some(b) = st.brush_brush.take() {
            set_live_brush(app, b);
        }
        st.pencil_mode = false;
    }
    if app.ui.tool != target {
        crate::tool_memory::select(app, target);
        crate::paint_mouse::sync_tool_brush(app);
    }
    if want_pencil && !in_pencil {
        st.brush_brush = Some(app.session.tools.brush.clone());
        match st.pencil_brush.clone() {
            Some(b) => set_live_brush(app, b),
            // Graphite on paper: textured, pressure sets the width and the darkness.
            None => run(app, "tools.setBrush", json!({ "preset": "Soft Pencil" })),
        }
        st.pencil_mode = true;
    }
    // The first time, the Eraser takes the brush it had last (whatever was painting before): a
    // soft or pressure-opacity brush then only fades strokes. Start it as a real eraser.
    if entry == Tool::Eraser && !st.eraser_ready {
        st.eraser_ready = true;
        run(app, "tools.setBrush", json!({ "preset": "Hard Round" }));
        run(
            app,
            "tools.setBrush",
            json!({ "brush": { "size": 40.0, "opacity": 1.0, "flow": 1.0, "hardness": 1.0, "pressureOpacity": false, "pressureSize": false, "spacing": 0.15, "smoothing": { "amount": 0.0 } } }),
        );
    }
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
    let anchor = st.brush_anchor.unwrap_or(pos2(RAIL + 10.0, BAR_TOP + 10.0));
    let lo = screen.top() + BAR_TOP + 6.0;
    let top = anchor.y.clamp(lo, (screen.bottom() - safe_bottom() - height - 8.0).max(lo));
    let mut close = false;
    egui::Area::new(egui::Id::new("touch-brush")).order(egui::Order::Foreground).fixed_pos(pos2(anchor.x, top)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(popover_room(ui)).min_scrolled_height(popover_room(ui)).show(ui, |ui| {
            let w = BRUSH_POP_W - 32.0;
            ui.set_width(w);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(active_tool_name(app, st)).size(20.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button_with_icon_size(ui, "x", 40.0, 20.0, false, "Cerrar").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(6.0);
            variant_chips(app, st, ui, t);
            match tool {
                Tool::Brush | Tool::Pencil | Tool::Eraser | Tool::Smudge | Tool::MixerBrush | Tool::Blur => brush_options(app, st, ui, w, t),
                Tool::Move => move_options(app, st, ui, t),
                Tool::Rectangle | Tool::EllipseShape | Tool::Line => hint(ui, "Arrastra sobre el lienzo para dibujar la forma con el color actual.", t),
                Tool::Type => text_options(app, st, ui, t),
                Tool::Patch => patch_options(app, ui, t),
                Tool::Crop => crop_options(app, ui, t),
                Tool::Artboard => artboard_options(app, st, ui, t),
                other => hint(ui, tool_hint(other), t),
            }
        });
        });
    });
    if close {
        st.brush_open = false;
    }
}

/// The tool's group variants as chips (Lazo · Rectangular · Varita…), when it has several.
fn variant_chips(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let Some(g) = fresco_group(app.ui.tool) else { return };
    if g.1.len() < 2 {
        return;
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for &(tool, name) in g.1 {
            if chip(ui, name, app.ui.tool == tool, t).clicked() && app.ui.tool != tool {
                pick_rail_tool(app, st, tool);
                st.variants.insert(g.0, tool);
            }
        }
    });
    ui.add_space(8.0);
}

fn fresco_group(tool: Tool) -> Option<(&'static str, &'static [(Tool, &'static str)])> {
    const G: &[(&str, &[(Tool, &str)])] = &[
        ("move", &[(Tool::Move, "Mover y transformar"), (Tool::Crop, "Recortar")]),
        ("select", &[(Tool::Lasso, "Lazo"), (Tool::RectMarquee, "Rectangular"), (Tool::MagicWand, "Varita mágica"), (Tool::QuickSelection, "Selección rápida")]),
        ("fill", &[(Tool::PaintBucket, "Relleno"), (Tool::Gradient, "Degradado")]),
        ("smudge", &[(Tool::Smudge, "Difuminar"), (Tool::Blur, "Desenfocar"), (Tool::Patch, "Parche")]),
        ("shapes", &[(Tool::Rectangle, "Rectángulo"), (Tool::EllipseShape, "Elipse"), (Tool::Line, "Línea"), (Tool::Artboard, "Mesa de trabajo")]),
    ];
    G.iter().find(|(_, v)| v.iter().any(|(t, _)| *t == tool)).copied()
}

/// Move: drag to move the layer (or selection); free transform, flips and quarter turns.
fn move_options(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    hint(ui, "Arrastra para mover la capa activa (o la selección). Con Transformación libre, arrastra las esquinas para escalar y fuera de ellas para rotar; luego Aplicar.", t);
    let ctx = ui.ctx().clone();
    ui.spacing_mut().item_spacing.y = 8.0;
    if big_button(ui, "Transformación libre", t).clicked() {
        invoke(app, &ctx, "edit.freeTransform");
        st.brush_open = false;
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
            notify(st, &ctx, &e);
        }
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
            // Big brushes space their dabs further apart: each dab of a 500 px brush covers
            // 250 000 pixels, and at 10 % spacing a quick stroke paints dozens per frame.
            let spacing = if s > 150.0 { 0.2 } else if s > 60.0 { 0.15 } else { b.spacing };
            let brush = if spacing > b.spacing { json!({ "size": s, "spacing": spacing }) } else { json!({ "size": s }) };
            run(app, "tools.setBrush", json!({ "brush": brush, "coalesce": "touch-size" }));
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
    // The touch shortcut is read from the raw touches (see the gestures), not as a tap, so the
    // pen can keep drawing while a finger holds it.
    if fresco::SHORTCUT_RECT.lock().unwrap_or_else(PoisonError::into_inner).is_some_and(|r| r.contains(pos)) {
        return false;
    }
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
    let size = vec2(300.0, 400.0);
    // Below the colour well, right-aligned with it.
    let anchor = st.color_anchor.unwrap_or(pos2(screen.right() - 80.0, BAR_H + 6.0));
    let left = if anchor.x < screen.center().x { anchor.x } else { (anchor.x - size.x).max(screen.left() + 8.0) };
    let anchor = pos2(anchor.x, anchor.y.clamp(screen.top() + BAR_TOP + 6.0, (screen.bottom() - safe_bottom() - size.y - 8.0).max(screen.top() + BAR_TOP + 6.0)));
    let mut commit: Option<Color32> = None;
    egui::Area::new(egui::Id::new("touch-color")).order(egui::Order::Foreground).fixed_pos(pos2(left, anchor.y)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            egui::ScrollArea::vertical().max_height(popover_room(ui)).min_scrolled_height(popover_room(ui)).show(ui, |ui| {
            let w = size.x - 32.0;
            ui.set_width(w);
            let mut hsv = st.picking.unwrap_or(st.hsv);
            let mut dragging = false;

            // Saturation (x) / brightness (y) square in the current hue.
            let (sq, resp) = ui.allocate_exact_size(vec2(w, 165.0), Sense::click_and_drag());
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
            ui.add_space(10.0);

            // Hue bar.
            let (bar, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click_and_drag());
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
            ui.add_space(10.0);

            // Old | new.
            let (pr, _) = ui.allocate_exact_size(vec2(w, 32.0), Sense::hover());
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
            ui.add_space(8.0);

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
                ui.label(egui::RichText::new(label).size(13.0).color(t.text_dim));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    for &col in colors {
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
                        ui.painter().rect_filled(r, CornerRadius::same(16), col);
                        ui.painter().rect_stroke(r, CornerRadius::same(16), Stroke::new(1.0, t.card_border), StrokeKind::Inside);
                        if resp.clicked() {
                            commit = Some(col);
                        }
                    }
                });
                ui.add_space(4.0);
            };
            let recent = st.recent.clone();
            swatch_row(ui, &recent, "Recientes");
            let swatches: Vec<Color32> = SWATCHES.iter().map(|s| Color32::from_rgb(s[0], s[1], s[2])).collect();
            swatch_row(ui, &swatches, "Muestras");
        });
        });
    });
    if let Some(c) = commit {
        set_fg(app, st, c);
        st.hsv = color_to_hsv(c, st.hsv);
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
    let rect = screen.shrink2(vec2(screen.width() * 0.08, 0.0)).with_min_y(screen.top() + BAR_H + 12.0).with_max_y(screen.bottom() - safe_bottom());
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
        out.push(Row { id: l.id.0, name: l.name.clone(), visible: l.visible, group, artboard });
        if let Some(children) = l.children() {
            rows(children, depth + 1, out);
        }
    }
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
    /// The finger holding the touch shortcut.
    shortcut_finger: Option<u64>,
    /// The pen's last resting point, since when it has rested there, and when the stroke began.
    pen_rest: Option<(Pos2, std::time::Instant, std::time::Instant)>,
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
                TouchPhase::Start => {
                    if !g.pens.contains(&e.id) {
                        g.pens.push(e.id);
                    }
                    g.pen_rest = Some((e.pos, at, at));
                }
                TouchPhase::Move => {
                    if !g.pens.contains(&e.id) {
                        g.pens.push(e.id);
                    }
                    // Where the pen last moved to, and when; a few points of jitter count as still.
                    match g.pen_rest {
                        Some((p, since, began)) if (e.pos - p).length() < 4.0 => g.pen_rest = Some((p, since, began)),
                        Some((_, _, began)) => g.pen_rest = Some((e.pos, at, began)),
                        None => g.pen_rest = Some((e.pos, at, at)),
                    }
                }
                TouchPhase::End => {
                    g.pens.retain(|p| *p != e.id);
                    if let Some((_, since, began)) = g.pen_rest.take()
                        && at.duration_since(began).as_secs_f32() > 0.25
                    {
                        request_shape(app, at.duration_since(since).as_secs_f32());
                    }
                }
                TouchPhase::Cancel => {
                    g.pens.retain(|p| *p != e.id);
                    g.pen_rest = None;
                }
            }
            continue;
        }
        // Fingers the app receives itself (on the bars, panels, popovers) are not gestures.
        if !e.dropped {
            continue;
        }
        match e.phase {
            TouchPhase::Start if fresco::SHORTCUT_RECT.lock().unwrap_or_else(PoisonError::into_inner).is_some_and(|r| r.contains(e.pos)) => {
                g.shortcut_finger = Some(e.id);
                fresco::SHORTCUT_HELD.store(true, Ordering::Relaxed);
            }
            TouchPhase::End | TouchPhase::Cancel if g.shortcut_finger == Some(e.id) => {
                g.shortcut_finger = None;
                fresco::SHORTCUT_HELD.store(false, Ordering::Relaxed);
            }
            _ if g.shortcut_finger == Some(e.id) => {}
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
    let mut rotation = crate::rotate_view::wrap_deg(view.rotation + if flip { -turn } else { turn });
    // Settings › Imán de rotación: within 4° of a multiple of the step, snap to it.
    let step = f32::from_bits(fresco::ROT_SNAP.load(Ordering::Relaxed));
    if step > 0.0 {
        let near = (rotation / step).round() * step;
        if (rotation - near).abs() < 4.0 {
            rotation = crate::rotate_view::wrap_deg(near);
        }
    }
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

// ---------------------------------------------------------------------------------------------
// Drag a selection: with a selection tool, a press inside the selection moves what is selected
// (the Move tool, just for that drag), as on iPad apps; a press outside starts a new selection.
// ---------------------------------------------------------------------------------------------

const SELECTION_TOOLS: &[Tool] =
    &[Tool::Lasso, Tool::PolygonLasso, Tool::MagneticLasso, Tool::RectMarquee, Tool::EllipseMarquee, Tool::QuickSelection, Tool::MagicWand, Tool::ObjectSelection];

/// The document point under a screen point of the canvas (points), for the active view.
fn doc_point(app: &PhotocraftApp, ctx: &egui::Context, pos: Pos2) -> Option<[f64; 2]> {
    let i = app.session.active_index()?;
    let rect = (*CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner))?;
    let view = app.ui.views.get(i)?;
    let ppp = ctx.pixels_per_point().max(0.1);
    let xf = crate::canvas::ViewXform { rect, zoom: view.zoom / ppp, center: view.center, flip: app.ui.view.flip_horizontal, rotation: view.rotation, aspect: 1.0 };
    Some(xf.to_doc(pos))
}

fn drag_selection(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context) {
    let (pressed, down, released, pos) = ctx.input(|i| (i.pointer.any_pressed(), i.pointer.any_down(), i.pointer.any_released(), i.pointer.interact_pos()));
    if let Some(prev) = st.spring_back {
        // Back to the selection tool once the drag has ended (not on the release frame: the Move
        // tool still has to finish the move then).
        if !down && !released && !pressed {
            st.spring_back = None;
            crate::tool_memory::select(app, prev);
        }
        return;
    }
    if !pressed || !SELECTION_TOOLS.contains(&app.ui.tool) {
        return;
    }
    let Some(pos) = pos else { return };
    let on_canvas = CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner).is_some_and(|r| r.contains(pos));
    if !on_canvas || ctx.layer_id_at(pos).is_some_and(|l| l.order != egui::Order::Background) {
        return;
    }
    let Some([x, y]) = doc_point(app, ctx, pos) else { return };
    let inside = app.session.active().and_then(|d| d.doc.selection.as_ref()).is_some_and(|sel| sel.sample_channel(x.floor() as i32, y.floor() as i32, 0) > 0.5);
    if inside {
        st.spring_back = Some(app.ui.tool);
        crate::tool_memory::select(app, Tool::Move);
    }
}

// ---------------------------------------------------------------------------------------------
// Shape snap: end a stroke by holding the pen still for a moment and a hand-drawn circle,
// ellipse or line is redrawn as a clean one with the same brush (Procreate's QuickShape).
// ---------------------------------------------------------------------------------------------

/// How long the pen must rest at the end of a stroke to ask for a shape.
const HOLD_SECS: f32 = 0.45;

/// A pen stroke ended with a hold: the journal length when it was asked, and when.
static SHAPE_REQUEST: Mutex<Option<(usize, std::time::Instant)>> = Mutex::new(None);

/// Called by the gestures when a pen lifts after resting `held` seconds at the end of a stroke.
fn request_shape(app: &PhotocraftApp, held: f32) {
    if held >= HOLD_SECS && fresco::SHAPE_SNAP.load(Ordering::Relaxed) {
        *SHAPE_REQUEST.lock().unwrap_or_else(PoisonError::into_inner) = Some((app.session.journal.len(), std::time::Instant::now()));
    }
}

/// Once the held stroke has been committed, replace it with the shape it was drawn as.
fn snap_shape(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context) {
    let Some((len, at)) = *SHAPE_REQUEST.lock().unwrap_or_else(PoisonError::into_inner) else { return };
    if at.elapsed().as_secs_f32() > 1.5 {
        *SHAPE_REQUEST.lock().unwrap_or_else(PoisonError::into_inner) = None;
        return;
    }
    let Some((id, params)) = app.session.journal.last().cloned() else { return };
    if app.session.journal.len() <= len {
        ctx.request_repaint();
        return; // not committed yet
    }
    *SHAPE_REQUEST.lock().unwrap_or_else(PoisonError::into_inner) = None;
    if !matches!(id.as_str(), "paint.stroke" | "paint.pencil") {
        return;
    }
    let Some(raw) = params.get("points").and_then(|v| v.as_array()) else { return };
    let pts: Vec<[f64; 3]> = raw
        .iter()
        .filter_map(|p| {
            let a = p.as_array()?;
            Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?, a.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0)])
        })
        .collect();
    let Some((name, shape)) = fit_shape(&pts) else { return };
    // Keep the stroke's typical pressure along the clean shape.
    let mut pressures: Vec<f64> = pts.iter().map(|p| p[2]).collect();
    pressures.sort_by(f64::total_cmp);
    let pressure = pressures.get(pressures.len() / 2).copied().unwrap_or(1.0);
    let mut new_params = params.clone();
    new_params["points"] = json!(shape.iter().map(|p| [p[0], p[1], pressure]).collect::<Vec<_>>());
    // Keep the corners sharp: no smoothing on the clean shape.
    if let Some(b) = new_params.get_mut("brush").filter(|b| b.is_object()) {
        b["smoothing"] = json!({ "amount": 0.0 });
    }
    if app.run("history.undo", json!({})).is_err() {
        return;
    }
    match app.run(&id, new_params) {
        Ok(_) => notify(st, ctx, &format!("Forma ajustada: {name}")),
        Err(e) => {
            let _ = app.run("history.redo", json!({}));
            notify(st, ctx, &e);
        }
    }
}

/// Ramer–Douglas–Peucker: the corners of an open path, within `eps` of every point.
fn rdp(p: &[[f64; 2]], eps: f64) -> Vec<[f64; 2]> {
    if p.len() < 3 {
        return p.to_vec();
    }
    let (a, b) = (p[0], p[p.len() - 1]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = (dx * dx + dy * dy).sqrt();
    let dist = |q: &[f64; 2]| if len < 1e-9 { ((q[0] - a[0]).powi(2) + (q[1] - a[1]).powi(2)).sqrt() } else { ((q[0] - a[0]) * dy - (q[1] - a[1]) * dx).abs() / len };
    let (mut far, mut idx) = (0.0, 0);
    for (i, q) in p.iter().enumerate().take(p.len() - 1).skip(1) {
        let d = dist(q);
        if d > far {
            far = d;
            idx = i;
        }
    }
    if far <= eps {
        return vec![a, b];
    }
    let mut left = rdp(&p[..=idx], eps);
    let right = rdp(&p[idx..], eps);
    left.pop();
    left.extend(right);
    left
}

fn dist2(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Distance from `q` to the segment `a`–`b`.
fn seg_dist(q: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1e-12 { 0.0 } else { (((q[0] - a[0]) * dx + (q[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) };
    dist2(q, [a[0] + dx * t, a[1] + dy * t])
}

/// The interior angle (degrees) at `b` between `a` and `c`.
fn corner_deg(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let (u, v) = ([a[0] - b[0], a[1] - b[1]], [c[0] - b[0], c[1] - b[1]]);
    let d = (u[0] * v[0] + u[1] * v[1]) / ((u[0].hypot(u[1]) * v[0].hypot(v[1])).max(1e-9));
    d.clamp(-1.0, 1.0).acos().to_degrees()
}

/// Drop corners that are nearly straight or too close to the previous one (hand jitter).
fn clean_corners(mut v: Vec<[f64; 2]>, closed: bool, min_gap: f64) -> Vec<[f64; 2]> {
    loop {
        let n = v.len();
        if n < 3 {
            return v;
        }
        let mut removed = false;
        for i in 0..n {
            if !closed && (i == 0 || i == n - 1) {
                continue;
            }
            let (a, b, c) = (v[(i + n - 1) % n], v[i], v[(i + 1) % n]);
            if corner_deg(a, b, c) > 160.0 || dist2(a, b) < min_gap {
                v.remove(i);
                removed = true;
                break;
            }
        }
        if !removed {
            return v;
        }
    }
}

/// Points along the polyline through `v` (closed: back to the first), every ~4 px, so the brush
/// follows straight edges and keeps the corners sharp.
fn densify(v: &[[f64; 2]], closed: bool) -> Vec<[f64; 2]> {
    let mut out = Vec::new();
    let n = v.len();
    let segs = if closed { n } else { n.saturating_sub(1) };
    for i in 0..segs {
        let (a, b) = (v[i], v[(i + 1) % n]);
        let steps = (dist2(a, b) / 4.0).ceil().max(1.0) as usize;
        for k in 0..steps {
            let f = k as f64 / steps as f64;
            out.push([a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]);
        }
    }
    out.push(if closed { v[0] } else { v[n - 1] });
    out
}

/// A closed hand-drawn polygon (3 to 6 corners) or an open polyline (2 to 4 segments): its clean
/// corners. A quadrilateral with near-right angles becomes an exact rectangle (or square).
fn fit_polygon(p: &[[f64; 2]], closed: bool, size: f64) -> Option<(&'static str, Vec<[f64; 2]>)> {
    let eps = 0.06 * size;
    let corners = if closed {
        // Split the loop at the point farthest from the start, simplify both halves.
        let far = (0..p.len()).max_by(|&i, &j| dist2(p[0], p[i]).total_cmp(&dist2(p[0], p[j])))?;
        let mut a = rdp(&p[..=far], eps);
        let mut back: Vec<[f64; 2]> = p[far..].to_vec();
        back.push(p[0]);
        let b = rdp(&back, eps);
        a.pop();
        a.extend(b);
        a.pop(); // the start again
        a
    } else {
        rdp(p, eps)
    };
    let v = clean_corners(corners, closed, 0.08 * size);
    let n = v.len();
    let ok = if closed { (3..=6).contains(&n) } else { (3..=5).contains(&n) };
    if !ok {
        return None;
    }
    // Every drawn point near the polygon.
    let segs = if closed { n } else { n - 1 };
    let err = p.iter().map(|&q| (0..segs).map(|i| seg_dist(q, v[i], v[(i + 1) % n])).fold(f64::MAX, f64::min)).sum::<f64>() / p.len() as f64;
    if err > 0.035 * size {
        return None;
    }
    if !closed {
        return Some(("línea quebrada", densify(&v, false)));
    }
    if n == 4 && (0..4).all(|i| (corner_deg(v[(i + 3) % 4], v[i], v[(i + 1) % 4]) - 90.0).abs() < 18.0) {
        // An exact rectangle on the axes of its longest side.
        let (i, _) = (0..4).map(|i| (i, dist2(v[i], v[(i + 1) % 4]))).max_by(|a, b| a.1.total_cmp(&b.1))?;
        let ang = (v[(i + 1) % 4][1] - v[i][1]).atan2(v[(i + 1) % 4][0] - v[i][0]);
        let (c, s) = (ang.cos(), ang.sin());
        let (mut umin, mut umax, mut wmin, mut wmax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for q in &v {
            let (u, w) = (q[0] * c + q[1] * s, -q[0] * s + q[1] * c);
            umin = umin.min(u);
            umax = umax.max(u);
            wmin = wmin.min(w);
            wmax = wmax.max(w);
        }
        let (mut hw, mut hh) = ((umax - umin) / 2.0, (wmax - wmin) / 2.0);
        let (cu, cw) = ((umax + umin) / 2.0, (wmax + wmin) / 2.0);
        let name = if (hw - hh).abs() / hw.max(hh) < 0.12 {
            let m = (hw + hh) / 2.0;
            hw = m;
            hh = m;
            "cuadrado"
        } else {
            "rectángulo"
        };
        let to_xy = |u: f64, w: f64| [u * c - w * s, u * s + w * c];
        let rect = vec![to_xy(cu - hw, cw - hh), to_xy(cu + hw, cw - hh), to_xy(cu + hw, cw + hh), to_xy(cu - hw, cw + hh)];
        // Start at the corner nearest where the stroke began.
        let k = (0..4).min_by(|&a, &b| dist2(rect[a], p[0]).total_cmp(&dist2(rect[b], p[0])))?;
        let ordered: Vec<[f64; 2]> = (0..4).map(|j| rect[(k + j) % 4]).collect();
        return Some((name, densify(&ordered, true)));
    }
    let name = match n {
        3 => "triángulo",
        4 => "cuadrilátero",
        5 => "pentágono",
        _ => "hexágono",
    };
    Some((name, densify(&v, true)))
}

/// The clean shape a hand-drawn stroke is closest to: a line, a polyline, a polygon (triangle,
/// rectangle, square…), a circle or an ellipse (any angle), as points to stroke; `None` when it is
/// none of them.
fn fit_shape(pts: &[[f64; 3]]) -> Option<(&'static str, Vec<[f64; 2]>)> {
    if pts.len() < 8 {
        return None;
    }
    let p: Vec<[f64; 2]> = pts.iter().map(|q| [q[0], q[1]]).collect();
    let length: f64 = p.windows(2).map(|w| ((w[1][0] - w[0][0]).powi(2) + (w[1][1] - w[0][1]).powi(2)).sqrt()).sum();
    let first = p[0];
    let last = p[p.len() - 1];
    let chord = ((last[0] - first[0]).powi(2) + (last[1] - first[1]).powi(2)).sqrt();
    if length < 20.0 {
        return None;
    }
    // A line: every point close to the segment between the ends.
    if chord > 0.85 * length {
        let (dx, dy) = ((last[0] - first[0]) / chord, (last[1] - first[1]) / chord);
        let dev = p.iter().map(|q| ((q[0] - first[0]) * dy - (q[1] - first[1]) * dx).abs()).fold(0.0, f64::max);
        if dev < (0.04 * chord).max(3.0) {
            let n = 48;
            return Some(("línea", (0..=n).map(|k| {
                let f = k as f64 / n as f64;
                [first[0] + (last[0] - first[0]) * f, first[1] + (last[1] - first[1]) * f]
            }).collect()));
        }
    }
    // A closed shape: the ends meet (within a fifth of the size).
    let (mut minx, mut miny, mut maxx, mut maxy) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for q in &p {
        minx = minx.min(q[0]);
        miny = miny.min(q[1]);
        maxx = maxx.max(q[0]);
        maxy = maxy.max(q[1]);
    }
    let size = (maxx - minx).max(maxy - miny);
    if size < 12.0 {
        return None;
    }
    let closed = chord <= 0.25 * size;
    // Corners first: a polygon or polyline (a circle simplifies to 7+ corners and falls through).
    if let Some(found) = fit_polygon(&p, closed, size) {
        return Some(found);
    }
    if !closed {
        return None;
    }
    // Principal axes of the points: the ellipse's angle; its radii from the extents along them.
    let n = p.len() as f64;
    let (mx, my) = (p.iter().map(|q| q[0]).sum::<f64>() / n, p.iter().map(|q| q[1]).sum::<f64>() / n);
    let (mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0);
    for q in &p {
        let (dx, dy) = (q[0] - mx, q[1] - my);
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    let angle = 0.5 * (2.0 * sxy).atan2(sxx - syy);
    let (c, s) = (angle.cos(), angle.sin());
    let (mut umin, mut umax, mut vmin, mut vmax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for q in &p {
        let (u, v) = ((q[0] - mx) * c + (q[1] - my) * s, -(q[0] - mx) * s + (q[1] - my) * c);
        umin = umin.min(u);
        umax = umax.max(u);
        vmin = vmin.min(v);
        vmax = vmax.max(v);
    }
    let (a, b) = ((umax - umin) / 2.0, (vmax - vmin) / 2.0);
    if a < 4.0 || b < 4.0 {
        return None;
    }
    let (cu, cv) = ((umax + umin) / 2.0, (vmax + vmin) / 2.0);
    let (cx, cy) = (mx + cu * c - cv * s, my + cu * s + cv * c);
    // How far the points stray from that ellipse (0 = on it).
    let err = p
        .iter()
        .map(|q| {
            let (u, v) = ((q[0] - cx) * c + (q[1] - cy) * s, -(q[0] - cx) * s + (q[1] - cy) * c);
            ((u / a).powi(2) + (v / b).powi(2)).sqrt() - 1.0
        })
        .map(f64::abs)
        .sum::<f64>()
        / n;
    if err > 0.12 {
        return None;
    }
    // Nearly round: a circle.
    let (name, a, b) = if (a / b).max(b / a) < 1.15 {
        let r = (a + b) / 2.0;
        ("círculo", r, r)
    } else {
        ("elipse", a, b)
    };
    let steps = 96;
    let start = (first[1] - cy).atan2(first[0] - cx);
    Some((
        name,
        (0..=steps)
            .map(|k| {
                let t = start + std::f64::consts::TAU * k as f64 / steps as f64;
                let (u, v) = (a * t.cos(), b * t.sin());
                [cx + u * c - v * s, cy + u * s + v * c]
            })
            .collect(),
    ))
}

#[cfg(test)]
mod shape_tests {
    use super::fit_shape;

    fn wobble(k: usize) -> f64 {
        ((k * 7919) % 13) as f64 / 13.0 - 0.5
    }

    #[test]
    fn a_hand_drawn_circle_becomes_a_circle() {
        let pts: Vec<[f64; 3]> = (0..=120)
            .map(|k| {
                let t = std::f64::consts::TAU * k as f64 / 120.0 * 0.97;
                let r = 200.0 + 6.0 * wobble(k);
                [500.0 + r * t.cos(), 400.0 + r * t.sin(), 0.6]
            })
            .collect();
        let (name, shape) = fit_shape(&pts).expect("a circle");
        assert_eq!(name, "círculo");
        for p in &shape {
            let r = ((p[0] - 500.0).powi(2) + (p[1] - 400.0).powi(2)).sqrt();
            assert!((r - 200.0).abs() < 8.0, "radius {r}");
        }
    }

    #[test]
    fn a_tilted_ellipse_stays_an_ellipse() {
        let (a, b, ang) = (260.0f64, 120.0f64, 0.6f64);
        let pts: Vec<[f64; 3]> = (0..=150)
            .map(|k| {
                let t = std::f64::consts::TAU * k as f64 / 150.0 * 0.98;
                let (u, v) = (a * t.cos(), b * t.sin());
                [300.0 + u * ang.cos() - v * ang.sin(), 300.0 + u * ang.sin() + v * ang.cos(), 1.0]
            })
            .collect();
        assert_eq!(fit_shape(&pts).map(|s| s.0), Some("elipse"));
    }

    #[test]
    fn a_shaky_line_becomes_straight() {
        let pts: Vec<[f64; 3]> = (0..=60).map(|k| [100.0 + k as f64 * 10.0, 200.0 + k as f64 * 3.0 + 4.0 * wobble(k), 1.0]).collect();
        let (name, shape) = fit_shape(&pts).expect("a line");
        assert_eq!(name, "línea");
        assert_eq!(shape.first().copied(), Some([100.0, 200.0 + 4.0 * wobble(0)]));
    }

    fn loop_through(corners: &[[f64; 2]], per_side: usize) -> Vec<[f64; 3]> {
        let mut out = Vec::new();
        for i in 0..corners.len() {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            for k in 0..per_side {
                let f = k as f64 / per_side as f64;
                out.push([a[0] + (b[0] - a[0]) * f + 3.0 * wobble(out.len()), a[1] + (b[1] - a[1]) * f + 3.0 * wobble(out.len() + 5), 1.0]);
            }
        }
        out.push([corners[0][0] + 6.0, corners[0][1] + 4.0, 1.0]);
        out
    }

    #[test]
    fn a_hand_drawn_square_becomes_a_square() {
        let pts = loop_through(&[[100.0, 100.0], [400.0, 104.0], [398.0, 402.0], [102.0, 398.0]], 30);
        let (name, shape) = fit_shape(&pts).expect("a square");
        assert_eq!(name, "cuadrado");
        assert!(shape.len() > 100, "densified edges");
    }

    #[test]
    fn a_hand_drawn_rectangle_and_triangle() {
        let rect = loop_through(&[[100.0, 100.0], [600.0, 100.0], [600.0, 300.0], [100.0, 300.0]], 30);
        assert_eq!(fit_shape(&rect).map(|s| s.0), Some("rectángulo"));
        let tri = loop_through(&[[300.0, 100.0], [500.0, 420.0], [100.0, 420.0]], 40);
        assert_eq!(fit_shape(&tri).map(|s| s.0), Some("triángulo"));
    }

    #[test]
    fn an_open_v_becomes_a_polyline() {
        let mut v: Vec<[f64; 3]> = (0..=40).map(|k| [100.0 + k as f64 * 5.0, 100.0 + k as f64 * 8.0 + 2.0 * wobble(k), 1.0]).collect();
        v.extend((1..=40).map(|k| [300.0 + k as f64 * 5.0, 420.0 - k as f64 * 8.0 + 2.0 * wobble(k + 3), 1.0]));
        assert_eq!(fit_shape(&v).map(|s| s.0), Some("línea quebrada"));
    }

    #[test]
    fn a_scribble_is_left_alone() {
        let pts: Vec<[f64; 3]> = (0..=80).map(|k| [100.0 + (k % 10) as f64 * 30.0, 100.0 + (k / 10) as f64 * 25.0 + ((k * 37) % 50) as f64, 1.0]).collect();
        assert!(fit_shape(&pts).is_none());
    }
}

#[cfg(test)]
mod touch_tests {
    use super::*;
    use crate::{PhotocraftApp, Services};
    use photocraft_engine::Session;

    fn app() -> (PhotocraftApp, TouchUi, egui::Context) {
        let ctx = egui::Context::default();
        let mut app = PhotocraftApp::new(Session::new(), Services::default());
        new_canvas(&mut app, &ctx);
        (app, TouchUi::new(), ctx)
    }

    fn pixel(app: &mut PhotocraftApp, x: u32, y: u32) -> Vec<f32> {
        serde_json::from_value(app.run("document.pixel", json!({ "x": x, "y": y })).expect("pixel")).expect("rgba")
    }

    #[test]
    fn a_new_canvas_draws_on_a_layer_above_a_white_background() {
        let (app, _, _) = app();
        let d = app.session.active().expect("doc");
        assert_eq!(d.doc.layers.len(), 2, "Background + Capa 1");
        let active = d.active_layer.expect("active");
        assert!(!d.doc.is_background(active), "drawing starts on Capa 1, not the Background");
        assert_eq!(d.doc.layer(active).map(|l| l.name.as_str()), Some("Capa 1"));
    }

    #[test]
    fn the_eraser_starts_as_a_real_eraser() {
        let (mut app, mut st, _) = app();
        run(&mut app, "tools.setBrush", json!({ "preset": "Soft Round Pressure Opacity" }));
        pick_rail_tool(&mut app, &mut st, Tool::Eraser);
        let b = &app.session.tools.brush;
        assert_eq!(app.ui.tool, Tool::Eraser);
        assert_eq!((b.opacity, b.flow, b.hardness), (1.0, 1.0, 1.0));
        assert!(!b.pressure_opacity, "light pressure must not only fade strokes");
        assert_eq!(b.smoothing.amount, 0.0);
    }

    #[test]
    fn erasing_on_the_drawing_layer_shows_the_white_background() {
        let (mut app, _, _) = app();
        run(&mut app, "tools.setColors", json!({ "foreground": "#000000" }));
        run(&mut app, "paint.stroke", json!({ "points": [[100, 100], [300, 100]], "size": 40 }));
        assert!(pixel(&mut app, 200, 100)[0] < 0.1, "the stroke is black");
        run(&mut app, "paint.stroke", json!({ "points": [[100, 100], [300, 100]], "size": 80, "erase": true }));
        let px = pixel(&mut app, 200, 100);
        assert!(px.iter().take(3).all(|v| *v > 0.99), "erased to the white Background: {px:?}");
        assert!(px.get(3).is_none_or(|a| *a > 0.99), "no transparent hole: {px:?}");
    }

    #[test]
    fn pencil_and_brush_keep_their_own_settings() {
        let (mut app, mut st, _) = app();
        pick_rail_tool(&mut app, &mut st, Tool::Pencil);
        assert!(st.pencil_mode && app.ui.tool == Tool::Brush);
        run(&mut app, "tools.setBrush", json!({ "brush": { "size": 7.0 } }));
        pick_rail_tool(&mut app, &mut st, Tool::Brush);
        assert!(!st.pencil_mode);
        run(&mut app, "tools.setBrush", json!({ "brush": { "size": 55.0 } }));
        pick_rail_tool(&mut app, &mut st, Tool::Pencil);
        assert_eq!(app.session.tools.brush.size, 7.0, "the pencil's size came back");
        pick_rail_tool(&mut app, &mut st, Tool::Eraser);
        pick_rail_tool(&mut app, &mut st, Tool::Brush);
        assert_eq!(app.session.tools.brush.size, 55.0, "the brush's size survived a trip through the eraser");
    }

    #[test]
    fn favourite_brushes_are_saved_and_loaded() {
        let dir = std::env::temp_dir().join(format!("stylusart-fav-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("favoritos.txt");
        let mut st = TouchUi::new();
        st.favorites_path = Some(path.clone());
        st.favorites.insert("Soft Pencil".into());
        st.favorites.insert("Hard Round".into());
        fresco::save_favorites_for_test(&mut st);
        let mut again = TouchUi::new();
        again.favorites_path = Some(path);
        fresco::load_favorites(&mut again);
        assert_eq!(again.favorites, st.favorites);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn popovers_stay_above_the_navigation_bar() {
        set_safe_bottom(48.0);
        assert_eq!(safe_bottom(), 48.0);
        set_safe_bottom(0.0);
        assert_eq!(safe_bottom(), 28.0, "a margin above the gesture area even without a bar");
        set_safe_bottom(500.0);
        assert_eq!(safe_bottom(), 160.0, "a bogus inset can't eat the screen");
        set_safe_bottom(0.0);
    }
}
