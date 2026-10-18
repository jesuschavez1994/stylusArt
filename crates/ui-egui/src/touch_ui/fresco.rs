//! The touch interface's layout, organised as drawing apps on iPad are (Adobe Fresco's workspace
//! was the reference for where things go; the icons, colours and drawings are our own): a thin
//! top bar (home, the document's name and zoom, undo, redo, help, share, settings, focus mode), a
//! tool rail on the left with tool groups, the colour and a properties strip (size, brush, smoothing,
//! brush settings), and a taskbar on the right (layers with thumbnails, layer properties, add,
//! visibility, more, and the precision tools at the bottom). A touch shortcut circle on the canvas
//! turns the brush into an eraser while a finger holds it.

use super::*;

pub(super) const TOP: f32 = 50.0;
pub(super) const RAIL: f32 = 56.0;
pub(super) const TASK: f32 = 52.0;
const B: f32 = 40.0;
const THUMBS_W: f32 = 96.0;

/// A rail button: a group of related tools, showing the one last used.
struct Group {
    key: &'static str,
    variants: &'static [(Tool, &'static str)],
}

/// The rail, top to bottom. `Tool::Pencil` stands for the Lápiz (the Brush with a graphite brush,
/// see `pick_rail_tool`).
const GROUPS: &[Group] = &[
    Group { key: "brush", variants: &[(Tool::Brush, "Pincel")] },
    Group { key: "pencil", variants: &[(Tool::Pencil, "Lápiz")] },
    Group { key: "wet", variants: &[(Tool::MixerBrush, "Pincel mezclador")] },
    Group { key: "eraser", variants: &[(Tool::Eraser, "Borrador")] },
    Group { key: "move", variants: &[(Tool::Move, "Mover y transformar"), (Tool::Crop, "Recortar")] },
    Group { key: "select", variants: &[(Tool::Lasso, "Lazo"), (Tool::RectMarquee, "Rectangular"), (Tool::MagicWand, "Varita mágica"), (Tool::QuickSelection, "Selección rápida")] },
    Group { key: "fill", variants: &[(Tool::PaintBucket, "Relleno"), (Tool::Gradient, "Degradado")] },
    Group { key: "smudge", variants: &[(Tool::Smudge, "Difuminar"), (Tool::Blur, "Desenfocar"), (Tool::Patch, "Parche")] },
    Group { key: "eyedropper", variants: &[(Tool::Eyedropper, "Cuentagotas")] },
    Group { key: "shapes", variants: &[(Tool::Rectangle, "Rectángulo"), (Tool::EllipseShape, "Elipse"), (Tool::Line, "Línea"), (Tool::Artboard, "Mesa de trabajo")] },
    Group { key: "text", variants: &[(Tool::Type, "Texto")] },
];

/// Tools whose settings are a brush (size, library, smoothing, brush settings).
pub(super) fn brushlike(app: &PhotocraftApp) -> bool {
    matches!(app.ui.tool, Tool::Brush | Tool::MixerBrush | Tool::Eraser | Tool::Smudge | Tool::Blur)
}

/// The Spanish name of a tool (any rail variant), for popover titles.
pub(super) fn variant_name(tool: Tool) -> Option<&'static str> {
    GROUPS.iter().flat_map(|g| g.variants.iter()).find(|(t, _)| *t == tool).map(|(_, n)| *n)
}

/// A 40 pt icon button; `corner` draws the small triangle of a button with variants or options.
fn tbtn(ui: &mut egui::Ui, name: &str, selected: bool, tip: &str, corner: bool, t: &Tokens) -> egui::Response {
    let r = icons::button_with_icon_size(ui, name, B, 22.0, selected, tip);
    if corner {
        let c = r.rect.right_bottom() + vec2(-4.0, -4.0);
        ui.painter().add(egui::Shape::convex_polygon(vec![c, c + vec2(-6.0, 0.0), c + vec2(0.0, -6.0)], t.text_dim, Stroke::NONE));
    }
    r
}

