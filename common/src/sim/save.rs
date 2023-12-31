use crate::graphics::Color;
use crate::sim::{self, scene, NodeRegion, TruthTable};
use glam::Vec2;
use serde::{Deserialize, Serialize};

pub type SaveId = crate::Id;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IoType {
    Input,
    Output,
}

#[derive(Serialize, Deserialize)]
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
            attrs: ChipAttrs {
                name: "And".into(),
                color: ItemColor::White,
                logic: Logic::Combinational,
            },
            region_size: 3,
            builtin: true,
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
            attrs: ChipAttrs {
                name: "Not".into(),
                color: ItemColor::White,
                logic: Logic::Combinational,
            },
            region_size: 2,
            builtin: true,
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

    pub fn used_colors(&self) -> impl Iterator<Item = ItemColor> + '_ {
        use crate::graphics::ui::CycleState;

        // Technically not O(n^2) because ItemColor::COUNT is constant, thus the complexity is actually O(n)
        (0..ItemColor::COUNT).filter_map(|v| {
            self.chips
                .iter()
                .any(|chip| chip.attrs.color.as_u8() == v)
                .then_some(ItemColor::from_u8(v).unwrap())
        })
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
/// Specifies either [Sequential Logic](https://en.wikipedia.org/wiki/Sequential_logic) or
/// [Combinational Logic](https://en.wikipedia.org/wiki/Combinational_logic).
pub enum Logic {
    Sequential,
    Combinational,
}
impl crate::ui::CycleState for Logic {
    fn from_u8(b: u8) -> Option<Self> {
        (b < 2).then(|| unsafe { std::mem::transmute(b) })
    }
    fn as_u8(&self) -> u8 {
        unsafe { std::mem::transmute(*self) }
    }
    fn label(&self) -> &'static str {
        match *self {
            Self::Sequential => "Sequential",
            Self::Combinational => "Combinational",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    pub const COUNT: u8 = 12;
    pub fn as_color(self) -> Color {
        match self {
            Self::White => Color::WHITE,
            Self::Gray => Color::shade(100),
            Self::Black => Color::BLACK,
            Self::Red => Color::RED,
            Self::Orange => Color::ORANGE,
            Self::Yellow => Color::YELLOW,
            Self::Green => Color::GREEN,
            Self::Cyan => Color::CYAN,
            Self::Blue => Color::BLUE,
            Self::Purple => Color::rgb(100, 0, 190),
            Self::Magenta => Color::MAGENTA,
            Self::Pink => Color::PINK,
        }
    }
}
impl crate::ui::CycleState for ItemColor {
    fn from_u8(b: u8) -> Option<Self> {
        (b < Self::COUNT).then(|| unsafe { std::mem::transmute(b) })
    }
    fn as_u8(&self) -> u8 {
        unsafe { std::mem::transmute(*self) }
    }
    fn label(&self) -> &'static str {
        match *self {
            Self::White => "White",
            Self::Gray => "Gray",
            Self::Black => "Black",
            Self::Red => "Red",
            Self::Orange => "Orange",
            Self::Yellow => "Yellow",
            Self::Green => "Green",
            Self::Cyan => "Cyan",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Magenta => "Magenta",
            Self::Pink => "Pink",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChipAttrs {
    pub name: String,
    pub color: ItemColor,
    pub logic: Logic,
}
impl Default for ChipAttrs {
    fn default() -> Self {
        Self {
            name: String::from("New Chip"),
            color: ItemColor::White,
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
