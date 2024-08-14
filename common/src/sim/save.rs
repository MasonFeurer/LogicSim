use crate::sim::{self, scene, NodeRegion, TruthTable, TruthTableId};
use egui::Color32 as Color;
use glam::Vec2;
use serde::{Deserialize, Serialize};

pub type SaveId = crate::Id;

pub fn create_chip_from_scene(scene: &crate::sim::scene::Scene) -> ChipSave {
    let region_size = scene.sim.next_region;
    let l_nodes = scene
        .l_nodes
        .states
        .iter()
        .map(|(addr, name)| (name.clone(), *addr, scene.sim.get_node(*addr)))
        .collect();
    let r_nodes = scene
        .r_nodes
        .states
        .iter()
        .map(|(addr, name)| (name.clone(), *addr, scene.sim.get_node(*addr)))
        .collect();
    let mut inner_nodes = Vec::new();
    for device in scene.devices.values() {
        for addr in device.sim_nodes() {
            inner_nodes.push((addr, scene.sim.get_node(addr)));
        }
    }
    ChipSave {
        attrs: scene.save_attrs.clone(),
        region_size,
        builtin: false,
        scene: Some(scene.clone()),
        l_nodes,
        r_nodes,
        inner_nodes,
    }
}

pub fn create_basic_chip(
    table_id: TruthTableId,
    name: &str,
    inputs: &[&str],
    outputs: &[&str],
    map: Box<[u64]>,
) -> (TruthTable, ChipSave) {
    let table = sim::TruthTable {
        num_inputs: inputs.len() as u8,
        num_outputs: outputs.len() as u8,
        name: name.into(),
        map,
    };
    let chip = ChipSave {
        attrs: ChipAttrs {
            name: name.into(),
            category: "Basic".into(),
            logic: Logic::Combinational,
        },
        region_size: (inputs.len() + outputs.len()) as u32,
        builtin: true,
        scene: None,
        l_nodes: inputs
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                (
                    String::from(*name),
                    sim::NodeAddr(idx as u32),
                    sim::Node::ZERO,
                )
            })
            .collect(),
        r_nodes: outputs
            .iter()
            .enumerate()
            .map(|(idx, name)| {
                (
                    String::from(*name),
                    sim::NodeAddr(idx as u32 + inputs.len() as u32),
                    sim::Node::new(
                        0,
                        sim::Source::new_table(sim::TruthTableSource::new(
                            table_id,
                            idx as u8,
                            sim::NodeAddr(0),
                        )),
                    ),
                )
            })
            .collect(),
        inner_nodes: vec![],
    };
    (table, chip)
}

/// Chips a user can optionally include in their new project.
/// (If you dont include a turing complete set, your project would be rendered unusable)
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum StartingChip {
    And = 0,
    Not = 1,
    Nand = 2,
    Or = 3,
    Nor = 4,
    Xor = 5,

    HalfAdder = 6,
    Adder = 7,
}
impl StartingChip {
    pub const COUNT: u8 = 8;

    pub fn from_u8(v: u8) -> Option<Self> {
        (v < Self::COUNT).then(|| unsafe { std::mem::transmute(v) })
    }

