//! Touch interface for tablets (StylusArt on Android): large targets, a tool rail whose active
//! tool opens a brush popover (stroke preview, width, opacity, hardness), a colour well, a Layers
//! sheet and palm rejection, in place of the desktop menu
//! bar, options bar, toolbar and dock. The canvas, dialogs and every action stay the same: each
//! control here runs the engine command the desktop UI runs (`tools.setBrush`, `history.undo`,
//! `layer.*`, `tools.setColors`).
//!
//! On when [`PhotocraftApp::touch`] is `Some`; the "…" menu switches back to the desktop
//! interface.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, pos2, vec2};
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
    (Tool::Blur, "Desenfocar"),
    (Tool::PaintBucket, "Bote de pintura"),
    (Tool::Gradient, "Degradado"),
    (Tool::Eyedropper, "Cuentagotas"),
    (Tool::Lasso, "Lazo"),
    (Tool::RectMarquee, "Marco rectangular"),
    (Tool::Move, "Mover"),
    (Tool::Type, "Texto"),
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

/// Touch interface state (which sheets are open).
#[derive(Clone, Debug, Default)]
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
        if app.session.active().is_none() {
            new_canvas(app, ui.ctx());
        }
    }
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
        brush_popover(app, &mut st, ui.ctx(), &t);
    }
    // "Interfaz de escritorio" in the menu clears the state; otherwise keep it.
    if app.touch_off_requested {
        app.touch_off_requested = false;
    } else {
        app.touch = Some(st);
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

/// Brush settings beside the tool rail: a live preview of the stroke at its real width, then
/// width, opacity and hardness sliders and the pressure switch (Samsung Notes' pen popover).
fn brush_popover(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let height = 540.0;
    let anchor = st.brush_anchor.unwrap_or(pos2(RAIL_W + 10.0, BAR_H + 10.0));
    let lo = screen.top() + BAR_H + 6.0;
    let top = anchor.y.clamp(lo, (screen.bottom() - height - 8.0).max(lo));
    let mut close = false;
    egui::Area::new(egui::Id::new("touch-brush")).order(egui::Order::Foreground).fixed_pos(pos2(anchor.x, top)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            let w = BRUSH_POP_W - 32.0;
            ui.set_width(w);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tool_name(app.ui.tool)).size(20.0).strong().color(t.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icons::button_with_icon_size(ui, "x", 40.0, 20.0, false, "Cerrar").clicked() {
                        close = true;
                    }
                });
            });
            ui.add_space(6.0);
            stroke_preview(app, ui, w, t);
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
        });
    });
    if close {
        st.brush_open = false;
    }
}

/// A wavy stroke with the brush's width (as it looks on the canvas at the current zoom, capped
/// to the preview's height), opacity, hardness and colour, tapered when pressure sets the width.
fn stroke_preview(app: &PhotocraftApp, ui: &mut egui::Ui, w: f32, t: &Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 100.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, CornerRadius::same(12), t.field);
    let b = &app.session.tools.brush;
    let zoom = app.point_zoom().max(0.01);
    let width = (b.size * zoom).clamp(1.0, rect.height() - 28.0);
    let erase = matches!(app.ui.tool, Tool::Eraser | Tool::BackgroundEraser | Tool::MagicEraser);
    let base = if erase { t.text_dim } else { fg_color(app) };
    let with_alpha = |a: f32| Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), (a.clamp(0.0, 1.0) * 255.0).round() as u8);
    let (x0, x1) = (rect.left() + 20.0 + width / 2.0, rect.right() - 20.0 - width / 2.0);
    let amp = (rect.height() / 2.0 - 14.0 - width / 2.0).max(0.0);
    const STEPS: usize = 96;
    let pts: Vec<(Pos2, f32)> = (0..=STEPS)
        .map(|i| {
            let f = i as f32 / STEPS as f32;
            let c = pos2(x0 + (x1 - x0) * f, rect.center().y - (f * std::f32::consts::TAU).sin() * amp * 0.7);
            // Pressure ramps in and out, like a real stroke.
            let pressure = if b.pressure_size { (f * std::f32::consts::PI).sin().max(0.12) } else { 1.0 };
            (c, (width * pressure / 2.0).max(0.5))
        })
        .collect();
    // A soft brush: a wider, fainter band under the core.
    let soft = 1.0 - b.hardness.clamp(0.0, 1.0);
    if soft > 0.01 {
        p.add(ribbon(&pts, 1.0 + soft, with_alpha(b.opacity * 0.35)));
    }
    p.add(ribbon(&pts, 1.0, with_alpha(b.opacity)));
}

