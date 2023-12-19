use crate::graphics::ui::{Align2, Painter};
use crate::graphics::{Color, Rect, Transform, MAIN_ATLAS};
use crate::input::PtrButton;
use crate::sim::{save, NodeAddr, NodeRegion, Sim};
use crate::Id;
use glam::{vec2, Vec2};
use std::collections::HashMap;
use std::fmt::Debug;

pub type SceneId = crate::Id;

#[derive(Debug, Clone, Copy)]
pub enum Rotation {
    Rot0,
    Rot90,
    Rot180,
    Rot270,
}
#[derive(Debug, Clone, Copy)]
pub struct SceneColor(pub u32);

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: u32,
    pub pos: Vec2,
    pub size: f32,
    pub state: NodeAddr,
}

#[derive(Debug, Clone)]
pub struct DeviceConnection(pub SceneId, pub Connection);

#[derive(Debug, Clone)]
pub struct NamedConnection<'a>(pub &'a str, pub Connection);

const NODE_SPACING: f32 = 5.0;
const CHIP_W: f32 = 100.0;
const NODE_SIZE: f32 = 30.0;
const BG_NODE_SIZE: f32 = 50.0;

#[derive(Debug, Clone, Default)]
pub struct ExternalNodes {
    pub pos: Vec2,
    pub states: Vec<NodeAddr>,
}
impl ExternalNodes {
    pub fn draw(
        &mut self,
        id: Id,
        t: Transform,
        ui: &mut Painter,
        bg_hovered: &mut bool,
        sim: &mut Sim,
        out: &mut SceneOutput,
    ) {
        let w = BG_NODE_SIZE;
        let header_h = ui.style().item_size.y;
        let h = (self.states.len() as f32 + 1.0) * (NODE_SPACING + w) + NODE_SPACING + header_h;
        let bounds = Rect::from_min_size(self.pos, vec2(w, h));
        if ui.input().area_hovered(t * bounds) {
            *bg_hovered = false;
        }
        let bg = ui.style().menu_background;
        ui.model_mut().rect(bounds, &MAIN_ATLAS.white, bg);
        let header_rect = Rect::from_min_size(bounds.min, vec2(w, ui.style().item_size.y));
        let header_text_size = ui.text_size("IO", w);
        ui.place_text(
            header_rect,
            ("IO", header_text_size),
            ui.style().text_color,
            Align2::CENTER,
        );

        ui.input_mut()
            .update_drag(id, t * bounds, self.pos, PtrButton::LEFT);
        if let Some(drag) = ui.input().get_drag_full(id) {
            let offset = drag.press_pos - t * drag.anchor;
            self.pos = t.inv() * (ui.input().ptr_pos() - offset);
        }
        // Draw nodes + Add Node Button
        {
            let node_colors = [Color::rgb(64, 2, 0).into(), Color::rgb(235, 19, 12).into()];
            let mut y = bounds.min.y + header_h + w * 0.5 + NODE_SPACING;
            let x = bounds.min.x + w * 0.5;

            for addr in &self.states {
                let center = vec2(x, y);
                let state = sim.nodes[addr.0 as usize].state();

                let fill_color = node_colors[state as usize];
                let int = ui.interact(Rect::from_circle(center, w * 0.5));
                if int.clicked {
                    out.clicked_output = Some(*addr);
                    // if sim.nodes[addr.0 as usize].source().ty() == SourceTy::None {
                    //     out.clicked_input = Some(*addr);
                    // } else {
                    //     out.clicked_output = Some(*addr);
                    // }
                } else if int.rclicked {
                    out.clicked_input = Some(*addr);
                }
                if int.hovered {
                    ui.model_mut().circle(center, w * 0.5 + 4.0, 20, int.color);
                }
                ui.model_mut().circle(center, w * 0.5, 20, fill_color);

                y += w + NODE_SPACING;
            }

            let button_int = ui.circle_button(Some(vec2(x, y)), Some(w), "+");
            if button_int.clicked {
                self.states.push(sim.alloc_node());
            }
            if button_int.rclicked {
                _ = self.states.pop();
            }
        }
    }
}

#[derive(Default, Clone)]
pub struct SceneOutput {
    pub clicked_output: Option<NodeAddr>,
    pub clicked_input: Option<NodeAddr>,
    pub clicked_chip: Option<SceneId>,
}

#[derive(Default, Debug)]
pub struct Scene {
    pub transform: Transform,
    pub left_nodes: ExternalNodes,
    pub right_nodes: ExternalNodes,
    pub devices: HashMap<SceneId, Box<dyn DeviceImpl>>,
    pub wires: Vec<Wire>,
    pub wire_bundles: Vec<WireBundle>,
}
impl Scene {
    pub fn clear(&mut self) {
        self.left_nodes.states.clear();
        self.right_nodes.states.clear();
        self.devices.clear();
        self.wires.clear();
        self.wire_bundles.clear();
    }

