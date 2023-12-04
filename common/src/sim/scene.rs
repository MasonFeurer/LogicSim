use crate::graphics::ui::Painter;
use crate::graphics::{Color, PanZoomTransform, Rect, TexCoords};
use crate::input::PtrButton;
use crate::sim::{save, NodeAddr, NodeRegion, Sim, SourceTy};
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

#[derive(Debug, Clone, Default)]
pub struct ExternalNodes {
    pub pos: Vec2,
    pub states: Vec<NodeAddr>,
}

#[derive(Default, Debug)]
pub struct Scene {
    pub transform: PanZoomTransform,
    pub left_nodes: ExternalNodes,
    pub right_nodes: ExternalNodes,
    pub devices: HashMap<SceneId, Box<dyn DeviceImpl>>,
    pub wires: Vec<Wire>,
    pub wire_bundles: Vec<WireBundle>,
}
impl Scene {
    pub fn draw(&mut self, painter: &mut Painter, bg_hovered: &mut bool, sim: &mut Sim) {
        painter.set_transform(self.transform.transform());
        for (id, device) in &mut self.devices {
            let bounds = device.bounds();
            if painter
                .input
                .area_hovered(self.transform.transform().apply2(bounds))
            {
                *bg_hovered = false;
            }

            let anchor = device.get_anchor();
            painter.input.update_drag(
                *id,
                self.transform.transform().apply2(bounds),
                anchor,
                PtrButton::LEFT,
            );
            if let Some(drag) = painter.input.get_drag_full(*id) {
                let offset = drag.press_pos - self.transform.transform().apply(drag.anchor);
                device.move_anchor(
                    self.transform
                        .inv_transform()
                        .apply(painter.input.ptr_pos() - offset),
                );
            }
            device.draw(Some(*id), painter, sim);
        }
        painter.reset_transform();
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
    fn draw(&self, id: Option<SceneId>, painter: &mut Painter, sim: &mut Sim);
    fn bounds(&self) -> Rect;
    fn connection_preview(&self, pos: Vec2) -> Option<NamedConnection>;
}

const NODE_SPACING: f32 = 5.0;
const CHIP_W: f32 = 60.0;
const NODE_SIZE: f32 = 30.0;

#[derive(Debug, Clone)]
pub struct Chip {
    pub region: NodeRegion,
    pub pos: Vec2,
    pub name: String,
    pub orientation: Rotation,
    pub save: Option<save::SaveId>,
    pub l_nodes: Vec<(NodeAddr, String)>,
    pub r_nodes: Vec<(NodeAddr, String)>,
    pub inner_nodes: Vec<NodeAddr>,
}
impl DeviceImpl for Chip {
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
    fn draw(&self, id: Option<SceneId>, painter: &mut Painter, sim: &mut Sim) {
        // let node_color = Color::shade(40).into();
        let node_colors = [Color::rgb(64, 2, 0).into(), Color::rgb(235, 19, 12).into()];

        let max_nodes = self.l_nodes.len().max(self.r_nodes.len()) as f32;
        let size = vec2(
            CHIP_W,
            max_nodes * (NODE_SIZE + NODE_SPACING) + NODE_SPACING,
        );
        let rect = Rect::from_center_size(self.pos, size);

        let chip_color = match id {
            Some(_) => Color::shade(200).into(),
            None => Color::shade(100).into(),
        };
        painter
            .model
            .rounded_rect(rect, 3.0, 20, &TexCoords::WHITE, chip_color);

        let mut y = rect.min.y + NODE_SPACING + NODE_SIZE * 0.5;
        for (addr, _name) in &self.l_nodes {
            let center = vec2(rect.min.x, y);
            let size = NODE_SIZE * 0.5;
            let state = sim.nodes[addr.0 as usize].state();

            let fill_color = node_colors[state as usize];
            let int = painter.interact(Rect::from_circle(center, size));
            if int.clicked && sim.nodes[addr.0 as usize].source().ty() == SourceTy::None {
                sim.nodes[addr.0 as usize].set_state(!state);
            }
            if int.hovered {
                painter.model.circle(center, size + 4.0, 20, int.color);
            }
            painter.model.circle(center, size, 20, fill_color);

            y += NODE_SIZE + NODE_SPACING;
        }

        y = rect.min.y + NODE_SPACING + NODE_SIZE * 0.5;
        for (addr, _name) in &self.r_nodes {
            let center = vec2(rect.max.x, y);
            let size = NODE_SIZE * 0.5;
            let state = sim.nodes[addr.0 as usize].state();

            let fill_color = node_colors[state as usize];
            let int = painter.interact(Rect::from_circle(center, size));
            if int.clicked && sim.nodes[addr.0 as usize].source().ty() == SourceTy::None {
                sim.nodes[addr.0 as usize].set_state(!state);
            }
            if int.hovered {
                painter.model.circle(center, size + 4.0, 20, int.color);
            }
            painter.model.circle(center, size, 20, fill_color);

            y += NODE_SIZE + NODE_SPACING;
        }
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
