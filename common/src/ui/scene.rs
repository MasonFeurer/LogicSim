use crate::save::{IoType, Library};
use crate::sim::scene::{ExternalNodes, NodeIdent, Scene, Side, UNIT};
use crate::sim::{Sim, Source};
use crate::ui::{pages::PageOutput, Transform};

use egui::epaint::QuadraticBezierShape;
use egui::{Align2, Button, Color32, Id, Painter, Rect, Response, Sense, Stroke, Ui};
use glam::{vec2, Vec2};

#[derive(Clone, Copy)]
enum LabelPlacement {
    Top,
    Left,
    Right,
}

fn label(p: &Painter, t: Transform, bounds: Rect, label: &str, place: LabelPlacement) -> Rect {
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
    p.text(t * pos, align2, label, Default::default(), Color32::WHITE)
}

pub fn show_scene<P>(
    ui: &mut Ui,
    library: &Library,
    scene: &mut Scene,
    snap_to_grid: bool,
    show_grid: bool,
    out: &mut PageOutput<P>,
) -> Response {
    scene.sim.update(&library.tables);

    let screen_size = ui.clip_rect().size();
    let screen_size = glam::vec2(screen_size.x, screen_size.y);

    // ----- Handle Pan + Zoom -----
    let rect = ui.available_rect_before_wrap();
    // (return value)
    let rs = ui.interact(rect, Id::from("pan+zoom"), Sense::click_and_drag());

    if let Some(egui::Pos2 { x, y }) = ui.ctx().pointer_latest_pos() {
        let zoom = ui.ctx().input(|state| state.zoom_delta());
        let drag = rs.drag_delta();
        scene.transform.translate(vec2(drag.x, drag.y));
        scene.transform.zoom(vec2(x, y), zoom - 1.0, 0.1..=100.0);
    }

    let t = scene.transform;

    // Draw Grid
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
            ui.painter().line_segment([a, b], stroke);
        }
        for i in 0..(line_count.x.ceil()) as u32 {
            let x = i as f32 * screen_gap + screen_offset.x;
            let a = egui::pos2(x, 0.0);
            let b = egui::pos2(x, screen_size.y);
            ui.painter().line_segment([a, b], stroke);
        }
    }

    // Draw external nodes
    draw_external_nodes(
        ui,
        t,
        &mut scene.l_nodes,
        Side::Left,
        &mut scene.sim,
        snap_to_grid,
        out,
    );
    draw_external_nodes(
        ui,
        t,
        &mut scene.r_nodes,
        Side::Right,
        &mut scene.sim,
        snap_to_grid,
        out,
    );

    // Draw Wires
    let mut rm_wire = None;
    for (idx, wire) in scene.wires.iter().enumerate() {
        let Some(src) = scene.node_info(wire.input) else {
            rm_wire = Some(idx);
            continue;
        };
        let Some(dst) = scene.node_info(wire.output) else {
            rm_wire = Some(idx);
            continue;
        };
        let state = scene.sim.get_node(src.addr).state();
        let (_clicked, rclicked) = draw_wire(
            ui,
            scene.transform,
            state,
            false,
            src.pos,
            dst.pos,
            &wire.anchors,
        );
        if rclicked {
            rm_wire = Some(idx);
        }
    }
    if let Some(idx) = rm_wire {
        let wire = scene.wires.remove(idx);
        if let Some(dst_info) = scene.node_info(wire.output) {
            scene.sim.nodes[dst_info.addr.0 as usize].set_source(Source::new_none());
        }
    }

    // Draw Devices
    let mut rm_device = None;
    for (device_id, device) in &mut scene.devices {
        let bounds = device.bounds();
        let color = Color32::from_gray(200);

        ui.painter().rect_filled(t * bounds, t * 4.0, color);

        let rs = ui.interact(
            t * bounds,
            Id::from("chip").with(device_id),
            Sense::click_and_drag(),
        );
        if rs.secondary_clicked() {
            // remove device from scene
            rm_device = Some(*device_id);
        }

        device.pos_mut().x += t.inv() * rs.drag_delta().x;
        device.pos_mut().y += t.inv() * rs.drag_delta().y;

        if snap_to_grid && rs.drag_stopped() {
            let off = device.size() * 0.5;
            *device.pos_mut() = off + UNIT * ((device.pos() - off) / UNIT).round();
        }

        label(ui.painter(), t, bounds, device.name(), LabelPlacement::Top);

        let colors = [Color32::BLACK, Color32::RED];

        for (i, (addr, name, ty)) in device.l_nodes().iter().enumerate() {
            let node = scene.sim.get_node(*addr);
            let color = colors[node.state() as usize];

            let center = egui::pos2(bounds.min.x, bounds.min.y + i as f32 * UNIT + UNIT * 0.5);
            let bounds = Rect::from_center_size(center, egui::vec2(UNIT, UNIT));

            let rs = ui.interact(
                t * bounds,
                Id::from(format!("{device_id:?}l{i}")),
                Sense::click(),
            );
            if rs.clicked() {
                out.clicked_node = Some((NodeIdent::DeviceL(*device_id, i as u32), *addr, *ty));
            }
            if rs.secondary_clicked() {
                out.rclicked_node = Some((NodeIdent::DeviceL(*device_id, i as u32), *addr, *ty));
            }

            ui.painter()
                .circle_filled(t * center, t * UNIT * 0.4, color);
            label(ui.painter(), t, bounds, name, LabelPlacement::Left);
        }
        for (i, (addr, name, ty)) in device.r_nodes().iter().enumerate() {
            let node = scene.sim.get_node(*addr);
            let color = colors[node.state() as usize];

            let center = egui::pos2(bounds.max.x, bounds.min.y + i as f32 * UNIT + UNIT * 0.5);
            let bounds = Rect::from_center_size(center, egui::vec2(UNIT, UNIT));

            let rs = ui.interact(
                t * bounds,
                Id::from(format!("{device_id:?}r{i}")),
                Sense::click(),
            );
            if rs.clicked() {
                out.clicked_node = Some((NodeIdent::DeviceR(*device_id, i as u32), *addr, *ty));
            }
            if rs.secondary_clicked() {
                out.rclicked_node = Some((NodeIdent::DeviceR(*device_id, i as u32), *addr, *ty));
            }

            ui.painter()
                .circle_filled(t * center, t * UNIT * 0.4, color);
            label(ui.painter(), t, bounds, name, LabelPlacement::Right);
        }
    }
    if let Some(id) = rm_device {
        scene.devices.remove(&id);
    }
    rs
}