/// An icon this set lacks, drawn: share (box and arrow), help (question mark), smoothing (wave),
/// rotate left/right.
fn glyph_button(ui: &mut egui::Ui, kind: &str, selected: bool, t: &Tokens) -> egui::Response {
    let (r, resp) = ui.allocate_exact_size(Vec2::splat(B), Sense::click());
    let p = ui.painter();
    if selected || resp.is_pointer_button_down_on() {
        p.rect_filled(r, CornerRadius::same(8), if selected { t.accent } else { t.pressed });
    }
    let s = Stroke::new(1.8, t.text);
    let c = r.center();
    match kind {
        "share" => {
            p.line_segment([c + vec2(0.0, 4.0), c + vec2(0.0, -9.0)], s);
            p.line_segment([c + vec2(-5.0, -4.0), c + vec2(0.0, -9.0)], s);
            p.line_segment([c + vec2(5.0, -4.0), c + vec2(0.0, -9.0)], s);
            p.add(egui::Shape::line(vec![c + vec2(-5.0, -1.0), c + vec2(-8.0, -1.0), c + vec2(-8.0, 9.0), c + vec2(8.0, 9.0), c + vec2(8.0, -1.0), c + vec2(5.0, -1.0)], s));
        }
        "help" => {
            p.circle_stroke(c, 10.0, s);
            p.text(c + vec2(0.0, 0.5), Align2::CENTER_CENTER, "?", egui::FontId::proportional(14.0), t.text);
        }
        "wave" => {
            let pts: Vec<Pos2> = (0..=24).map(|k| {
                let f = k as f32 / 24.0;
                c + vec2(-10.0 + 20.0 * f, -5.0 * (f * std::f32::consts::TAU).sin())
            }).collect();
            p.add(egui::Shape::line(pts, s));
        }
        "fliph" | "flipv" => {
            // Two mirrored triangles across a dashed axis.
            let (ax, tri) = if kind == "fliph" {
                ((c + vec2(0.0, -10.0), c + vec2(0.0, 10.0)), [[vec2(-2.5, -6.0), vec2(-10.0, 6.0), vec2(-2.5, 6.0)], [vec2(2.5, -6.0), vec2(10.0, 6.0), vec2(2.5, 6.0)]])
            } else {
                ((c + vec2(-10.0, 0.0), c + vec2(10.0, 0.0)), [[vec2(-6.0, -2.5), vec2(6.0, -10.0), vec2(6.0, -2.5)], [vec2(-6.0, 2.5), vec2(6.0, 10.0), vec2(6.0, 2.5)]])
            };
            p.add(egui::Shape::dashed_line(&[ax.0, ax.1], Stroke::new(1.2, t.text), 2.0, 2.0));
            for tri in tri {
                p.add(egui::Shape::closed_line(tri.iter().map(|v| c + *v).collect(), s));
            }
        }
        "rotl" | "rotr" => {
            let sx = if kind == "rotl" { -1.0 } else { 1.0 };
            p.rect_stroke(Rect::from_center_size(c + vec2(sx * 3.0, 3.0), Vec2::splat(11.0)), CornerRadius::same(1), s, StrokeKind::Middle);
            let arc: Vec<Pos2> = (0..=10).map(|k| {
                let a = std::f32::consts::PI * (1.0 + k as f32 / 20.0);
                c + vec2(sx * (-2.0 + 7.0 * a.cos()), -3.0 + 7.0 * a.sin())
            }).collect();
            let end = *arc.last().unwrap_or(&c);
            p.add(egui::Shape::line(arc, s));
            p.line_segment([end, end + vec2(sx * -3.0, -2.5)], s);
        }
        _ => {}
    }
    resp
}

// ---------------------------------------------------------------------------------------------
// Top bar
// ---------------------------------------------------------------------------------------------

pub(super) fn top_bar(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    egui::Panel::top("touch-top")
        .resizable(false)
        .exact_size(TOP)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(8, 0)))
        .show(ui, |ui| {
            let bar = ui.max_rect();
            ui.horizontal_centered(|ui| {
                if tbtn(ui, "house", st.home_open, "Inicio", false, t).clicked() {
                    let open = !st.home_open;
                    close_popovers(st);
                    st.home_open = open;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if tbtn(ui, "maximize-2", st.focus, "Pantalla completa", false, t).clicked() {
                        st.focus = !st.focus;
                    }
                    if tbtn(ui, "settings", st.settings_open, "Ajustes", false, t).clicked() {
                        let open = !st.settings_open;
                        close_popovers(st);
                        st.settings_open = open;
                    }
                    if glyph_button(ui, "share", st.share_open, t).clicked() {
                        let open = !st.share_open;
                        close_popovers(st);
                        st.share_open = open;
                    }
                    if glyph_button(ui, "help", st.help_open, t).clicked() {
                        let open = !st.help_open;
                        close_popovers(st);
                        st.help_open = open;
                    }
                    ui.add_space(8.0);
                    if tbtn(ui, "redo-2", false, "Rehacer", false, t).clicked() {
                        run(app, "history.redo", json!({}));
                    }
                    if tbtn(ui, "undo-2", false, "Deshacer", false, t).clicked() {
                        run(app, "history.undo", json!({}));
                    }
                });
            });
            // Document name and zoom, centred on the bar; the zoom chip fits the image on screen.
            let name = app.session.active().map(|d| d.doc.name.clone()).unwrap_or_else(|| "StylusArt".into());
            let zoom = app.session.active_index().and_then(|i| app.ui.views.get(i)).map_or(100.0, |v| v.zoom * 100.0);
            let p = ui.painter();
            let title = p.layout_no_wrap(name, egui::FontId::proportional(17.0), t.text);
            let chip_text = format!("{} %", zoom.round());
            let chip = p.layout_no_wrap(chip_text, egui::FontId::proportional(12.5), t.text_dim);
            let total = title.size().x + 12.0 + chip.size().x + 18.0;
            let x0 = bar.center().x - total / 2.0;
            let cy = bar.center().y;
            p.galley(pos2(x0, cy - title.size().y / 2.0), title.clone(), t.text);
            let chip_r = Rect::from_min_size(pos2(x0 + title.size().x + 12.0, cy - 11.0), vec2(chip.size().x + 18.0, 22.0));
            p.rect_filled(chip_r, CornerRadius::same(5), t.field);
            p.galley(pos2(chip_r.left() + 9.0, cy - chip.size().y / 2.0), chip, t.text_dim);
            if ui.interact(chip_r, egui::Id::new("touch-zoom-chip"), Sense::click()).clicked() {
                let ctx = ui.ctx().clone();
                invoke(app, &ctx, "view.fitOnScreen");
            }
        });
}

/// Close every popover (opening one closes the others).
pub(super) fn close_popovers(st: &mut TouchUi) {
    st.home_open = false;
    st.share_open = false;
    st.settings_open = false;
    st.help_open = false;
    st.color_open = false;
    st.brush_open = false;
    st.size_open = false;
    st.smooth_open = false;
    st.layer_props_open = false;
    st.layer_more_open = false;
    st.precision_open = false;
    st.library_open = false;
    st.picking = None;
}

