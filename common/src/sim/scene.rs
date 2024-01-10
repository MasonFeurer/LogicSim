use crate::graphics::ui::{Align2, Interaction, Painter};
use crate::graphics::{Color, Rect, Transform, MAIN_ATLAS};
use crate::input::PtrButton;
use crate::sim::save::ChipAttrs;
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
#[repr(u8)]
pub enum Side {
    Left,
    Right,
}

fn draw_node(p: &mut Painter, state: u8, center: Vec2, r: f32) -> Interaction {
    let node_colors = [Color::rgb(64, 2, 0), Color::rgb(235, 19, 12)];

    let mut fill_color = node_colors[(state == 1) as usize];
    let int = p.interact(Rect::from_circle(center, r));

    if int.hovered {
        fill_color = fill_color.darken(40);
    }
    p.model_mut().circle(center, r, 40, fill_color);
    int
}

pub fn draw_wire(
    p: &mut Painter,
    state: u8,
    force_unhovered: bool,
    start: Vec2,
    end: Vec2,
    anchors: &[Vec2],
) -> Interaction {
    let mut points = std::iter::once(start)
        .chain(anchors.iter().copied())
        .chain(std::iter::once(end));
    let mut lines = Vec::new();

    let mut prev = points.next().unwrap();
    for n in points {
        lines.push([prev, n]);
        prev = n;
    }
    let mut hovered = false;
    for line in &lines {
        hovered |= p.interact_line(*line, 10.0).hovered;
    }
    if force_unhovered {
        hovered = false;
    }
    let int = p.interact_hovered(hovered);

    let colors = [Color::rgb(64, 2, 0), Color::rgb(235, 19, 12)];
    let mut color = colors[(state == 1) as usize];
    if hovered {
        color = color.darken(40);
    }

    let mut prev: Option<[Vec2; 2]> = None;
    for idx in 0..lines.len() {
        let mut line = lines[idx];
        let len = (line[1] - line[0]).abs().length();

        if idx > 0 {
            line[0] += (line[1] - line[0]).normalize() * (len * 0.5).min(40.0);
        }
        if idx != lines.len() - 1 {
            line[1] += (line[0] - line[1]).normalize() * (len * 0.5).min(40.0);
        }
        p.model_mut().line(line, 4.0, &MAIN_ATLAS.white, color);
        if let Some(prev) = prev {
            let curve = [prev[1], lines[idx][0], line[0]];
            p.model_mut().curve(curve, 40, 4.0, color);
        }
        prev = Some(line);
    }
    int
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ExternalNodes {
    pub pos: Vec2,
    pub states: Vec<(NodeAddr, String)>,
}
impl ExternalNodes {
    pub fn draw(&mut self, side: Side, p: &mut Painter, sim: &mut Sim, out: &mut SceneOutput) {
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
        let interact_bounds =
            Rect::from_min_size(self.pos - Vec2::Y * w * 0.5, vec2(w, h + w * 0.5));

        if p.interact(interact_bounds).hovered {
            out.item_hovered = true;
        }

        {
            let bg_c = p.style().menu_background;
            let handle_c = p.style().item_press_color;
            p.model_mut()
                .rect(interact_bounds, &MAIN_ATLAS.white, handle_c);
            p.model_mut().rect(bounds, &MAIN_ATLAS.white, bg_c);
        }

        if let Some(new_pos) = p.interact_drag(id, interact_bounds, self.pos, PtrButton::LEFT) {
            self.pos = new_pos;
        }

        // Draw Nodes
        let mut y = self.pos.y + w * 0.5 + NODE_SPACING;
        let x = self.pos.x + w * 0.5;

        for (idx, (addr, name)) in self.states.iter_mut().enumerate() {
            let center = vec2(x, y);
            let pin_center = center + vec2((w * 0.5 + NODE_SIZE * 0.5) * pin_dir, 0.0);
            let state = sim.nodes[addr.0 as usize].state();

            let int = draw_node(p, state, center, w * 0.5);
            let pin_int = draw_node(p, state, pin_center, NODE_SIZE * 0.5);

            if pin_int.hovered {
                out.item_hovered = true;
            }
            if int.clicked {
                out.toggle_node_state = Some(*addr);
            }
            let min = match side {
                Side::Left => vec2(x - 100.0 - w * 0.5, y - w * 0.5),
                Side::Right => vec2(x + w * 0.5, y - w * 0.5),
            };
            let rect = Rect::from_min_size(min, vec2(100.0, w));
            p.text_edit(
                Some(rect),
                Id::new((side as u32 * 256) + idx as u32),
                "name",
                name,
            );
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
            self.states.push((sim.alloc_node(), String::new()));
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
        let addr = self.states.get(idx as usize)?.0;
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
    pub item_hovered: bool,
    pub clicked_node: Option<(NodeIdent, NodeAddr)>,
    pub toggle_node_state: Option<NodeAddr>,
    pub clicked_chip: Option<SceneId>,
    pub rclicked_chip: Option<SceneId>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub sim: Sim,
    pub save_attrs: ChipAttrs,
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

    pub fn draw(&mut self, p: &mut Painter) -> SceneOutput {
        let mut out = SceneOutput::default();

        // Draw External IO
        self.l_nodes.draw(Side::Left, p, &mut self.sim, &mut out);
        self.r_nodes.draw(Side::Right, p, &mut self.sim, &mut out);

        // Draw Devices
        for (id, device) in &mut self.devices {
            let bounds = device.bounds();
            if p.interact(bounds).hovered {
                out.item_hovered = true;
            }

            let anchor = device.get_anchor();
            if let Some(new_pos) = p.interact_drag(*id, bounds, anchor, PtrButton::LEFT) {
                device.move_anchor(new_pos);
            }
            device.draw(Some(*id), p, &mut self.sim, &mut out);
        }

        // Draw Wires
        let mut rm_wire = None;
        for (idx, wire) in self.wires.iter().enumerate() {
            let Some(src) = self.node_info(wire.input) else {
                rm_wire = Some(idx);
                continue;
            };
            let Some(dst) = self.node_info(wire.output) else {
                rm_wire = Some(idx);
                continue;
            };
            let state = self.sim.get_node(src.addr).state();
            let int = draw_wire(p, state, false, src.pos, dst.pos, &wire.anchors);
            if int.rclicked {
                rm_wire = Some(idx);
            }
        }
        if let Some(idx) = rm_wire {
            let wire = self.wires.remove(idx);
            if let Some(dst_info) = self.node_info(wire.output) {
                self.sim.nodes[dst_info.addr.0 as usize].set_source(crate::Source::new_none());
            }
        }
        out
    }

    pub fn rm_wire_by_target(&mut self, output: NodeAddr) -> Option<Wire> {
        for idx in 0..self.wires.len() {
            if let Some(info) = self.node_info(self.wires[idx].output) {
                if info.addr == output {
                    return Some(self.wires.remove(idx));
                }
            }
        }
        None
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
    pub pos: Vec2,
    pub addr: NodeAddr,
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
    pub attrs: ChipAttrs,
    pub region: NodeRegion,
    pub pos: Vec2,
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
    pub fn draw(&self, id: Option<SceneId>, p: &mut Painter, sim: &mut Sim, out: &mut SceneOutput) {
        let size = self.size();
        let rect = Rect::from_center_size(self.pos, size);

        let chip_color = match id {
            Some(_) => self.attrs.color.as_color(),
            None => self.attrs.color.as_color().darken(120),
        };
        p.model_mut()
            .rounded_rect(rect, 10.0, 20, &MAIN_ATLAS.white, chip_color);

        let chip_int = p.interact(rect);
        if chip_int.clicked {
            out.clicked_chip = id;
        }
        if chip_int.rclicked {
            out.rclicked_chip = id;
        }

        let mut y = rect.min.y + NODE_SPACING + NODE_SIZE * 0.5;
        for (idx, (addr, _name, _ty)) in self.l_nodes.iter().enumerate() {
            let center = vec2(rect.min.x, y);
            let state = sim.nodes[addr.0 as usize].state();

            let int = draw_node(p, state, center, NODE_SIZE * 0.5);
            out.item_hovered |= int.hovered;
            if int.clicked {
                if let Some(id) = id {
                    out.clicked_node = Some((NodeIdent::DeviceL(id, idx as u32), *addr));
                }
            }
            if int.rclicked {
                out.toggle_node_state = Some(*addr);
            }
            y += NODE_SIZE + NODE_SPACING;
        }

        y = rect.min.y + NODE_SPACING + NODE_SIZE * 0.5;
        for (idx, (addr, _name, _ty)) in self.r_nodes.iter().enumerate() {
            let center = vec2(rect.max.x, y);
            let state = sim.nodes[addr.0 as usize].state();

            let int = draw_node(p, state, center, NODE_SIZE * 0.5);
            out.item_hovered |= int.hovered;
            if int.clicked {
                if let Some(id) = id {
                    out.clicked_node = Some((NodeIdent::DeviceR(id, idx as u32), *addr));
                }
            }
            if int.rclicked {
                out.toggle_node_state = Some(*addr);
            }
            y += NODE_SIZE + NODE_SPACING;
        }
        let text_size = p.text_size(&self.attrs.name, NODE_SIZE * 0.5);
        p.place_text(
            Rect::from_center_size(
                vec2(self.pos.x, self.pos.y - size.y * 0.5 - text_size.y * 0.5),
                text_size,
            ),
            (&self.attrs.name, text_size),
            p.style().text_color,
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
