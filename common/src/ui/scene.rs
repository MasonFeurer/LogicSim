use crate::sim::scene::{Scene, UNIT};
use crate::ui::Transform;
use egui::{Align2, Color32, Id, Painter, Rect, Sense, Ui};
use glam::vec2;

enum LabelPlacement {
    Top,
    Left,
    Right,
}

fn label(p: &Painter, t: Transform, bounds: Rect, label: &str, place: LabelPlacement) {
    // Place label above `bounds`, centerd
    let fsize = t.inv() * 15.0;
    let (pos, align2) = match place {
        LabelPlacement::Top => (
            bounds.center_top() - egui::vec2(0.0, fsize * 0.5),
            Align2::CENTER_CENTER,
        ),
        LabelPlacement::Left => (bounds.left_center(), Align2::RIGHT_CENTER),
        LabelPlacement::Right => (bounds.right_center(), Align2::LEFT_CENTER),
    };
    p.text(t * pos, align2, label, Default::default(), Color32::WHITE);
}

pub fn show_scene(ui: &mut Ui, scene: &mut Scene, snap_to_grid: bool, show_grid: bool) {
    let screen_size = ui.clip_rect().size();
    let screen_size = glam::vec2(screen_size.x, screen_size.y);

    // ----- Handle Pan + Zoom -----
    let rect = ui.available_rect_before_wrap();
    let rs = ui.interact(rect, Id::from("pan+zoom"), Sense::click_and_drag());

    if let Some(egui::Pos2 { x, y }) = ui.ctx().pointer_latest_pos() {
        let zoom = ui.ctx().input(|state| state.zoom_delta());
        let drag = rs.drag_delta();
        scene.transform.translate(vec2(drag.x, drag.y));
        scene.transform.zoom(vec2(x, y), zoom - 1.0, 0.1..=100.0);
    }

    let ptr_released = ui.input(|state| {
        state
            .events
            .iter()
            .find(|event| match event {
                egui::Event::PointerButton { pressed, .. } => !*pressed,
                _ => false,
            })
            .is_some()
    });

    let t = scene.transform;
    let p = ui.painter();

    if show_grid {
        // How far away from the screens origin we should show the lines
        let screen_offset = t.offset % (t * UNIT);
        // How far apart the lines should appear on screen
        let screen_gap = t * UNIT;
        // The number of lines to show across the width and height of the screen
        let line_count = screen_size / screen_gap;

        let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(50));
        for i in 0..(line_count.y.ceil()) as u32 {
            let y = i as f32 * screen_gap + screen_offset.y;
            let a = egui::pos2(0.0, y);
            let b = egui::pos2(screen_size.x, y);
            p.line_segment([a, b], stroke);
        }
        for i in 0..(line_count.x.ceil()) as u32 {
            let x = i as f32 * screen_gap + screen_offset.x;
            let a = egui::pos2(x, 0.0);
            let b = egui::pos2(x, screen_size.y);
            p.line_segment([a, b], stroke);
        }
    }

    for (id, device) in &mut scene.devices {
        let bounds = device.bounds();
        let color = Color32::from_gray(200);

        p.rect_filled(t * bounds, t * 4.0, color);

        let rs = ui.interact(
            t * bounds,
            Id::from("chip").with(id),
            Sense::click_and_drag(),
        );

        device.pos_mut().x += t.inv() * rs.drag_delta().x;
        device.pos_mut().y += t.inv() * rs.drag_delta().y;

        if snap_to_grid && ptr_released {
            let u = UNIT;
            *device.pos_mut() = u * (device.pos() / u).round();
        }

        label(p, t, bounds, device.name(), LabelPlacement::Top);

        for (i, (_addr, name, _ty)) in device.l_nodes().iter().enumerate() {
            // let is_input = matches!(ty, crate::sim::save::IoType::Input);

            let center = egui::pos2(bounds.min.x, bounds.min.y + i as f32 * UNIT + UNIT * 0.5);
            p.circle_filled(t * center, t * UNIT * 0.4, Color32::BLACK);

            let bounds = Rect::from_center_size(center, egui::vec2(UNIT, UNIT));
            label(p, t, bounds, name, LabelPlacement::Left);
        }
        for (i, (_addr, name, _ty)) in device.r_nodes().iter().enumerate() {
            // let is_input = matches!(ty, crate::sim::save::IoType::Input);

            let center = egui::pos2(bounds.max.x, bounds.min.y + i as f32 * UNIT + UNIT * 0.5);
            p.circle_filled(t * center, t * UNIT * 0.4, Color32::BLACK);

            let bounds = Rect::from_center_size(center, egui::vec2(UNIT, UNIT));
            label(p, t, bounds, name, LabelPlacement::Right);
        }
    }
}