/// A popover at `pos` (clamped above the safe bottom), its content scrolling when it doesn't fit.
fn popover(ctx: &egui::Context, id: &str, pos: Pos2, width: f32, t: &Tokens, add: impl FnOnce(&mut egui::Ui)) {
    let screen = ctx.content_rect();
    let x = pos.x.clamp(screen.left() + 8.0, (screen.right() - width - 8.0).max(screen.left() + 8.0));
    egui::Area::new(egui::Id::new(id)).order(egui::Order::Foreground).fixed_pos(pos2(x, pos.y)).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            ui.set_width(width - 32.0);
            // An area remembers last frame's size: give the scroll area its full room when it has
            // to scroll, or it would never grow back after a smaller frame.
            let room = popover_room(ui);
            egui::ScrollArea::vertical().max_height(room).min_scrolled_height(room).show(ui, add);
        });
    });
}

fn title_row(ui: &mut egui::Ui, title: &str, t: &Tokens) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).size(18.0).strong().color(t.text));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            close = icons::button_with_icon_size(ui, "x", 34.0, 18.0, false, "Cerrar").clicked();
        });
    });
    ui.add_space(4.0);
    close
}

pub(super) fn home_menu(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let mut close = false;
    popover(ctx, "touch-home", pos2(8.0, TOP + 6.0), 300.0, t, |ui| {
        close |= title_row(ui, "Inicio", t);
        ui.spacing_mut().item_spacing.y = 8.0;
        if big_button(ui, "Nuevo lienzo", t).clicked() {
            new_canvas(app, ctx);
            close = true;
        }
        if big_button(ui, "Abrir imagen…", t).clicked() {
            open_images(app, st, ctx);
            close = true;
        }
    });
    if close {
        st.home_open = false;
    }
}

pub(super) fn share_menu(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let mut close = false;
    let right = ctx.content_rect().right();
    popover(ctx, "touch-share", pos2(right - 8.0 - 300.0 - 3.0 * (B + 8.0), TOP + 6.0), 300.0, t, |ui| {
        close |= title_row(ui, "Compartir y exportar", t);
        ui.spacing_mut().item_spacing.y = 8.0;
        for (label, ext, transparent) in [("Exportar PNG", "png", false), ("Exportar PNG sin fondo", "png", true), ("Exportar JPG", "jpg", false)] {
            if big_button(ui, label, t).clicked() {
                export_image(app, st, ctx, ext, transparent);
                close = true;
            }
        }
        hint(ui, "Se guarda en Imágenes/StylusArt.", t);
    });
    if close {
        st.share_open = false;
    }
}

pub(super) fn help_popover(st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let mut close = false;
    let right = ctx.content_rect().right();
    popover(ctx, "touch-help", pos2(right - 8.0 - 360.0 - 4.0 * (B + 8.0), TOP + 6.0), 360.0, t, |ui| {
        close |= title_row(ui, "Gestos y atajos", t);
        for line in [
            "Dos dedos: mover, acercar y rotar el lienzo.",
            "Toque con dos dedos: deshacer. Con tres: rehacer.",
            "Mantén el círculo táctil con un dedo: el pincel borra.",
            "Termina un trazo dejando el lápiz quieto medio segundo: se convierte en línea, círculo, elipse o polígono.",
            "Toca otra vez la herramienta activa para ver sus opciones.",
            "Con el lazo, arrastra dentro de la selección para moverla.",
            "La palma no pinta: solo el lápiz dibuja sobre el lienzo.",
        ] {
            ui.label(egui::RichText::new(format!("•  {line}")).size(15.0).color(t.text));
            ui.add_space(4.0);
        }
    });
    if close {
        st.help_open = false;
    }
}

/// Document and view settings (the gear): flip and rotate the view, rotation snapping, the
/// touch shortcut, finger painting.
pub(super) fn settings_panel(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let mut close = false;
    let right = ctx.content_rect().right();
    popover(ctx, "touch-settings", pos2(right - 8.0 - 320.0 - B - 8.0, TOP + 6.0), 320.0, t, |ui| {
        close |= title_row(ui, "Ajustes", t);
        if let Some(d) = app.session.active() {
            ui.label(egui::RichText::new(d.doc.name.clone()).size(16.0).color(t.text));
            ui.label(egui::RichText::new(format!("Tamaño  {} × {} px", d.doc.size.width, d.doc.size.height)).size(14.0).color(t.accent));
        }
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Vista").size(16.0).strong().color(t.text));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Voltear lienzo").size(15.0).color(t.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if glyph_button(ui, "flipv", false, t).clicked() {
                    // Flipping vertically is a half turn plus a horizontal flip of the view.
                    invoke(app, ctx, "view.flipHorizontal");
                    turn_view(app, 180.0);
                }
                if glyph_button(ui, "fliph", app.ui.view.flip_horizontal, t).clicked() {
                    invoke(app, ctx, "view.flipHorizontal");
                }
            });
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Rotar lienzo").size(15.0).color(t.text));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if glyph_button(ui, "rotr", false, t).clicked() {
                    turn_view(app, 90.0);
                }
                if glyph_button(ui, "rotl", false, t).clicked() {
                    turn_view(app, -90.0);
                }
            });
        });
        let snap = f32::from_bits(ROT_SNAP.load(Ordering::Relaxed));
        let mut on = snap > 0.0;
        if switch_row(ui, "Imán de rotación", &mut on, t) {
            ROT_SNAP.store(if on { 45.0f32 } else { 0.0f32 }.to_bits(), Ordering::Relaxed);
        }
        if on {
            ui.horizontal(|ui| {
                for step in [30.0f32, 45.0, 90.0] {
                    if chip(ui, &format!("{step}°"), snap == step, t).clicked() {
                        ROT_SNAP.store(step.to_bits(), Ordering::Relaxed);
                    }
                }
            });
        }
        ui.add_space(6.0);
        let mut shortcut = TOUCH_SHORTCUT.load(Ordering::Relaxed);
        if switch_row(ui, "Atajo táctil", &mut shortcut, t) {
            TOUCH_SHORTCUT.store(shortcut, Ordering::Relaxed);
        }
        let mut finger = FINGER_PAINT.load(Ordering::Relaxed);
        if switch_row(ui, "Dibujar con el dedo", &mut finger, t) {
            FINGER_PAINT.store(finger, Ordering::Relaxed);
        }
        let mut snap_shapes = SHAPE_SNAP.load(Ordering::Relaxed);
        if switch_row(ui, "Ajustar a forma", &mut snap_shapes, t) {
            SHAPE_SNAP.store(snap_shapes, Ordering::Relaxed);
        }
    });
    if close {
        st.settings_open = false;
    }
}