    pub fn init(&mut self, view: Rect) {
        self.left_nodes.pos = vec2(view.min.x, view.min.y + view.height() * 0.3);
        self.right_nodes.pos = vec2(view.max.x - BG_NODE_SIZE, view.min.y + view.height() * 0.3);
    }

    pub fn draw(&mut self, ui: &mut Painter, bg_hovered: &mut bool, sim: &mut Sim) -> SceneOutput {
        let mut out = SceneOutput::default();

        self.left_nodes.draw(
            Id::new("lio"),
            self.transform,
            ui,
            bg_hovered,
            sim,
            &mut out,
        );
        self.right_nodes.draw(
            Id::new("rio"),
            self.transform,
            ui,
            bg_hovered,
            sim,
            &mut out,
        );

        for (id, device) in &mut self.devices {
            let bounds = device.bounds();
            if ui.input().area_hovered(self.transform * bounds) {
                *bg_hovered = false;
            }

            let anchor = device.get_anchor();
            ui.input_mut()
                .update_drag(*id, self.transform * bounds, anchor, PtrButton::LEFT);
            if let Some(drag) = ui.input().get_drag_full(*id) {
                let offset = drag.press_pos - self.transform * drag.anchor;
                device.move_anchor(self.transform.inv() * (ui.input().ptr_pos() - offset));
            }
            device.draw(Some(*id), ui, sim, &mut out);
        }
        out
    }

    pub fn add_device(&mut self, device: impl DeviceImpl + 'static) {
        self.devices
            .insert(SceneId::new(fastrand::u32(..)), Box::new(device));
    }
}

pub trait DeviceImpl: Debug {
    fn get_anchor(&self) -> Vec2;
    fn move_anchor(&mut self, pos: Vec2);
    fn size(&self) -> Vec2;
    fn draw(&self, id: Option<SceneId>, ui: &mut Painter, sim: &mut Sim, out: &mut SceneOutput);
    fn bounds(&self) -> Rect;
    fn connection_preview(&self, pos: Vec2) -> Option<NamedConnection>;
    fn sim_nodes(&self) -> Vec<NodeAddr>;
}

#[derive(Debug, Clone)]
pub struct Chip {
    pub region: NodeRegion,
    pub pos: Vec2,
    pub name: String,
    pub orientation: Rotation,
    pub save: Option<save::SaveId>,
    pub l_nodes: Vec<(NodeAddr, String, save::IoType)>,
    pub r_nodes: Vec<(NodeAddr, String, save::IoType)>,
    pub inner_nodes: Vec<NodeAddr>,
}
impl DeviceImpl for Chip {
    fn sim_nodes(&self) -> Vec<NodeAddr> {
        let mut out =
            Vec::with_capacity(self.l_nodes.len() + self.r_nodes.len() + self.inner_nodes.len());
        for (addr, ..) in &self.l_nodes {
            out.push(*addr);
        }
        for (addr, ..) in &self.r_nodes {
            out.push(*addr);
        }
        for addr in &self.inner_nodes {
            out.push(*addr);
        }
        out
    }

    fn get_anchor(&self) -> Vec2 {
        self.pos
    }
    fn move_anchor(&mut self, pos: Vec2) {
        self.pos = pos;
    }

