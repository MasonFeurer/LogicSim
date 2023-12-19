use crate::sim::{self, scene, NodeRegion, TruthTable};
use crate::Id;
use glam::Vec2;

#[derive(Clone, Debug)]
pub enum IoType {
    Input,
    Output,
}

pub struct Library {
    pub tables: Vec<TruthTable>,
    pub chips: Vec<ChipSave>,
}
impl Default for Library {
    fn default() -> Self {
        let and_table = sim::TruthTable {
            num_inputs: 2,
            num_outputs: 1,
            name: "And".into(),
            map: Box::new([0, 0, 0, 1]),
        };
        let not_table = sim::TruthTable {
            num_inputs: 1,
            num_outputs: 1,
            name: "Not".into(),
            map: Box::new([1, 0]),
        };
        let and = ChipSave {
            region_size: 3,
            name: "And".into(),
            scene: None,
            l_nodes: vec![
                ("a".into(), sim::NodeAddr(0), sim::Node::ZERO),
                ("b".into(), sim::NodeAddr(1), sim::Node::ZERO),
            ],
            r_nodes: vec![(
                "out".into(),
                sim::NodeAddr(2),
                sim::Node::new(
                    false,
                    sim::Source::new_table(sim::TruthTableSource {
                        inputs: sim::NodeAddr(0),
                        output: 0,
                        id: sim::TruthTableId(0),
                    }),
                ),
            )],
            inner_nodes: vec![],
        };
        let not = ChipSave {
            region_size: 2,
            name: "Not".into(),
            scene: None,
            l_nodes: vec![("in".into(), sim::NodeAddr(0), sim::Node::ZERO)],
            r_nodes: vec![(
                "out".into(),
                sim::NodeAddr(1),
                sim::Node::new(
                    false,
                    sim::Source::new_table(sim::TruthTableSource {
                        inputs: sim::NodeAddr(0),
                        output: 0,
                        id: sim::TruthTableId(1),
                    }),
                ),
            )],
            inner_nodes: vec![],
        };
        Self {
            tables: vec![and_table, not_table],
            chips: vec![and, not],
        }
    }
}
impl Library {
    pub fn add(&mut self, chips: &[ChipSave]) {
        self.chips.extend(chips.iter().cloned())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SaveId(pub u32);

/// Note: A Node can only hava 1 source,
/// so if a chip writes to one of its external pins than that pin can not
/// be written to externally.

/// A device can not save externally interactive components like lights or buttons.
#[derive(Debug, Clone)]
pub struct ChipSave {
    pub region_size: u32,
    pub name: String,
    pub scene: Option<Id>,
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