fn turn_view(app: &mut PhotocraftApp, by: f32) {
    if let Some(i) = app.session.active_index()
        && let Some(v) = app.ui.views.get_mut(i)
    {
        v.rotation = crate::rotate_view::wrap_deg(v.rotation + by);
    }
}

/// Rotation snapping step in degrees (0 = off), applied when two fingers turn the view.
pub(super) static ROT_SNAP: AtomicU32 = AtomicU32::new(0);
/// The touch shortcut circle is shown.
pub(super) static TOUCH_SHORTCUT: AtomicBool = AtomicBool::new(true);
/// Holding the pen at the end of a stroke snaps it to a shape.
pub(super) static SHAPE_SNAP: AtomicBool = AtomicBool::new(true);

// ---------------------------------------------------------------------------------------------
// Left rail
// ---------------------------------------------------------------------------------------------

pub(super) fn tool_rail(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let bottom = (8.0 + safe_bottom()).min(120.0) as i8;
    egui::Panel::left("touch-tools")
        .resizable(false)
        .exact_size(RAIL)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 8, right: 8, top: 8, bottom }))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                let ctx = ui.ctx().clone();
                for g in GROUPS {
                    let current = st.variants.get(g.key).copied().unwrap_or(g.variants[0].0);
                    let selected = g.variants.iter().any(|(v, _)| rail_selected(app, st, *v));
                    let shown = if selected { g.variants.iter().map(|(v, _)| *v).find(|v| rail_selected(app, st, *v)).unwrap_or(current) } else { current };
                    let name = variant_name(shown).unwrap_or("");
                    let resp = tbtn(ui, icons::tool_icon(shown), selected, name, g.variants.len() > 1 || is_brush_group(g.key), t);
                    if resp.clicked() {
                        if selected {
                            // A second tap: the brush library for brushes, else the tool's options.
                            // The eraser's second tap shows its own settings (width, strength…).
                            let brushy = is_brush_group(g.key) && g.key != "eraser";
                            let open = if brushy { !st.library_open } else { !st.brush_open };
                            close_popovers(st);
                            if brushy {
                                st.library_open = open;
                            } else {
                                st.brush_open = open;
                                st.brush_anchor = Some(pos2(resp.rect.right() + 12.0, resp.rect.top()));
                            }
                        } else {
                            close_popovers(st);
                            pick_rail_tool(app, st, current);
                            st.variants.insert(g.key, current);
                        }
                    }
                }
                if tbtn(ui, "image", false, "Imagen", false, t).clicked() {
                    close_popovers(st);
                    open_images(app, st, &ctx);
                }
                ui.add_space(8.0);
                rail_color(app, st, ui, t);
                ui.add_space(8.0);
                properties_dock(app, st, ui, t);
            });
        });
}

fn is_brush_group(key: &str) -> bool {
    matches!(key, "brush" | "pencil" | "wet" | "eraser")
}

/// The colour circle on the rail.
fn rail_color(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let (r, resp) = ui.allocate_exact_size(vec2(B, B), Sense::click());
    let c = r.center();
    ui.painter().circle_filled(c, 15.0, shown_fg(app, st));
    ui.painter().circle_stroke(c, 15.0, Stroke::new(2.0, if st.color_open { t.accent } else { t.text }));
    if resp.clicked() {
        let open = !st.color_open;
        close_popovers(st);
        st.color_open = open;
        st.color_anchor = Some(pos2(r.right() + 12.0, r.top() - 180.0));
        st.hsv = color_to_hsv(fg_color(app), st.hsv);
    }
}

/// The properties strip under the colour: brush size (a number), the brush (library), smoothing,
/// brush settings. Dimmed for tools without a brush.
fn properties_dock(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let active = brushlike(app);
    egui::Frame::NONE.fill(t.field).corner_radius(CornerRadius::same(10)).inner_margin(egui::Margin::symmetric(0, 8)).show(ui, |ui| {
        ui.set_width(B);
        if !active {
            ui.disable();
        }
        ui.spacing_mut().item_spacing.y = 6.0;
        // Size: the number, tap for its slider.
        let size = app.session.tools.brush.size;
        let (r, resp) = ui.allocate_exact_size(vec2(B, 34.0), Sense::click());
        ui.painter().rect_filled(r.shrink2(vec2(4.0, 0.0)), CornerRadius::same(6), if st.size_open { t.accent } else { t.chrome });
        ui.painter().text(r.center(), Align2::CENTER_CENTER, fmt_size(size), egui::FontId::proportional(15.0), t.text);
        if resp.clicked() {
            let open = !st.size_open;
            close_popovers(st);
            st.size_open = open;
            st.dock_anchor = Some(pos2(r.right() + 16.0, r.top() - 40.0));
        }
        // The brush tip: the library.
        let (r, resp) = ui.allocate_exact_size(vec2(B, 34.0), Sense::click());
        let sq = Rect::from_center_size(r.center(), Vec2::splat(22.0));
        ui.painter().rect_filled(sq, CornerRadius::same(5), t.text);
        ui.painter().rect_filled(sq.shrink(4.0), CornerRadius::same(3), if st.library_open { t.accent } else { t.chrome });
        if resp.clicked() {
            let open = !st.library_open;
            close_popovers(st);
            st.library_open = open;
        }
        // Smoothing.
        let resp = glyph_button(ui, "wave", st.smooth_open, t);
        if resp.clicked() {
            let open = !st.smooth_open;
            close_popovers(st);
            st.smooth_open = open;
            st.dock_anchor = Some(pos2(resp.rect.right() + 16.0, resp.rect.top() - 40.0));
        }
        // Brush settings.
        let resp = tbtn(ui, "sliders-horizontal", st.brush_open, "Ajustes del pincel", false, t);
        if resp.clicked() {
            let open = !st.brush_open;
            close_popovers(st);
            st.brush_open = open;
            st.brush_anchor = Some(pos2(resp.rect.right() + 16.0, resp.rect.top() - 420.0));
        }
    });
}

