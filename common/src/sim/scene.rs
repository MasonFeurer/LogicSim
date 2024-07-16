use crate::sim::save::ChipAttrs;
use crate::sim::{save, NodeAddr, NodeRegion, Sim};
use crate::ui::Transform;

use egui::Rect;

use glam::{vec2, Vec2};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

pub const UNIT: f32 = 20.0;
pub const CHIP_W: f32 = UNIT * 2.0;

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

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Side {
    Left,
    Right,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct ExternalNodes {
    pub pos: Vec2,
    pub states: Vec<(NodeAddr, String)>,
}
impl ExternalNodes {
    pub fn node_info(&self, side: Side, idx: u32) -> Option<NodeInfo> {
        let _pin_dir = match side {
            Side::Left => 1.0,
            Side::Right => -1.0,
        };
        let addr = self.states.get(idx as usize)?.0;
        // TODO
        let x = 0.0;
        let y = 0.0;
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
        // self.r_nodes.pos = vec2(view.max.x - BG_NODE_SIZE, view.min.y + view.height() * 0.3);
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

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BuiltinDeviceTy {
    Button = 0,
    Switch = 1,
    Light = 2,
}
impl BuiltinDeviceTy {
    pub const COUNT: u8 = 3;

    pub fn name(self) -> &'static str {
        match self {
            Self::Button => "Button",
            Self::Switch => "Switch",
            Self::Light => "Light",
        }
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        (v < Self::COUNT).then(|| unsafe { std::mem::transmute(v) })
    }

    pub fn size(self) -> Vec2 {
        match self {
            Self::Button => vec2(20.0, 20.0),
            Self::Switch => vec2(20.0, 20.0),
            Self::Light => vec2(20.0, 20.0),
        }
    }

    pub fn io(self) -> (u8, u8) /* inputs, outputs */ {
        match self {
            Self::Button => (0, 1),
            Self::Switch => (0, 1),
            Self::Light => (1, 0),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BuiltinDevice {
    pub ty: BuiltinDeviceTy,
    pub region: NodeRegion,
    pub pos: Vec2,
    pub rotation: Rotation,
    pub l_nodes: Vec<(NodeAddr, String, save::IoType)>,
    pub r_nodes: Vec<(NodeAddr, String, save::IoType)>,
}
impl BuiltinDevice {
    #[inline(always)]
    pub fn size(&self) -> Vec2 {
        self.ty.size()
    }

    fn node_info(&self, side: Side, idx: u32) -> Option<NodeInfo> {
        let (x, nodes) = match side {
            Side::Left => (self.pos.x - CHIP_W * 0.5, &self.l_nodes),
            Side::Right => (self.pos.x + CHIP_W * 0.5, &self.r_nodes),
        };
        let size = self.size();
        let y = self.pos.y - size.y * 0.5 + (idx as f32) * UNIT + UNIT * 0.5;

        let pos = vec2(x, y);
        let addr = nodes.get(idx as usize)?.0;
        Some(NodeInfo { pos, addr })
    }

    pub fn bounds(&self) -> Rect {
        let size = self.size();
        Rect::from_center_size(
            egui::pos2(self.pos.x, self.pos.y),
            egui::vec2(size.x, size.y),
        )
    }

    fn sim_nodes(&self) -> Vec<NodeAddr> {
        let mut out = Vec::with_capacity(self.l_nodes.len() + self.r_nodes.len());
        for (addr, ..) in &self.l_nodes {
            out.push(*addr);
        }
        for (addr, ..) in &self.r_nodes {
            out.push(*addr);
        }
        out
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
        let y = self.pos.y - size.y * 0.5 + (idx as f32) * UNIT + UNIT * 0.5;

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

    pub fn size(&self) -> Vec2 {
        let max_nodes = self.l_nodes.len().max(self.r_nodes.len()) as f32;
        vec2(CHIP_W, max_nodes * UNIT)
    }

    pub fn bounds(&self) -> Rect {
        let size = self.size();
        Rect::from_center_size(
            egui::pos2(self.pos.x, self.pos.y),
            egui::vec2(size.x, size.y),
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Device {
    Chip(Chip),
    Builtin(BuiltinDevice),
}
impl Device {
    pub fn name(&self) -> &str {
        match self {
            Self::Chip(chip) => &chip.attrs.name,
            Self::Builtin(builtin) => builtin.ty.name(),
        }
    }

    pub fn l_nodes(&self) -> &[(NodeAddr, String, save::IoType)] {
        match self {
            Self::Chip(x) => &x.l_nodes,
            Self::Builtin(x) => &x.l_nodes,
        }
    }

    pub fn r_nodes(&self) -> &[(NodeAddr, String, save::IoType)] {
        match self {
            Self::Chip(x) => &x.r_nodes,
            Self::Builtin(x) => &x.r_nodes,
        }
    }

    pub fn pos(&self) -> Vec2 {
        match self {
            Self::Chip(x) => x.pos,
            Self::Builtin(x) => x.pos,
        }
    }
    pub fn pos_mut(&mut self) -> &mut Vec2 {
        match self {
            Self::Chip(x) => &mut x.pos,
            Self::Builtin(x) => &mut x.pos,
        }
    }

    pub fn bounds(&self) -> Rect {
        match self {
            Self::Chip(x) => x.bounds(),
            Self::Builtin(x) => x.bounds(),
        }
    }

    pub fn size(&self) -> Vec2 {
        match self {
            Self::Chip(x) => x.size(),
            Self::Builtin(x) => x.size(),
        }
    }

    pub fn sim_nodes(&self) -> Vec<NodeAddr> {
        match self {
            Self::Chip(x) => x.sim_nodes(),
            Self::Builtin(x) => x.sim_nodes(),
        }
    }

    pub fn node_info(&self, side: Side, idx: u32) -> Option<NodeInfo> {
        match self {
            Self::Chip(x) => x.node_info(side, idx),
            Self::Builtin(x) => x.node_info(side, idx),
        }
    }
}
impl From<Chip> for Device {
    fn from(x: Chip) -> Device {
        Self::Chip(x)
    }
}
impl From<BuiltinDevice> for Device {
    fn from(x: BuiltinDevice) -> Device {
        Self::Builtin(x)
    }
}
