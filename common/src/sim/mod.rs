pub mod save;
pub mod scene;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TruthTableId(pub u16);

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct NodeAddr(pub u32);

#[derive(Clone, Copy, Debug)]
pub struct NodeRange(NodeAddr, NodeAddr);
impl NodeRange {
    pub fn count(self) -> u32 {
        (self.1).0 - (self.0).0 + 1
    }
}
impl IntoIterator for NodeRange {
    type Item = NodeAddr;
    type IntoIter = std::iter::Map<std::ops::RangeInclusive<u32>, fn(u32) -> NodeAddr>;

    fn into_iter(self) -> Self::IntoIter {
        ((self.0).0..=(self.1).0).map(NodeAddr)
    }
}

/// ### Representation:
/// byte 0:
///   bit 0 = state (0 = off, 1 = on),
///   bits 2, 3: SourceTy
///   bits 4..: unused
///
/// bytes 1-7: SourceData
///
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct Node(u64);
impl Node {
    pub const ZERO: Self = Self(0);

    pub const fn new(state: bool, src: Source) -> Self {
        let state = ((state as u64) << 63) | (src.0 & 0x7FFFFFFFFFFFFFFF);
        Self(state)
    }

    #[inline(always)]
    pub const fn state(&self) -> bool {
        (self.0 >> 63) == 1
    }
    #[inline(always)]
    pub fn set_state(&mut self, state: bool) {
        self.0 = (self.0 & 0x7FFFFFFFFFFFFFFF) | ((state as u64) << 63);
    }

    #[inline(always)]
    pub const fn source(&self) -> Source {
        Source(self.0)
    }
    #[inline(always)]
    pub fn set_source(&mut self, source: Source) {
        self.0 = (self.0 & 0x8000000000000000) | (source.0 & 0x7FFFFFFFFFFFFFFF);
    }
    #[inline(always)]
    pub fn set_source_unchecked(&mut self, source: Source) {
        self.0 = source.0;
    }
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(u8)]
pub enum SourceTy {
    None,
    Copy,
    Table,
}
impl SourceTy {
    #[inline(always)]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::None),
            1 => Some(Self::Copy),
            2 => Some(Self::Table),
            _ => None,
        }
    }
    #[inline(always)]
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Copy => 1,
            Self::Table => 2,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Source(u64);
impl Source {
    #[inline(always)]
    pub const fn new_none() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub const fn new_table(table: TruthTableSource) -> Self {
        Self(
            ((SourceTy::Table.as_u8() as u64) << 56)
                | ((table.inputs.0 as u64) << 24)
                | ((table.output as u64) << 16)
                | (table.id.0 as u64),
        )
    }
    #[inline(always)]
    pub const fn new_copy(addr: NodeAddr) -> Self {
        Self(((SourceTy::Copy.as_u8() as u64) << 56) | ((addr.0 as u64) << 24))
    }

    #[inline(always)]
    pub fn ty(&self) -> SourceTy {
        SourceTy::from_u8(((self.0 & 0x0300000000000000) >> 56) as u8).unwrap()
    }

    /// # Safety
    /// self.0 must hold a valid representation of SourceTy at bits 2-3.
    #[inline(always)]
    pub const unsafe fn ty_unchecked(&self) -> SourceTy {
        std::mem::transmute(((self.0 & 0x0300000000000000) >> 56) as u8)
    }
    #[inline(always)]
    pub fn set_ty(&mut self, t: SourceTy) {
        self.0 = (self.0 & 0x00FFFFFFFFFFFFFF) | (t.as_u8() as u64) << 56;
    }

    #[inline(always)]
    pub const fn table(&self) -> TruthTableSource {
        TruthTableSource {
            inputs: NodeAddr(((self.0 & 0x00FFFFFFFF000000) >> 24) as u32),
            output: ((self.0 & 0x0000000000FF0000) >> 16) as u8,
            id: TruthTableId((self.0 & 0x000000000000FFFF) as u16),
        }
    }
    #[inline(always)]
    pub fn set_table(&mut self, table: TruthTableSource) {
        let bits =
            ((table.inputs.0 as u64) << 24) | ((table.output as u64) << 16) | (table.id.0 as u64);
        self.0 = (self.0 & 0xFF00000000000000) | bits;
    }