/// Brush size: a slider, and the brush at its real size on the canvas while it is dragged.
pub(super) fn size_popover(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let at = st.dock_anchor.unwrap_or(pos2(RAIL + 12.0, 600.0));
    let mut dragging = None;
    popover(ctx, "touch-size", at, 320.0, t, |ui| {
        let b = app.session.tools.brush.clone();
        caption(ui, "Tamaño", &format!("{} px", fmt_size(b.size)), t);
        let u = (b.size.max(1.0).ln() / SLIDER_MAX_SIZE.ln()).clamp(0.0, 1.0);
        if let Some(u) = h_slider(ui, u, SliderFill::Accent, t) {
            let s = SLIDER_MAX_SIZE.powf(u).clamp(1.0, SLIDER_MAX_SIZE);
            let s = if s < 10.0 { (s * 10.0).round() / 10.0 } else { s.round() };
            if s != b.size {
                let spacing = if s > 150.0 { 0.2 } else if s > 60.0 { 0.15 } else { b.spacing };
                let brush = if spacing > b.spacing { json!({ "size": s, "spacing": spacing }) } else { json!({ "size": s }) };
                run(app, "tools.setBrush", json!({ "brush": brush, "coalesce": "touch-size" }));
            }
            dragging = Some(s);
        }
    });
    if let (Some(s), Some(canvas)) = (dragging, *CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner)) {
        let r = (s * app.point_zoom() / 2.0).clamp(1.0, canvas.height() * 0.45);
        let fg = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("touch-size-preview")));
        fg.circle(canvas.center(), r, Color32::from_black_alpha(30), Stroke::new(1.5, Color32::from_gray(30)));
        let label = Rect::from_center_size(canvas.center() + vec2(0.0, r + 24.0), vec2(84.0, 28.0));
        fg.rect_filled(label, CornerRadius::same(14), t.chrome);
        fg.text(label.center(), Align2::CENTER_CENTER, format!("{} px", fmt_size(s)), egui::FontId::proportional(14.0), t.text);
    }
}

pub(super) fn smooth_popover(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let at = st.dock_anchor.unwrap_or(pos2(RAIL + 12.0, 650.0));
    popover(ctx, "touch-smooth", at, 320.0, t, |ui| {
        let amount = app.session.tools.brush.smoothing.amount;
        caption(ui, "Suavizado", &format!("{} %", (amount * 100.0).round()), t);
        if let Some(u) = h_slider(ui, amount, SliderFill::Accent, t) {
            let v = (u * 100.0).round() / 100.0;
            if v != amount {
                run(app, "tools.setBrush", json!({ "brush": { "smoothing": { "amount": v } }, "coalesce": "touch-smoothing" }));
            }
        }
        hint(ui, "Más suavizado da líneas más limpias; menos, trazos más rápidos.", t);
    });
}

// ---------------------------------------------------------------------------------------------
// Brush library with favourites
// ---------------------------------------------------------------------------------------------

/// The preset groups in Spanish.
fn group_label(group: &str) -> String {
    match group {
        "General" => "Generales".into(),
        "Dry Media" => "Secos".into(),
        "Wet Media" => "Húmedos".into(),
        "Special Effects" => "Efectos".into(),
        "" => "Otros".into(),
        other => other.into(),
    }
}

