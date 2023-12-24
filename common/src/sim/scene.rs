use crate::graphics::ui::{Align2, Interaction, Painter};
use crate::graphics::{Color, Rect, Transform, MAIN_ATLAS};
use crate::input::PtrButton;
use crate::sim::{save, NodeAddr, NodeRegion, Sim};
use crate::Id;

use glam::{vec2, Vec2};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

pub type SceneId = crate::Id;

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub enum Rotation {
    #[default]
    A0,
    A90,
    A180,
    A270,
}
impl Rotation {
    pub fn next(self) -> Self {
        match self {
            Self::A0 => Self::A270,
            Self::A90 => Self::A0,
            Self::A180 => Self::A90,
            Self::A270 => Self::A180,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum NodeIdent {
    LExternal(u32),
    RExternal(u32),
    DeviceL(SceneId, u32),
    DeviceR(SceneId, u32),
}

// note: it's not unnaceptable for the app to allow you to end a wire connection on the output of a chip.
// It should just not be very useful, like back-feeding a voltage into the output of a read breadboard component.
// Because from the outside, a node doesn't exactly say weather it can be written to or not.
#[derive(Clone, Serialize, Deserialize)]
pub struct Wire {
    pub input: NodeIdent,
    pub output: NodeIdent,
    pub anchors: Vec<Vec2>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WireBundle {
    pub inputs: Vec<NodeIdent>,
    pub outputs: Vec<NodeIdent>,
    pub anchors: Vec<Vec2>,
}

const NODE_SPACING: f32 = 5.0;
const CHIP_W: f32 = 100.0;
const NODE_SIZE: f32 = 30.0;
const BG_NODE_SIZE: f32 = 50.0;

#[derive(Clone, Copy)]
pub enum Side {
    Left,
    Right,
}

fn draw_node(p: &mut Painter, state: bool, center: Vec2, r: f32) -> Interaction {
    let node_colors = [Color::rgb(64, 2, 0).into(), Color::rgb(235, 19, 12).into()];

    let fill_color = node_colors[state as usize];
    let int = p.interact(Rect::from_circle(center, r));
    if int.hovered {
        p.model_mut().circle(center, r + 4.0, 20, int.color);
    }
    p.model_mut().circle(center, r, 20, fill_color);
    int
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ExternalNodes {
    pub pos: Vec2,
    pub states: Vec<NodeAddr>,
}
impl ExternalNodes {
    pub fn draw(
        &mut self,
        side: Side,
        t: Transform,
        p: &mut Painter,
        bg_hovered: &mut bool,
        sim: &mut Sim,
        out: &mut SceneOutput,
    ) {
        let id = match side {
            Side::Left => Id::new("l_external"),
            Side::Right => Id::new("r_external"),
        };
        let pin_dir = match side {
            Side::Left => 1.0,
            Side::Right => -1.0,
        };
        let w = BG_NODE_SIZE;
        let h = (self.states.len() as f32 + 1.0) * (NODE_SPACING + w) + NODE_SPACING;
        let bounds = Rect::from_min_size(self.pos, vec2(w, h));
        let handle_bounds = Rect::from_min_size(self.pos - Vec2::Y * w * 0.5, vec2(w, w * 0.5));

        if p.input().area_hovered(t * bounds) || p.input().area_hovered(t * handle_bounds) {
            *bg_hovered = false;
        }

        {
            let bg = p.style().menu_background;
            let handle = p.style().item_press_color;
            p.model_mut().rect(bounds, &MAIN_ATLAS.white, bg);
            p.model_mut().rect(handle_bounds, &MAIN_ATLAS.white, handle);
        }

        if let Some(new_pos) = p.interact_drag(id, handle_bounds, self.pos, PtrButton::LEFT) {
            self.pos = new_pos;
        }

        // Draw Nodes
        let mut y = self.pos.y + w * 0.5 + NODE_SPACING;
        let x = self.pos.x + w * 0.5;

        for (idx, addr) in self.states.iter().enumerate() {
            let center = vec2(x, y);
            let pin_center = center + vec2((w * 0.5 + NODE_SIZE * 0.5) * pin_dir, 0.0);
            let state = sim.nodes[addr.0 as usize].state();

            let int = draw_node(p, state, center, w * 0.5);
            let pin_int = draw_node(p, state, pin_center, NODE_SIZE * 0.5);

            if int.clicked {
                out.toggle_node_state = Some(*addr);
            }
            if pin_int.clicked {
                out.clicked_node = Some(match side {
                    Side::Left => (NodeIdent::LExternal(idx as u32), *addr),
                    Side::Right => (NodeIdent::RExternal(idx as u32), *addr),
                });
            }
            y += w + NODE_SPACING;
        }

        // Draw [+] Button
        let button_int = p.circle_button(Some(vec2(x, y)), Some(w), "+");
        if button_int.clicked {
            self.states.push(sim.alloc_node());
        }
        if button_int.rclicked {
            _ = self.states.pop();
        }
    }

    pub fn node_info(&self, side: Side, idx: u32) -> Option<NodeInfo> {
        let pin_dir = match side {
            Side::Left => 1.0,
            Side::Right => -1.0,
        };
        let addr = *self.states.get(idx as usize)?;
        let y = self.pos.y
            + BG_NODE_SIZE * 0.5
            + NODE_SPACING
            + (NODE_SPACING + BG_NODE_SIZE) * idx as f32;
        let x = self.pos.x + BG_NODE_SIZE * 0.5 + (BG_NODE_SIZE * 0.5 + NODE_SIZE * 0.5) * pin_dir;
        let pos = vec2(x, y);
        Some(NodeInfo { addr, pos })
    }
}

#[derive(Default, Clone)]
pub struct SceneOutput {
    pub clicked_node: Option<(NodeIdent, NodeAddr)>,
    pub toggle_node_state: Option<NodeAddr>,
    pub clicked_output: Option<NodeAddr>,
    pub clicked_input: Option<NodeAddr>,
    pub clicked_chip: Option<SceneId>,
    pub rclicked_chip: Option<SceneId>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub sim: Sim,
    pub transform: Transform,
    pub l_nodes: ExternalNodes,
    pub r_nodes: ExternalNodes,
    pub devices: HashMap<SceneId, Device>,
    pub wires: Vec<Wire>,
    pub wire_bundles: Vec<WireBundle>,
}
impl Scene {
    pub fn clear(&mut self) {
        self.l_nodes.states.clear();
        self.r_nodes.states.clear();
        self.devices.clear();
        self.wires.clear();
        self.wire_bundles.clear();
        self.sim.clear();
    }

    pub fn init(&mut self, view: Rect) {
        self.l_nodes.pos = vec2(view.min.x, view.min.y + view.height() * 0.3);
        self.r_nodes.pos = vec2(view.max.x - BG_NODE_SIZE, view.min.y + view.height() * 0.3);
    }

    pub fn draw(&mut self, p: &mut Painter, bg_hovered: &mut bool) -> SceneOutput {
        let mut out = SceneOutput::default();

        // Draw External IO
        self.l_nodes.draw(
            Side::Left,
            self.transform,
            p,
            bg_hovered,
            &mut self.sim,
            &mut out,
        );
        self.r_nodes.draw(
            Side::Right,
            self.transform,
            p,
            bg_hovered,
            &mut self.sim,
            &mut out,
        );

        // Draw Devices
        for (id, device) in &mut self.devices {
            let bounds = device.bounds();
            if p.input().area_hovered(self.transform * bounds) {
                *bg_hovered = false;
            }

            let anchor = device.get_anchor();
            if let Some(new_pos) = p.interact_drag(*id, bounds, anchor, PtrButton::LEFT) {
                device.move_anchor(new_pos);
            }
            device.draw(Some(*id), p, &mut self.sim, &mut out);
        }

        // Draw Wires
        let colors = [Color::rgb(64, 2, 0).into(), Color::rgb(235, 19, 12).into()];
        for wire in &self.wires {
            let Some(src) = self.node_info(wire.input) else {
                continue;
            };
            let Some(dst) = self.node_info(wire.output) else {
                continue;
            };
            let color = colors[self.sim.get_node(src.addr).state() as usize];
            p.model_mut()
                .line([src.pos, dst.pos], 4.0, &MAIN_ATLAS.white, color);
        }
        out
    }

    pub fn node_info(&self, ident: NodeIdent) -> Option<NodeInfo> {
        match ident {
            NodeIdent::LExternal(idx) => self.l_nodes.node_info(Side::Left, idx),
            NodeIdent::RExternal(idx) => self.r_nodes.node_info(Side::Right, idx),
            NodeIdent::DeviceL(id, idx) => self.devices.get(&id)?.node_info(Side::Left, idx),
            NodeIdent::DeviceR(id, idx) => self.devices.get(&id)?.node_info(Side::Right, idx),
        }
    }

    pub fn add_device(&mut self, device: impl Into<Device>) {
        self.devices
            .insert(SceneId::new(fastrand::u32(..)), device.into());
    }
}

pub struct NodeInfo {
    pos: Vec2,
    addr: NodeAddr,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Device {
    Chip(Chip),
}
impl Device {
    pub fn draw(&self, id: Option<SceneId>, p: &mut Painter, sim: &mut Sim, out: &mut SceneOutput) {
        match self {
            Self::Chip(chip) => chip.draw(id, p, sim, out),
        }
    }

    pub fn move_anchor(&mut self, pos: Vec2) {
        match self {
            Self::Chip(chip) => chip.move_anchor(pos),
        }
    }

    pub fn bounds(&self) -> Rect {
        match self {
            Self::Chip(chip) => chip.bounds(),
        }
    }

    pub fn get_anchor(&self) -> Vec2 {
        match self {
            Self::Chip(chip) => chip.get_anchor(),
        }
    }

    pub fn sim_nodes(&self) -> Vec<NodeAddr> {
        match self {
            Self::Chip(chip) => chip.sim_nodes(),
        }
    }

    pub fn node_info(&self, side: Side, idx: u32) -> Option<NodeInfo> {
        match self {
            Self::Chip(chip) => chip.node_info(side, idx),
        }
    }
}
impl From<Chip> for Device {
    fn from(c: Chip) -> Device {
        Self::Chip(c)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Chip {
    pub region: NodeRegion,
    pub pos: Vec2,
    pub name: String,
    pub color: Color,
    pub rotation: Rotation,
    pub save: Option<usize>,
    pub l_nodes: Vec<(NodeAddr, String, save::IoType)>,
    pub r_nodes: Vec<(NodeAddr, String, save::IoType)>,
    pub inner_nodes: Vec<NodeAddr>,
}
impl Chip {
    fn node_info(&self, side: Side, idx: u32) -> Option<NodeInfo> {
        let (x, nodes) = match side {
            Side::Left => (self.pos.x - CHIP_W * 0.5, &self.l_nodes),
            Side::Right => (self.pos.x + CHIP_W * 0.5, &self.r_nodes),
        };
        let size = self.size();
        let y = self.pos.y - size.y * 0.5
            + NODE_SPACING
            + NODE_SIZE * 0.5
            + (NODE_SIZE + NODE_SPACING) * idx as f32;
        let pos = vec2(x, y);
        let addr = nodes.get(idx as usize)?.0;
        Some(NodeInfo { pos, addr })
    }

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

    pub fn size(&self) -> Vec2 {
        let max_nodes = self.l_nodes.len().max(self.r_nodes.len()) as f32;
        vec2(
            CHIP_W,
            max_nodes * (NODE_SIZE + NODE_SPACING) + NODE_SPACING,
        )
    }
    pub fn draw(
        &self,
        id: Option<SceneId>,
        ui: &mut Painter,
        sim: &mut Sim,
        out: &mut SceneOutput,
    ) {
        let node_colors = [Color::rgb(64, 2, 0).into(), Color::rgb(235, 19, 12).into()];

        let size = self.size();
        let rect = Rect::from_center_size(self.pos, size);

        let chip_color = match id {
            Some(_) => self.color,
            None => self.color.darken(120),
        };
        ui.model_mut()
            .rounded_rect(rect, 10.0, 20, &MAIN_ATLAS.white, chip_color.into());

        let chip_int = ui.interact(rect);
        if chip_int.clicked {
            out.clicked_chip = id;
        }
        if chip_int.rclicked {
            out.rclicked_chip = id;
        }

        let mut y = rect.min.y + NODE_SPACING + NODE_SIZE * 0.5;
        for (idx, (addr, _name, ty)) in self.l_nodes.iter().enumerate() {
            let center = vec2(rect.min.x, y);
            let size = NODE_SIZE * 0.5;
            let state = sim.nodes[addr.0 as usize].state();

            let fill_color = node_colors[state as usize];
            let int = ui.interact(Rect::from_circle(center, size));
            if int.clicked {
                if let Some(id) = id {
                    out.clicked_node = Some((NodeIdent::DeviceL(id, idx as u32), *addr));
                }
            }
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
        for (idx, (addr, _name, ty)) in self.r_nodes.iter().enumerate() {
            let center = vec2(rect.max.x, y);
            let size = NODE_SIZE * 0.5;
            let state = sim.nodes[addr.0 as usize].state();

            let fill_color = node_colors[state as usize];
            let int = ui.interact(Rect::from_circle(center, size));
            if int.clicked {
                if let Some(id) = id {
                    out.clicked_node = Some((NodeIdent::DeviceR(id, idx as u32), *addr));
                }
            }
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
        let text_size = ui.text_size(&self.name, NODE_SIZE * 0.5);
        ui.place_text(
            Rect::from_center_size(
                vec2(self.pos.x, self.pos.y - size.y * 0.5 - text_size.y * 0.5),
                text_size,
            ),
            (&self.name, text_size),
            ui.style().text_color,
            Align2::CENTER,
        );
    }

    pub fn bounds(&self) -> Rect {
        let size = self.size();
        Rect::from_center_size(self.pos, size)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Button {
    pub pos: Vec2,
    pub state: NodeAddr,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Light {
    pub color: Color,
    pub pos: Vec2,
    pub state: NodeAddr,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Bus {
    pub pos: Vec2,
    pub height: f32,
    pub reads: Vec<NodeAddr>,
    pub state: NodeAddr,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SegmentDisplay {
    pub pos: Vec2,
    pub inputs: [NodeAddr; 7],
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MatrixDisplay {
    pub pos: Vec2,
    pub inputs: [NodeAddr; 7],
}