    fn size(&self) -> Vec2 {
        let max_nodes = self.l_nodes.len().max(self.r_nodes.len()) as f32;
        vec2(
            CHIP_W,
            max_nodes * (NODE_SIZE + NODE_SPACING) + NODE_SPACING,
        )
    }
    fn draw(&self, id: Option<SceneId>, ui: &mut Painter, sim: &mut Sim, out: &mut SceneOutput) {
        // let node_color = Color::shade(40).into();
        let node_colors = [Color::rgb(64, 2, 0).into(), Color::rgb(235, 19, 12).into()];

        let max_nodes = self.l_nodes.len().max(self.r_nodes.len()) as f32;
        let size = vec2(
            CHIP_W,
            max_nodes * (NODE_SIZE + NODE_SPACING) + NODE_SPACING,
        );
        let rect = Rect::from_center_size(self.pos, size);

        let chip_color = match id {
            Some(_) => ui.style().item_color,
            None => Color::shade(125).into(),
        };
        ui.model_mut()
            .rounded_rect(rect, 3.0, 20, &MAIN_ATLAS.white, chip_color);

        let chip_int = ui.interact(rect);
        if chip_int.clicked {
            out.clicked_chip = id;
        }

        let mut y = rect.min.y + NODE_SPACING + NODE_SIZE * 0.5;
        for (addr, _name, ty) in &self.l_nodes {
            let center = vec2(rect.min.x, y);
            let size = NODE_SIZE * 0.5;
            let state = sim.nodes[addr.0 as usize].state();

            let fill_color = node_colors[state as usize];
            let int = ui.interact(Rect::from_circle(center, size));
            if int.clicked {
                match ty {
                    &save::IoType::Input => out.clicked_input = Some(*addr),
                    &save::IoType::Output => out.clicked_output = Some(*addr),
                }
            }
            if int.hovered {
                ui.model_mut().circle(center, size + 4.0, 20, int.color);
            }
            ui.model_mut().circle(center, size, 20, fill_color);

            y += NODE_SIZE + NODE_SPACING;
        }

        y = rect.min.y + NODE_SPACING + NODE_SIZE * 0.5;
        for (addr, _name, ty) in &self.r_nodes {
            let center = vec2(rect.max.x, y);
            let size = NODE_SIZE * 0.5;
            let state = sim.nodes[addr.0 as usize].state();

            let fill_color = node_colors[state as usize];
            let int = ui.interact(Rect::from_circle(center, size));
            if int.clicked {
                match ty {
                    &save::IoType::Input => out.clicked_input = Some(*addr),
                    &save::IoType::Output => out.clicked_output = Some(*addr),
                }
            }
            if int.hovered {
                ui.model_mut().circle(center, size + 4.0, 20, int.color);
            }
            ui.model_mut().circle(center, size, 20, fill_color);

            y += NODE_SIZE + NODE_SPACING;
        }
        let text_size = ui.text_size(&self.name, size.y * 0.5);
        ui.place_text(
            Rect::from_center_size(self.pos, size * 0.5),
            (&self.name, text_size),
            ui.style().text_color,
            Align2::CENTER,
        );
    }

    fn bounds(&self) -> Rect {
        let size = self.size();
        Rect::from_center_size(self.pos, size)
    }
    fn connection_preview(&self, pos: Vec2) -> Option<NamedConnection> {
        let bounds = self.bounds();

        if pos.x <= bounds.min.x + NODE_SIZE * 0.5 {
            let offset_y = pos.y - bounds.min.y;
            let node_idx = (offset_y / bounds.height()).floor() as i32;
            if node_idx < 0 || node_idx >= self.l_nodes.len() as i32 {
                return None;
            }
            Some(NamedConnection(
                &self.l_nodes[node_idx as usize].1,
                Connection {
                    state: self.l_nodes[node_idx as usize].0,
                    id: node_idx as u32,
                    pos: vec2(bounds.min.x, bounds.min.y + node_idx as f32 * NODE_SIZE),
                    size: NODE_SIZE,
                },
            ))
        } else if pos.x >= bounds.max.x - NODE_SIZE * 0.5 {
            let offset_y = pos.y - bounds.min.y;
            let node_idx = (offset_y / bounds.height()).floor() as i32;
            if node_idx < 0 || node_idx >= self.r_nodes.len() as i32 {
                return None;
            }
            Some(NamedConnection(
                &self.r_nodes[node_idx as usize].1,
                Connection {
                    state: self.r_nodes[node_idx as usize].0,
                    id: node_idx as u32,
                    pos: vec2(bounds.max.x, bounds.min.y + node_idx as f32 * NODE_SIZE),
                    size: NODE_SIZE,
                },
            ))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct Button {
    pub pos: Vec2,
    pub state: NodeAddr,
}

#[derive(Debug, Clone)]
pub struct Light {
    pub color: SceneColor,
    pub pos: Vec2,
    pub state: NodeAddr,
}

#[derive(Debug, Clone)]
pub struct Bus {
    pub pos: Vec2,
    pub height: f32,
    pub reads: Vec<NodeAddr>,
    pub state: NodeAddr,
}

#[derive(Debug, Clone)]
pub struct SevenSegDisplayProto {
    pub pos: Vec2,
    pub inputs: [NodeAddr; 7],
}

#[derive(Debug, Clone)]
pub struct SevenSegDisplay {
    pub pos: Vec2,
    pub inputs: [NodeAddr; 4],
}

#[derive(Debug, Clone)]
pub struct Wire {
    pub input: DeviceConnection,
    pub output: DeviceConnection,
    pub anchors: Vec<Vec2>,
}

#[derive(Debug, Clone)]
pub struct WireBundle {
    pub inputs: Vec<DeviceConnection>,
    pub outputs: Vec<DeviceConnection>,
    pub anchors: Vec<Vec2>,
}