pub(super) fn brush_library(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let width = 380.0f32.min(screen.width() - RAIL - 40.0);
    let height = screen.height() - TOP - 20.0 - safe_bottom();
    let at = pos2(RAIL + 10.0, TOP + 10.0);
    let mut close = false;
    let mut pick: Option<String> = None;
    let mut toggle_fav: Option<String> = None;
    let color = Color32::from_gray(232);
    let mut groups: Vec<String> = Vec::new();
    for p in &app.session.tools.presets {
        if !groups.contains(&p.group) {
            groups.push(p.group.clone());
        }
    }
    egui::Area::new(egui::Id::new("touch-brush-library")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        popover_frame(t).show(ui, |ui| {
            ui.set_width(width - 32.0);
            ui.set_height(height - 32.0);
            close |= title_row(ui, &format!("{} · pinceles", active_tool_name(app, st)), t);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                let mut tabs = vec![("★ Favoritos".to_string(), "★".to_string()), ("Todos".to_string(), String::new())];
                tabs.extend(groups.iter().map(|g| (group_label(g), g.clone())));
                for (label, key) in tabs {
                    if chip(ui, &label, st.library_tab == key, t).clicked() {
                        st.library_tab = key;
                    }
                }
            });
            ui.add(egui::TextEdit::singleline(&mut st.brush_query).hint_text("Buscar pincel…").desired_width(f32::INFINITY).font(egui::FontId::proportional(16.0)).margin(egui::Margin::symmetric(10, 6)));
            ui.add_space(6.0);
            let q = st.brush_query.to_lowercase();
            let list: Vec<(usize, String, String)> = app
                .session
                .tools
                .presets
                .iter()
                .enumerate()
                .filter(|(_, p)| match st.library_tab.as_str() {
                    "★" => st.favorites.contains(&p.name),
                    "" => true,
                    g => p.group == g,
                })
                .filter(|(_, p)| q.is_empty() || p.name.to_lowercase().contains(&q))
                .map(|(i, p)| (i, p.name.clone(), group_label(&p.group)))
                .collect();
            if list.is_empty() {
                hint(ui, if st.library_tab == "★" { "Aún no tienes favoritos: toca la estrella de un pincel para añadirlo aquí." } else { "No hay pinceles que coincidan." }, t);
                return;
            }
            let row_h = 84.0;
            let ppp = ui.ctx().pixels_per_point();
            let current = app.session.tools.current_preset.clone();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, row_h, list.len(), |ui, range| {
                for (i, name, group) in list.get(range).unwrap_or_default() {
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), row_h - 6.0), Sense::click());
                    let sel = current.as_deref() == Some(name.as_str());
                    ui.painter().rect_filled(r, CornerRadius::same(10), if sel { t.row_selected } else if resp.is_pointer_button_down_on() { t.pressed } else { t.field });
                    let stroke_r = Rect::from_min_max(pos2(r.left() + 10.0, r.top() + 6.0), pos2(r.right() - 56.0, r.bottom() - 26.0));
                    if let Some(p) = app.session.tools.presets.get(*i) {
                        let tex = crate::brush_preview::stroke_texture(ui.ctx(), &format!("touch-lib-{i}"), &p.brush, (stroke_r.width() * ppp) as u32, (stroke_r.height() * ppp) as u32, color);
                        ui.painter().image(tex.id(), stroke_r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    ui.painter().text(pos2(r.left() + 14.0, r.bottom() - 13.0), Align2::LEFT_CENTER, name, egui::FontId::proportional(15.0), t.text);
                    let _ = group;
                    let star = Rect::from_center_size(pos2(r.right() - 28.0, r.center().y), Vec2::splat(40.0));
                    let fav = st.favorites.contains(name);
                    icons::paint(ui, star, if fav { "star-filled" } else { "star" }, 22.0, if fav { Color32::from_rgb(255, 201, 60) } else { t.text_dim });
                    if resp.clicked() {
                        let tap = resp.interact_pointer_pos().unwrap_or(Pos2::ZERO);
                        if star.contains(tap) {
                            toggle_fav = Some(name.clone());
                        } else {
                            pick = Some(name.clone());
                        }
                    }
                    ui.add_space(6.0);
                }
            });
        });
    });
    if let Some(name) = toggle_fav {
        if !st.favorites.remove(&name) {
            st.favorites.insert(name);
        }
        save_favorites(st);
    }
    if let Some(name) = pick {
        match app.run("tools.setBrush", json!({ "preset": name })) {
            Ok(_) => notify(st, ctx, &format!("Pincel: {name}")),
            Err(e) => notify(st, ctx, &e),
        }
    }
    if close {
        st.library_open = false;
    }
}

/// Favourite brushes, one name per line, in the app's data folder.
pub(super) fn load_favorites(st: &mut TouchUi) {
    if let Some(path) = &st.favorites_path
        && let Ok(text) = std::fs::read_to_string(path)
    {
        st.favorites = text.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
    }
}

#[cfg(test)]
pub(super) fn save_favorites_for_test(st: &mut TouchUi) {
    save_favorites(st);
}