pub fn draw_external_nodes<P>(
    ui: &mut Ui,
    t: Transform,
    en: &mut ExternalNodes,
    side: Side,
    sim: &mut Sim,
    snap_to_grid: bool,
    out: &mut PageOutput<P>,
) {
    let id = match side {
        Side::Left => Id::new("l_external"),
        Side::Right => Id::new("r_external"),
    };

    let w = UNIT;
    let h = (en.states.len() as f32) * UNIT + UNIT;

    let bounds = t * Rect::from_min_size(egui::pos2(en.pos.x, en.pos.y), egui::vec2(w, h));
    let interact_bounds =
        Rect::from_min_size(bounds.min, bounds.size() + t * (egui::Vec2::Y * UNIT));

    let rs = ui.interact(interact_bounds, id, Sense::click_and_drag());

    {
        let handle_c = ui.visuals().widgets.active.bg_fill;
        ui.painter().rect_filled(interact_bounds, 0.0, handle_c);
        ui.painter().rect_filled(bounds, 0.0, handle_c);
    }

    en.pos.x += t.inv() * rs.drag_delta().x;
    en.pos.y += t.inv() * rs.drag_delta().y;

    if snap_to_grid && rs.drag_stopped() {
        en.pos = UNIT * (en.pos / UNIT).round();
    }

    let label_ = match side {
        Side::Left => "left IO",
        Side::Right => "right IO",
    };
    label(
        ui.painter(),
        t,
        t.inv() * bounds,
        label_,
        LabelPlacement::Top,
    );

    // Draw Nodes
    let Vec2 { x, mut y } = t * (en.pos + vec2(0.5 * UNIT, 0.5 * UNIT));

    let label_placement = match side {
        Side::Left => LabelPlacement::Left,
        Side::Right => LabelPlacement::Right,
    };

    for (idx, (addr, name)) in en.states.iter_mut().enumerate() {
        let id = id.with(idx.to_string());
        let state = sim.nodes[addr.0 as usize].state();

        let rs = {
            let w = t * UNIT;
            let colors = [Color32::BLACK, Color32::RED];

            let color = colors[(state == 1) as usize];
            let rs = ui.interact(
                Rect::from_center_size(egui::pos2(x, y), egui::vec2(w, w)),
                Id::from(addr.0.to_string()),
                Sense::click(),
            );

            ui.painter().circle_filled(egui::pos2(x, y), w * 0.5, color);
            rs
        };

        let ident = match side {
            Side::Left => NodeIdent::LExternal(idx as u32),
            Side::Right => NodeIdent::RExternal(idx as u32),
        };

        if rs.clicked() {
            out.clicked_node = Some((ident, *addr, IoType::Input));
        }
        if rs.secondary_clicked() {
            out.rclicked_node = Some((ident, *addr, IoType::Input));
        }

        let bounds = Rect::from_center_size(t.inv() * egui::pos2(x, y), egui::vec2(UNIT, UNIT));

        if ui.data(|data| data.get_temp::<bool>(id) == Some(true)) {
            // we are editing the label
            let w = 100.0;
            let field_font_h = ui.ctx().fonts(|fonts| {
                fonts.row_height(ui.style().text_styles.get(&egui::TextStyle::Body).unwrap())
            });
            let field_h = field_font_h + 4.0 /* estimate of the amount of more height an egui::TextEdit has compared to a egui::Label */;
            let field_rect = match side {
                Side::Left => Rect::from_min_size(
                    t * bounds.left_center() - egui::vec2(w, field_h * 0.5),
                    egui::vec2(w, 10.0),
                ),
                Side::Right => Rect::from_min_size(
                    t * bounds.right_center() - egui::vec2(0.0, field_h * 0.5),
                    egui::vec2(w, 10.0),
                ),
            };
            let mut ui = ui.child_ui(field_rect, ui.layout().clone(), None);
            let rs = ui.put(field_rect, egui::TextEdit::singleline(name));
            if rs.lost_focus() {
                ui.data_mut(|data| data.insert_temp(id, false));
            }
        } else {
            // we are not editing the label
            let label_rect = label(ui.painter(), t, bounds, name, label_placement);
            if ui.interact(label_rect, id, Sense::click()).clicked() {
                ui.data_mut(|data| data.insert_temp(id, true));
            }
        }
        y += t * UNIT;
    }

    // Draw [+] Button
    let button = Button::new("+").rounding(t * UNIT * 0.5);
    let rect = Rect::from_center_size(egui::pos2(x, y), t * egui::vec2(UNIT, UNIT));
    let mut ui = ui.child_ui(rect, ui.layout().clone(), None);
    let rs = ui.put(rect, button);
    if rs.clicked() {
        en.states.push((sim.alloc_node(), String::from("unnamed")));
    }
    if rs.secondary_clicked() {
        _ = en.states.pop();
    }
}