    #[inline(always)]
    pub const fn addr(&self) -> NodeAddr {
        NodeAddr(((self.0 & 0x00FFFFFFFF000000) >> 24) as u32)
    }
    #[inline(always)]
    pub fn set_addr(&mut self, a: NodeAddr) {
        self.0 = (self.0 & 0xFF00000000FFFFFF) | ((a.0 as u64) << 24);
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TruthTableSource {
    pub inputs: NodeAddr, // implicitly a range of len 'table.num_inputs'
    pub output: u8,
    pub id: TruthTableId,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct NodeRegion {
    pub min: NodeAddr,
    pub max: NodeAddr,
}
impl NodeRegion {
    pub fn map(&self, addr: NodeAddr) -> NodeAddr {
        NodeAddr(addr.0 + self.min.0)
    }

    pub fn map_src(&self, mut src: Source) -> Source {
        if src.ty() == SourceTy::Copy {
            src.set_addr(self.map(src.addr()));
        }
        if src.ty() == SourceTy::Table {
            let mut table = src.table();
            table.inputs = self.map(table.inputs);
            src.set_table(table);
        }
        src
    }

    #[inline(always)]
    pub fn map_node(&self, mut node: Node) -> Node {
        node.set_source(self.map_src(node.source()));
        node
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TruthTable {
    pub num_inputs: u8,
    pub num_outputs: u8,
    pub name: String,
    pub map: Box<[u64]>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Sim {
    pub nodes: Vec<Node>,
    pub next_region: u32,
}
impl Default for Sim {
    fn default() -> Self {
        Self {
            nodes: vec![Node::default()],
            next_region: 1,
        }
    }
}
impl Sim {
    pub fn clear(&mut self) {
        self.nodes = vec![Node::default()];
        self.next_region = 1;
    }

    pub fn set_node_src(&mut self, addr: NodeAddr, src: Source) {
        self.nodes[addr.0 as usize].set_source(src);
    }

    #[inline(always)]
    pub fn set_node(&mut self, addr: NodeAddr, node: Node) {
        self.nodes[addr.0 as usize] = node;
    }
    #[inline(always)]
    pub fn get_node(&self, addr: NodeAddr) -> Node {
        self.nodes
            .get(addr.0 as usize)
            .copied()
            .unwrap_or(Node::ZERO)
    }
    #[inline(always)]
    pub fn mut_node(&mut self, addr: NodeAddr) -> &mut Node {
        self.nodes.get_mut(addr.0 as usize).unwrap()
    }

    pub fn alloc_node(&mut self) -> NodeAddr {
        self.alloc_region(1).min
    }

    pub fn alloc_region(&mut self, size: u32) -> NodeRegion {
        let min = self.next_region;
        self.next_region += size;
        let max = min + size;
        self.nodes.resize(max as usize, Node::default());
        NodeRegion {
            min: NodeAddr(min),
            max: NodeAddr(max),
        }
    }

    fn update_node(node: Node, out: &mut Node, nodes: &[Node], tables: &[TruthTable]) {
        match node.source().ty() {
            SourceTy::None => {}
            SourceTy::Copy => out.set_state(nodes[node.source().addr().0 as usize].state()),
            SourceTy::Table => {
                let table_src = node.source().table();
                let table = &tables[table_src.id.0 as usize];
                let input_nodes = &nodes[table_src.inputs.0 as usize
                    ..(table_src.inputs.0 as usize + table.num_inputs as usize)];
                let mut input: u32 = 0;
                for (idx, node) in input_nodes.iter().enumerate() {
                    input |= (node.state() as u32) << idx as u32;
                }
                let output = table.map[input as usize];
                let x = table_src.output as u64;
                let sel_output = ((output & (1u64 << x)) >> x) != 0;
                out.set_state(sel_output);
            }
        }
    }

    pub fn update(&mut self, tables: &[TruthTable]) {
        let mut new_nodes = self.nodes.clone();
        for (idx, node) in self.nodes.iter().enumerate() {
            Self::update_node(*node, &mut new_nodes[idx], &self.nodes, tables);
        }
        self.nodes = new_nodes;
    }

    pub fn into_chip(&self) -> save::ChipSave {
        todo!()
    }

    pub fn add_chip(&mut self, _chip: &save::ChipSave) {
        todo!()
    }
}