fn save_favorites(st: &mut TouchUi) {
    if let Some(path) = &st.favorites_path {
        let mut names: Vec<&String> = st.favorites.iter().collect();
        names.sort();
        let text = names.iter().map(|n| n.as_str()).collect::<Vec<_>>().join("\n");
        if let Err(e) = std::fs::write(path, text) {
            log::warn!("favourites: {e}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Right taskbar, layer thumbnails and their popovers
// ---------------------------------------------------------------------------------------------

pub(super) fn task_bar(app: &mut PhotocraftApp, st: &mut TouchUi, ui: &mut egui::Ui, t: &Tokens) {
    let bottom = (8.0 + safe_bottom()).min(120.0) as i8;
    egui::Panel::right("touch-task")
        .resizable(false)
        .exact_size(TASK)
        .frame(egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin { left: 6, right: 6, top: 8, bottom }))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let full = ui.max_rect();
            if tbtn(ui, "layers", st.layers_open, "Capas", false, t).clicked() {
                st.layers_open = !st.layers_open;
            }
            let resp = tbtn(ui, "sliders-horizontal", st.layer_props_open, "Propiedades de la capa", false, t);
            if resp.clicked() {
                let open = !st.layer_props_open;
                close_popovers(st);
                st.layer_props_open = open;
            }
            ui.add_space((full.height() * 0.32).max(8.0));
            if tbtn(ui, "plus", false, "Nueva capa", false, t).clicked() {
                run(app, "layer.new.layer", json!({}));
            }
            let visible = active_layer(app).map(|l| l.visible).unwrap_or(true);
            if tbtn(ui, if visible { "eye" } else { "eye-off" }, false, "Mostrar u ocultar la capa", false, t).clicked()
                && let Some(l) = active_layer(app)
            {
                let (id, v) = (l.id.0, l.visible);
                run(app, "layer.setProps", json!({ "layer": id, "visible": !v }));
            }
            if tbtn(ui, "ellipsis", st.layer_more_open, "Más", false, t).clicked() {
                let open = !st.layer_more_open;
                close_popovers(st);
                st.layer_more_open = open;
            }
            // The precision tools, at the bottom.
            let ruler = Rect::from_min_size(pos2(full.left(), full.bottom() - B), Vec2::splat(B));
            let resp = ui.put(ruler, |ui: &mut egui::Ui| tbtn(ui, "ruler", st.precision_open, "Precisión", true, t));
            if resp.clicked() {
                let open = !st.precision_open;
                close_popovers(st);
                st.precision_open = open;
            }
        });
}

fn active_layer(app: &PhotocraftApp) -> Option<&Layer> {
    let d = app.session.active()?;
    d.doc.layer(d.active_layer?)
}

/// The layers as thumbnails beside the taskbar, topmost first; a tap selects.
pub(super) fn layer_thumbs(app: &mut PhotocraftApp, ui: &mut egui::Ui, t: &Tokens) {
    let Some(d) = app.session.active() else { return };
    let doc = d.doc.clone();
    let active = d.active_layer.map(|l| l.0);
    let mut list = Vec::new();
    rows(&doc.layers, 0, &mut list);
    let ctx = ui.ctx().clone();
    let bottom = (8.0 + safe_bottom()).min(120.0) as i8;
    egui::Panel::right("touch-thumbs")
        .resizable(false)
        .exact_size(THUMBS_W)
        .frame(egui::Frame::NONE.fill(t.dock).inner_margin(egui::Margin { left: 8, right: 8, top: 8, bottom }))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for row in &list {
                    let (r, resp) = ui.allocate_exact_size(vec2(THUMBS_W - 16.0, THUMBS_W - 16.0 + 18.0), Sense::click());
                    let sq = Rect::from_min_size(r.min, Vec2::splat(r.width()));
                    let p = ui.painter();
                    p.rect_filled(sq, CornerRadius::same(6), Color32::WHITE);
                    if let Some(layer) = doc.layer(photocraft_doc::LayerId(row.id))
                        && matches!(layer.content, LayerContent::Raster(_))
                    {
                        let (tex, _) = app.layer_thumb(&ctx, &doc, layer);
                        ui.painter().image(tex, sq.shrink(2.0), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    } else {
                        let icon = if row.artboard { "artboard" } else if row.group { "folder" } else { "adjustment-layer" };
                        icons::paint(ui, sq, icon, 26.0, Color32::from_gray(90));
                    }
                    let p = ui.painter();
                    if !row.visible {
                        p.rect_filled(sq, CornerRadius::same(6), Color32::from_black_alpha(120));
                        icons::paint(ui, sq, "eye-off", 22.0, Color32::WHITE);
                    }
                    let sel = Some(row.id) == active;
                    ui.painter().rect_stroke(sq, CornerRadius::same(6), Stroke::new(if sel { 3.0 } else { 1.0 }, if sel { t.accent } else { t.card_border }), StrokeKind::Outside);
                    let name: String = row.name.chars().take(10).collect();
                    ui.painter().text(pos2(r.center().x, r.bottom() - 4.0), Align2::CENTER_BOTTOM, name, egui::FontId::proportional(11.0), if sel { t.text } else { t.text_dim });
                    if resp.clicked() && !sel {
                        run(app, "layer.select", json!({ "layer": row.id }));
                    }
                }
            });
        });
}

pub(super) fn layer_props(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let right = ctx.content_rect().right();
    let mut close = false;
    let x = right - TASK - if st.layers_open { THUMBS_W } else { 0.0 } - 330.0;
    popover(ctx, "touch-layer-props", pos2(x, TOP + 50.0), 320.0, t, |ui| {
        close |= title_row(ui, "Propiedades de la capa", t);
        let Some(d) = app.session.active() else { return };
        let Some(id) = d.active_layer else { return };
        let is_bg = d.doc.is_background(id);
        let Some(l) = d.doc.layer(id) else { return };
        let (opacity, blend, mask, clipped, lock_t) = (l.opacity, l.blend, l.mask.is_some(), l.clipped, l.locks.transparency);
        let id = id.0;
        if is_bg {
            hint(ui, "El Fondo no tiene opacidad ni modo de fusión. Para dibujar y borrar sin tocarlo, añade una capa con +.", t);
        } else {
            let shown = st.opacity_drag.filter(|(l, _)| *l == id).map_or(opacity, |(_, v)| v);
            caption(ui, "Opacidad", &format!("{} %", (shown * 100.0).round()), t);
            match h_slider(ui, shown, SliderFill::Accent, t) {
                Some(u) => st.opacity_drag = Some((id, (u * 100.0).round() / 100.0)),
                None => {
                    if let Some((l, v)) = st.opacity_drag.take()
                        && (v - opacity).abs() > f32::EPSILON
                    {
                        run(app, "layer.setProps", json!({ "layer": l, "opacity": v }));
                    }
                }
            }
            ui.add_space(8.0);
            blend_picker(app, st, ui, id, blend, t);
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            if chip(ui, "Quitar fondo", false, t).clicked() {
                remove_background(app, st, ctx);
            }
            if chip(ui, if mask { "Quitar máscara" } else { "Añadir máscara" }, mask, t).clicked()
                && let Err(e) = app.run(if mask { "layer.layerMask.delete" } else { "layer.layerMask.revealAll" }, json!({}))
            {
                notify(st, ctx, &e);
            }
            if !is_bg {
                if chip(ui, "Máscara de recorte", clipped, t).clicked()
                    && let Err(e) = app.run("layer.setProps", json!({ "layer": id, "clipped": !clipped }))
                {
                    notify(st, ctx, &e);
                }
                if chip(ui, "Pintar dentro", lock_t, t).clicked()
                    && let Err(e) = app.run("layer.setProps", json!({ "layer": id, "locks": { "transparency": !lock_t } }))
                {
                    notify(st, ctx, &e);
                }
            }
        });
    });
    if close {
        st.layer_props_open = false;
    }
}

