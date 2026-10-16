//! Touch interface for tablets (StylusArt on Android): large targets, a tool rail with brush
//! size and opacity sliders, a colour well, and a Layers sheet, in place of the desktop menu
//! bar, options bar, toolbar and dock. The canvas, dialogs and every action stay the same: each
//! control here runs the engine command the desktop UI runs (`tools.setBrush`, `history.undo`,
//! `layer.*`, `tools.setColors`).
//!
//! On when [`PhotocraftApp::touch`] is `Some`; the "…" menu switches back to the desktop
//! interface.

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
const SLIDER_W: f32 = 40.0;
const SLIDER_H: f32 = 190.0;
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
    /// A blank canvas was created at start, so the app opens ready to draw.
    pub started: bool,
    /// The active layer's opacity while its slider is dragged: committed as one step on release.
    opacity_drag: Option<(u64, f32)>,
    /// Where the colour popover opens (beside the colour well), in points.
    color_anchor: Option<Pos2>,
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
    tool_rail(app, ui, &t);
    slider_dock(app, ui.ctx(), &t);
    if st.layers_open {
        layers_sheet(app, &mut st, ui, &t);
    }
    if st.menu_open {
        menu_popover(app, &mut st, ui.ctx(), &t);
    }
    if st.color_open {
        color_popover(app, &mut st, ui.ctx(), &t);
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

fn tool_rail(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::left("touch-tools")
        .resizable(false)
        .exact_size(RAIL_W)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(13, 10)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                for &(tool, tip) in TOOLS {
                    if icon_button(ui, icons::tool_icon(tool), app.ui.tool == tool, tip).clicked() {
                        crate::tool_memory::select(app, tool);
                        crate::paint_mouse::sync_tool_brush(app);
                    }
                }
            });
        });
}

/// The size and opacity sliders, floating over the canvas' left edge (as in Procreate and
/// Photoshop on iPad), vertically centred.
fn slider_dock(app: &mut PhotocraftApp, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let h = 2.0 * SLIDER_H + 80.0;
    let top = (screen.center().y - h / 2.0).max(BAR_H + 8.0);
    egui::Area::new(egui::Id::new("touch-sliders")).order(egui::Order::Middle).fixed_pos(pos2(screen.left() + RAIL_W + 10.0, top)).show(ctx, |ui| {
        egui::Frame::NONE
            .fill(t.chrome.gamma_multiply(0.92))
            .corner_radius(CornerRadius::same(28))
            .inner_margin(egui::Margin::symmetric(6, 10))
            .show(ui, |ui| {
                ui.set_width(SLIDER_W + 8.0);
                brush_sliders(app, ui, t);
            });
    });
}

/// Brush size (logarithmic, 1–1000 px) and opacity (1–100 %) as tall touch sliders.
fn brush_sliders(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let b = &app.session.tools.brush;
    let size_u = (b.size.max(1.0).ln() / SLIDER_MAX_SIZE.ln()).clamp(0.0, 1.0);
    let (size, opacity) = (b.size, b.opacity);
    slider_caption(ui, "Tamaño", t);
    if let Some(u) = vertical_slider(ui, "touch-size", size_u, &format!("{} px", fmt_size(size)), t) {
        let s = SLIDER_MAX_SIZE.powf(u).clamp(1.0, SLIDER_MAX_SIZE);
        let s = if s < 10.0 { (s * 10.0).round() / 10.0 } else { s.round() };
        if s != size {
            run(app, "tools.setBrush", json!({ "brush": { "size": s }, "coalesce": "touch-size" }));
        }
        size_preview(ui, s, t);
    }
    ui.add_space(16.0);
    slider_caption(ui, "Opac.", t);
    if let Some(u) = vertical_slider(ui, "touch-opacity", opacity, &format!("{} %", (opacity * 100.0).round()), t) {
        let o = (u * 100.0).round().clamp(1.0, 100.0) / 100.0;
        if o != opacity {
            run(app, "tools.setBrush", json!({ "brush": { "opacity": o }, "coalesce": "touch-opacity" }));
        }
    }
}

fn slider_caption(ui: &mut egui::Ui, text: &str, t: &Tokens) {
    ui.vertical_centered(|ui| ui.label(egui::RichText::new(text).size(12.0).color(t.text_dim)));
    ui.add_space(2.0);
}

fn fmt_size(s: f32) -> String {
    if s < 10.0 { format!("{s:.1}") } else { format!("{}", s.round()) }
}

/// A tall slider filled from the bottom to `value` (0..1). While touched, returns the value under
/// the finger and shows `label` beside it.
fn vertical_slider(ui: &mut egui::Ui, id: &str, value: f32, label: &str, t: &Tokens) -> Option<f32> {
    let w = ui.available_width();
    let (outer, resp) = ui.allocate_exact_size(vec2(w, SLIDER_H), Sense::click_and_drag());
    let rect = Rect::from_center_size(outer.center(), vec2(SLIDER_W, SLIDER_H));
    let painter = ui.painter();
    let r = CornerRadius::same((SLIDER_W / 2.0) as u8);
    painter.rect_filled(rect, r, t.field);
    let fill_h = (rect.height() * value.clamp(0.0, 1.0)).max(SLIDER_W);
    let fill = Rect::from_min_max(pos2(rect.left(), rect.bottom() - fill_h), rect.right_bottom());
    painter.rect_filled(fill, r, t.accent_soft);
    painter.rect_stroke(rect, r, Stroke::new(1.0, t.field_border), StrokeKind::Inside);
    // The thumb.
    let y = (rect.bottom() - fill_h + SLIDER_W / 2.0).clamp(rect.top() + SLIDER_W / 2.0, rect.bottom() - SLIDER_W / 2.0);
    painter.circle_filled(pos2(rect.center().x, y), SLIDER_W / 2.0 - 4.0, t.accent);
    let _ = id;
    let active = resp.is_pointer_button_down_on() || resp.dragged();
    let p = resp.interact_pointer_pos().filter(|_| active)?;
    let u = ((rect.bottom() - SLIDER_W / 2.0 - p.y) / (rect.height() - SLIDER_W)).clamp(0.0, 1.0);
    let bubble = Rect::from_min_size(pos2(rect.right() + 18.0, p.y - 20.0), vec2(104.0, 40.0));
    let fg = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("touch-bubble")));
    fg.rect_filled(bubble, CornerRadius::same(10), t.card);
    fg.rect_stroke(bubble, CornerRadius::same(10), Stroke::new(1.0, t.card_border), StrokeKind::Inside);
    fg.text(bubble.center(), Align2::CENTER_CENTER, label, egui::FontId::proportional(18.0), t.text);
    Some(u)
}

/// The brush footprint at its real on-screen size while the size slider is dragged.
fn size_preview(ui: &egui::Ui, size: f32, t: &Tokens) {
    let ctx = ui.ctx();
    let screen = ctx.content_rect();
    let r = (size / ctx.pixels_per_point() / 2.0).clamp(1.0, screen.height() * 0.45);
    let c = pos2(screen.center().x, screen.center().y);
    let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("touch-size-preview")));
    fg.circle(c, r, Color32::from_black_alpha(40), Stroke::new(2.0, t.text));
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
