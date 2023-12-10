use crate::sim::{self, scene, scene::Scene, NodeRegion};
use glam::Vec2;

#[derive(Clone, Debug)]
pub enum IoType {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy)]
pub struct SaveId(pub u32);

/// Note: A Node can only hava 1 source,
/// so if a chip writes to one of its external pins than that pin can not
/// be written to externally.

/// A device can not save externally interactive components like lights or buttons.
#[derive(Debug)]
pub struct ChipSave {
    pub region_size: u32,
    pub name: String,
    pub scene: Option<Scene>,
    pub l_nodes: Vec<(String, sim::NodeAddr, sim::Node)>,
    pub r_nodes: Vec<(String, sim::NodeAddr, sim::Node)>,
    pub inner_nodes: Vec<(sim::NodeAddr, sim::Node)>,
}
impl ChipSave {
    pub fn preview(&self, pos: Vec2, orientation: scene::Rotation) -> scene::Chip {
        fn io_ty(node: &sim::Node) -> IoType {
            match node.source().ty() {
                sim::SourceTy::None => IoType::Input,
                _ => IoType::Output,
            }
        }

        let l_nodes: Vec<_> = self
            .l_nodes
            .iter()
            .map(|(name, _, state)| (sim::NodeAddr(0), name.clone(), io_ty(state)))
            .collect();
        let r_nodes: Vec<_> = self
            .r_nodes
            .iter()
            .map(|(name, _, state)| (sim::NodeAddr(0), name.clone(), io_ty(state)))
            .collect();

        scene::Chip {
            region: NodeRegion::default(),
            pos,
            name: self.name.clone(),
            orientation,
            save: None,
            l_nodes,
            r_nodes,
            inner_nodes: vec![],
        }
    }
}
