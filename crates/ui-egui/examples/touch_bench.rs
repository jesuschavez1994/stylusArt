//! Responsiveness of the touch interface (StylusArt): how long switching the active layer and
//! picking a colour take, as the command alone and as the whole frame drawn after it, on a
//! tablet-sized canvas (3200 × 2136) with painted layers and the Layers sheet open.
//!
//! ```sh
//! cargo run --release -p photocraft-ui-egui --example touch_bench
//! ```

use std::time::Instant;

use photocraft_ui_egui::{PhotocraftApp, Services};
use serde_json::json;

fn stats(name: &str, v: &[f64]) {
    let mut s = v.to_vec();
    s.sort_by(f64::total_cmp);
    let pct = |p: f64| s.get(((s.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0.0);
    println!("{name:<34} median {:>7.2} ms   p90 {:>7.2} ms   max {:>7.2} ms", pct(0.5), pct(0.9), pct(1.0));
}

fn main() {
    let mut harness = egui_kittest::Harness::builder().with_size(egui::vec2(1600.0, 1068.0)).with_pixels_per_point(2.0).with_max_steps(64).wgpu().build_eframe(|cc| {
        PhotocraftApp::setup_context(&cc.egui_ctx, Default::default());
        let mut app = PhotocraftApp::new(photocraft_engine::Session::new(), Services::default());
        let mut touch = photocraft_ui_egui::touch_ui::TouchUi::new();
        touch.layers_open = true;
        app.touch = Some(touch);
        // The CPU canvas: the software adapter here has no usable GPU (the tablet's is real).
        app.perf.gpu_info.selected = "cpu".into();
        app.perf.gpu_info.canvas = "cpu".into();
        app
    });
    harness.run_steps(4);
    let app = harness.state_mut();
    app.run("file.new", json!({"width": 3200, "height": 2136, "background": "white", "name": "Bench"})).expect("new");
    for (i, c) in ["#e63946", "#2a9d8f", "#457b9d", "#f4a261", "#8338ec", "#ffb703"].iter().enumerate() {
        app.run("layer.new.layer", json!({"name": format!("Capa {}", i + 1)})).expect("layer");
        app.run("tools.setColors", json!({"foreground": c})).expect("color");
        let (x, y) = (200 + i as i64 * 420, 300 + i as i64 * 180);
        app.run("select.rect", json!({"rect": [x, y, 900, 900]})).ok();
        app.run("edit.fill", json!({"contents": "foreground"})).expect("fill");
        app.run("select.deselect", json!({})).ok();
    }
    harness.run_steps(8);
    let ids: Vec<u64> = harness.state().session.active().map(|d| d.doc.layers.iter().map(|l| l.id.0).collect()).unwrap_or_default();
    println!("layers: {}", ids.len());

    let (mut cmd, mut frame) = (Vec::new(), Vec::new());
    for i in 0..40 {
        let id = ids[1 + i % (ids.len() - 1)];
        let t = Instant::now();
        harness.state_mut().run("layer.select", json!({"layer": id})).expect("select");
        cmd.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        harness.step();
        frame.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    stats("layer.select (command)", &cmd);
    stats("frame after layer.select", &frame);

    let (mut cmd, mut frame) = (Vec::new(), Vec::new());
    for i in 0..60 {
        let c = format!("#{:02x}{:02x}{:02x}", (i * 4) % 256, (255 - i * 3) % 256, (i * 7) % 256);
        let t = Instant::now();
        harness.state_mut().run("tools.setColors", json!({"foreground": c})).expect("color");
        cmd.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        harness.step();
        frame.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    stats("tools.setColors (command)", &cmd);
    stats("frame after tools.setColors", &frame);

    // Eraser strokes across the canvas (a 2400 px stroke, 120 points), by size and dab spacing.
    let pts: Vec<[f64; 3]> = (0..120).map(|k| [300.0 + k as f64 * 20.0, 800.0 + (k as f64 * 0.2).sin() * 300.0, 1.0]).collect();
    for (size, spacing) in [(40.0, 0.1), (400.0, 0.1), (400.0, 0.2)] {
        let mut times = Vec::new();
        for _ in 0..5 {
            let t = Instant::now();
            harness.state_mut().run("paint.stroke", json!({"points": pts, "size": size, "spacing": spacing, "hardness": 1.0, "erase": true})).expect("erase");
            times.push(t.elapsed().as_secs_f64() * 1000.0);
            harness.state_mut().run("history.undo", json!({})).expect("undo");
        }
        stats(&format!("eraser {size} px, spacing {}%", spacing * 100.0), &times);
    }

    let mut idle = Vec::new();
    for _ in 0..30 {
        let t = Instant::now();
        harness.step();
        idle.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    stats("idle frame (nothing changed)", &idle);
}