pub(super) fn layer_more(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let right = ctx.content_rect().right();
    let mut close = false;
    let x = right - TASK - if st.layers_open { THUMBS_W } else { 0.0 } - 290.0;
    let y = ctx.content_rect().top() + TOP + 300.0;
    popover(ctx, "touch-layer-more", pos2(x, y), 280.0, t, |ui| {
        close |= title_row(ui, "Capa", t);
        ui.spacing_mut().item_spacing.y = 8.0;
        for (label, id) in [("Duplicar", "layer.duplicate"), ("Combinar hacia abajo", "layer.mergeLayers"), ("Eliminar", "layer.delete")] {
            if big_button(ui, label, t).clicked() {
                if let Err(e) = app.run(id, json!({})) {
                    notify(st, ctx, &e);
                }
                close = true;
            }
        }
    });
    if close {
        st.layer_more_open = false;
    }
}

/// Precision (the ruler at the bottom of the taskbar): shapes, symmetry, painting inside the
/// layer's pixels, shape snapping.
pub(super) fn precision_popover(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let screen = ctx.content_rect();
    let mut close = false;
    let height = 380.0;
    let x = screen.right() - TASK - if st.layers_open { THUMBS_W } else { 0.0 } - 340.0;
    let y = (screen.bottom() - safe_bottom() - height).max(screen.top() + TOP + 6.0);
    popover(ctx, "touch-precision", pos2(x, y), 330.0, t, |ui| {
        close |= title_row(ui, "Precisión", t);
        if big_button(ui, "Formas: rectángulo · elipse · línea", t).clicked() {
            pick_rail_tool(app, st, Tool::Rectangle);
            st.variants.insert("shapes", Tool::Rectangle);
            close = true;
        }
        ui.add_space(8.0);
        symmetry_options(app, st, ui, t);
        ui.add_space(8.0);
        if let Some(l) = active_layer(app).filter(|_| app.session.active().is_some_and(|d| d.active_layer.is_some_and(|id| !d.doc.is_background(id)))) {
            let (id, mut on) = (l.id.0, l.locks.transparency);
            if switch_row(ui, "Pintar dentro", &mut on, t) {
                run(app, "layer.setProps", json!({ "layer": id, "locks": { "transparency": on } }));
            }
        }
        let mut snap = SHAPE_SNAP.load(Ordering::Relaxed);
        if switch_row(ui, "Ajustar a forma", &mut snap, t) {
            SHAPE_SNAP.store(snap, Ordering::Relaxed);
        }
    });
    if close {
        st.precision_open = false;
    }
}

// ---------------------------------------------------------------------------------------------
// Touch shortcut: a circle on the canvas; while a finger holds it, the brush erases.
// ---------------------------------------------------------------------------------------------

/// Where the touch shortcut circle is (points), for the touch filter and gestures.
pub(super) static SHORTCUT_RECT: Mutex<Option<Rect>> = Mutex::new(None);
/// A finger holds the touch shortcut.
pub(super) static SHORTCUT_HELD: AtomicBool = AtomicBool::new(false);

pub(super) fn touch_shortcut(app: &mut PhotocraftApp, st: &mut TouchUi, ctx: &egui::Context, t: &Tokens) {
    let canvas = *CANVAS_RECT.lock().unwrap_or_else(PoisonError::into_inner);
    let shown = TOUCH_SHORTCUT.load(Ordering::Relaxed) && canvas.is_some();
    let held = shown && SHORTCUT_HELD.load(Ordering::Relaxed);
    // Hold: the eraser; release: back to the tool it replaced.
    if held && st.shortcut_prev.is_none() && app.ui.tool != Tool::Eraser {
        let prev = if app.ui.tool == Tool::Brush && st.pencil_mode { Tool::Pencil } else { app.ui.tool };
        st.shortcut_prev = Some(prev);
        pick_rail_tool(app, st, Tool::Eraser);
    } else if !held && let Some(prev) = st.shortcut_prev.take() {
        pick_rail_tool(app, st, prev);
    }
    let Some(canvas) = canvas.filter(|_| shown) else {
        *SHORTCUT_RECT.lock().unwrap_or_else(PoisonError::into_inner) = None;
        return;
    };
    let screen_bottom = ctx.content_rect().bottom() - safe_bottom();
    let c = pos2(canvas.left() + 46.0, canvas.bottom().min(screen_bottom) - 46.0);
    let r = Rect::from_center_size(c, Vec2::splat(56.0));
    *SHORTCUT_RECT.lock().unwrap_or_else(PoisonError::into_inner) = Some(r);
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("touch-shortcut")));
    p.circle(c, 24.0, if held { t.accent.gamma_multiply(0.5) } else { Color32::from_white_alpha(18) }, Stroke::new(1.6, if held { t.accent } else { Color32::from_gray(180) }));
    if held {
        let label = Rect::from_min_size(pos2(c.x + 32.0, c.y - 14.0), vec2(90.0, 28.0));
        p.rect_filled(label, CornerRadius::same(8), t.accent);
        p.text(label.center(), Align2::CENTER_CENTER, "Borrador", egui::FontId::proportional(14.0), Color32::WHITE);
    }
}