/// A filled band along `pts` (centre, half-width), as one triangle strip so a translucent colour
/// doesn't darken where it overlaps itself.
fn ribbon(pts: &[(Pos2, f32)], scale: f32, color: Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    for (i, &(c, r)) in pts.iter().enumerate() {
        let prev = pts.get(i.saturating_sub(1)).map_or(c, |p| p.0);
        let next = pts.get(i + 1).map_or(c, |p| p.0);
        let d = next - prev;
        let n = if d.length() > 0.0 { vec2(-d.y, d.x).normalized() } else { vec2(0.0, 1.0) };
        mesh.colored_vertex(c + n * r * scale, color);
        mesh.colored_vertex(c - n * r * scale, color);
        if i > 0 {
            let k = (2 * i) as u32;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k + 1, k);
        }
    }
    mesh
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

fn set_fg(app: &mut PhotocraftApp, c: Color32) {
    run(app, "tools.setColors", json!({ "foreground": format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()) }));
}

fn color_well(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let (outer, resp) = ui.allocate_exact_size(Vec2::splat(BTN), Sense::click());
    let c = outer.center();
    let p = ui.painter();
    p.circle_filled(c, BTN / 2.0 - 2.0, fg_color(app));
    p.circle_stroke(c, BTN / 2.0 - 2.0, Stroke::new(3.0, if st.color_open { t.accent } else { t.text }));
    if resp.clicked() {
        st.color_open = !st.color_open;
        st.menu_open = false;
        st.color_anchor = Some(pos2(outer.right(), outer.bottom() + 6.0));
    }
}

fn color_popover(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let size = vec2(360.0, 470.0);
    // Below the colour well, right-aligned with it.
    let anchor = st.color_anchor.unwrap_or(pos2(screen.right() - 80.0, BAR_H + 6.0));
    let left = (anchor.x - size.x).max(screen.left() + 8.0);
    let mut close = false;
    egui::Area::new(egui::Id::new("touch-color")).order(egui::Order::Foreground).fixed_pos(pos2(left, anchor.y)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            ui.set_width(size.x - 32.0);
            ui.spacing_mut().slider_width = size.x - 32.0;
            ui.spacing_mut().interact_size = vec2(40.0, 40.0);
            let mut c = fg_color(app);
            let before = c;
            egui::color_picker::color_picker_color32(ui, &mut c, egui::color_picker::Alpha::Opaque);
            if c != before {
                set_fg(app, c);
            }
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                for s in SWATCHES {
                    let col = Color32::from_rgb(s[0], s[1], s[2]);
                    let (r, resp) = ui.allocate_exact_size(Vec2::splat(42.0), Sense::click());
                    ui.painter().rect_filled(r, CornerRadius::same(8), col);
                    ui.painter().rect_stroke(r, CornerRadius::same(8), Stroke::new(1.0, t.card_border), StrokeKind::Inside);
                    if resp.clicked() {
                        set_fg(app, col);
                    }
                }
            });
            ui.add_space(6.0);
            if big_button(ui, "Listo", t).clicked() {
                close = true;
            }
        });
    });
    if close {
        st.color_open = false;
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
            ui.set_width(300.0);
            ui.spacing_mut().item_spacing.y = 8.0;
            if big_button(ui, "Nuevo lienzo", t).clicked() {
                new_canvas(app, ctx);
                close = true;
            }
            if big_button(ui, "Abrir…", t).clicked() {
                invoke(app, ctx, "file.open");
                close = true;
            }
            if big_button(ui, "Exportar…", t).clicked() {
                invoke(app, ctx, "file.export.exportAs");
                close = true;
            }
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
            // Topmost first, as in the Layers panel.
            let rows: Vec<(u64, String, bool, f32)> = d.doc.layers.iter().rev().map(|l| (l.id.0, l.name.clone(), l.visible, l.opacity)).collect();
            let active_opacity = rows.iter().find(|r| Some(r.0) == active).map(|r| r.3);
            egui::ScrollArea::vertical().max_height(ui.available_height() - 110.0).show(ui, |ui| {
                for (id, name, visible, _) in &rows {
                    layer_row(app, ui, *id, name, *visible, Some(*id) == active, t);
                }
            });
            if let (Some(id), Some(op)) = (active, active_opacity) {
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

fn layer_row(app: &mut PhotocraftApp, ui: &mut egui::Ui, id: u64, name: &str, visible: bool, selected: bool, t: &Tokens) {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click());
    let p = ui.painter();
    p.rect_filled(r, CornerRadius::same(12), if selected { t.row_selected } else { t.card });
    let eye = Rect::from_min_size(r.min, vec2(ROW_H, ROW_H));
    icons::paint(ui, eye, if visible { "eye" } else { "eye-off" }, 26.0, if visible { t.text } else { t.text_faint });
    p.text(pos2(eye.right() + 4.0, r.center().y), Align2::LEFT_CENTER, name, egui::FontId::proportional(18.0), t.text);
    if resp.clicked() {
        let tap = resp.interact_pointer_pos().unwrap_or(Pos2::ZERO);
        if eye.contains(tap) {
            run(app, "layer.setProps", json!({ "layer": id, "visible": !visible }));
        } else if !selected {
            run(app, "layer.select", json!({ "layer": id }));
        }
    }
    ui.add_space(6.0);
}