    pub fn create(self, library: &mut Library) {
        let table_id = library.allocate_table_empty();
        let (table, chip) = match self {
            Self::And => create_basic_chip(
                table_id,
                "And",
                &["a", "b"],
                &["out"],
                Box::new([0, 0, 0, 1]),
            ),
            Self::Not => create_basic_chip(table_id, "Not", &["in"], &["out"], Box::new([1, 0])),
            Self::Nand => create_basic_chip(
                table_id,
                "Nand",
                &["a", "b"],
                &["out"],
                Box::new([1, 1, 1, 0]),
            ),
            Self::Or => create_basic_chip(
                table_id,
                "Or",
                &["a", "b"],
                &["out"],
                Box::new([0, 1, 1, 1]),
            ),
            Self::Nor => create_basic_chip(
                table_id,
                "Nor",
                &["a", "b"],
                &["out"],
                Box::new([1, 0, 0, 0]),
            ),
            Self::Xor => create_basic_chip(
                table_id,
                "Xor",
                &["a", "b"],
                &["out"],
                Box::new([0, 1, 1, 0]),
            ),
            Self::HalfAdder => create_basic_chip(
                table_id,
                "HalfAdder",
                &["a", "b"],
                &["sum", "cout"],
                Box::new([0b00, 0b01, 0b01, 0b10]),
            ),
            Self::Adder => create_basic_chip(
                table_id,
                "Adder",
                &["a", "b", "cin"],
                &["sum", "cout"],
                //inputs:  000   001   010   011   100   101   110   111
                Box::new([0b00, 0b01, 0b01, 0b10, 0b01, 0b10, 0b10, 0b11]),
            ),
        };
        library.add_chip(chip);
        library.tables[table_id.0 as usize] = table;
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Project {
    pub name: String,
    pub scenes: Vec<scene::Scene>,
    pub library: Library,
}
impl Project {
    pub fn new(name: String, starting_chips: Vec<StartingChip>) -> Self {
        let mut library = Library::default();
        starting_chips
            .into_iter()
            .for_each(|chip| chip.create(&mut library));
        Self {
            name,
            scenes: vec![],
            library,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum IoType {
    Input,
    Output,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Library {
    pub tables: Vec<TruthTable>,
    pub chips: Vec<ChipSave>,
}
impl Library {
    pub fn categories<'a>(&'a self) -> impl Iterator<Item = &'a str> + '_ {
        let mut results: Vec<&'a str> = vec![];
        for chip in &self.chips {
            if !results.contains(&chip.attrs.category.as_str()) {
                results.push(chip.attrs.category.as_str());
            }
        }
        results.into_iter()
    }

    pub fn chips_in_category<'a: 'b, 'b>(
        &'a self,
        category: &'b str,
    ) -> impl Iterator<Item = (usize, &'a ChipSave)> + 'b {
        self.chips
            .iter()
            .enumerate()
            .filter(move |(_, chip)| chip.attrs.category.as_str() == category)
    }

    pub fn add_chip(&mut self, chip: ChipSave) {
        self.chips.push(chip);
    }

    pub fn allocate_table_empty(&mut self) -> TruthTableId {
        let id = TruthTableId(self.tables.len() as u8);
        self.tables.push(Default::default());
        id
    }

    // pub fn used_colors(&self) -> impl Iterator<Item = ItemColor> + '_ {
    //     (0..ItemColor::COUNT).filter_map(|v| {
    //         self.chips
    //             .iter()
    //             .any(|chip| chip.attrs.color as u8 == v)
    //             .then_some(ItemColor::from_u8(v).unwrap())
    //     })
    // }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Debug)]
#[repr(u8)]
/// Specifies either [Sequential Logic](https://en.wikipedia.org/wiki/Sequential_logic) or
/// [Combinational Logic](https://en.wikipedia.org/wiki/Combinational_logic).
pub enum Logic {
    Sequential,
    Combinational,
}
impl Logic {
    pub fn cycle(self) -> Self {
        match self {
            Self::Sequential => Self::Combinational,
            Self::Combinational => Self::Sequential,
        }
    }
    pub fn cycle_in_place(&mut self) {
        *self = (*self).cycle();
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ItemColor {
    White,
    Gray,
    Black,
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
    Magenta,
    Pink,
}
impl ItemColor {
    pub fn from_u8(v: u8) -> Option<Self> {
        (v < Self::COUNT).then(|| unsafe { std::mem::transmute(v) })
    }

    pub const COUNT: u8 = 12;
    pub fn as_color(self) -> Color {
        match self {
            Self::White => Color::WHITE,
            Self::Gray => Color::from_gray(100),
            Self::Black => Color::BLACK,
            Self::Red => Color::RED,
            Self::Orange => Color::from_rgb(0xFF, 0x5F, 0),
            Self::Yellow => Color::YELLOW,
            Self::Green => Color::GREEN,
            Self::Cyan => Color::from_rgb(0, 255, 255),
            Self::Blue => Color::BLUE,
            Self::Purple => Color::from_rgb(100, 0, 190),
            Self::Magenta => Color::from_rgb(255, 0, 255),
            Self::Pink => Color::from_rgb(0xFC, 0x88, 0xA3),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChipAttrs {
    pub name: String,
    pub category: String,
    pub logic: Logic,
}
impl Default for ChipAttrs {
    fn default() -> Self {
        Self {
            name: String::from("New Chip"),
            category: String::from("Basic"),
            logic: Logic::Combinational,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChipSave {
    pub builtin: bool,
    pub region_size: u32,
    pub attrs: ChipAttrs,
    pub scene: Option<scene::Scene>,
    pub l_nodes: Vec<(String, sim::NodeAddr, sim::Node)>,
    pub r_nodes: Vec<(String, sim::NodeAddr, sim::Node)>,
    pub inner_nodes: Vec<(sim::NodeAddr, sim::Node)>,
}
impl ChipSave {
    pub fn preview(&self, pos: Vec2, rotation: scene::Rotation) -> scene::Chip {
        fn io_ty(node: &sim::Node) -> IoType {
            match node.source().ty() {
                sim::SourceTy::NONE => IoType::Input,
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
            attrs: self.attrs.clone(),
            region: NodeRegion::default(),
            pos,
            rotation,
            save: None,
            l_nodes,
            r_nodes,
            inner_nodes: vec![],
        }
    }
}