pub fn draw_wire(
    ui: &mut Ui,
    t: Transform,
    state: u8,
    force_unhovered: bool,
    start: Vec2,
    end: Vec2,
    anchors: &[Vec2],
) -> (bool, bool) {
    use crate::ui::line_contains_point;

    let mut points = std::iter::once(start)
        .chain(anchors.iter().copied())
        .chain(std::iter::once(end));
    let mut lines = Vec::new();

    let ptr = ui.ctx().pointer_latest_pos().unwrap_or(egui::Pos2::ZERO);
    let ptr = vec2(ptr.x, ptr.y);
    let p = ui.painter();

    let mut prev = points.next().unwrap();
    for n in points {
        lines.push((prev, n));
        prev = n;
    }

    let hovered = !force_unhovered
        && lines
            .iter()
            .any(|line| line_contains_point(*line, 10.0, t.inv() * ptr));
    let hovered = hovered
        && ui
            .ctx()
            .interaction_snapshot(|ss| ss.contains_pointer.len() <= 2);

    let colors = [Color32::from_rgb(64, 2, 0), Color32::from_rgb(235, 19, 12)];
    let mut color = colors[(state == 1) as usize];
    if hovered {
        // color = color.darken(40);
        color = Color32::BLACK;
    }

    let stroke = Stroke::new(2.0, color);

    let mut prev: Option<(Vec2, Vec2)> = None;
    for idx in 0..lines.len() {
        let mut line = lines[idx];
        let len = (line.1 - line.0).abs().length();

        if idx > 0 {
            line.0 += (line.1 - line.0).normalize() * (len * 0.5).min(40.0);
        }
        if idx != lines.len() - 1 {
            line.1 += (line.0 - line.1).normalize() * (len * 0.5).min(40.0);
        }

        p.line_segment(
            [
                t * egui::pos2(line.0.x, line.0.y),
                t * egui::pos2(line.1.x, line.1.y),
            ],
            stroke,
        );
        if let Some(prev) = prev {
            let points = [prev.1, lines[idx].0, line.0];
            let points = [
                t * egui::pos2(points[0].x, points[0].y),
                t * egui::pos2(points[1].x, points[1].y),
                t * egui::pos2(points[2].x, points[2].y),
            ];

            p.add(QuadraticBezierShape {
                points,
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: stroke.into(),
            });
        }
        prev = Some(line);
    }
    let lclicked = ui.input(|state| {
        state.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::PointerButton {
                    pressed: true,
                    button: egui::PointerButton::Primary,
                    ..
                }
            )
        })
    });
    let rclicked = ui.input(|state| {
        state.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::PointerButton {
                    pressed: true,
                    button: egui::PointerButton::Secondary,
                    ..
                }
            )
        })
    });
    (hovered && lclicked, hovered && rclicked)
}
