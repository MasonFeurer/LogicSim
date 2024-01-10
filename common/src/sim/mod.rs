pub mod save;
pub mod scene;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TruthTableId(pub u8);

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
/// - byte 0: state (0 = off, 1 = on) (or a node can hold a byte as it's state),
/// - bytes 1..: source
///
#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug)]
#[repr(C)]
pub struct Node(u64);
impl Node {
    pub const ZERO: Self = Self(0);

    #[inline(always)]
    pub fn toggle_state(&mut self) {
        self.set_state(1u8.wrapping_sub(self.state()));
    }

    #[inline(always)]
    pub fn new(state: u8, src: Source) -> Self {
        Self(((state as u64) << 56) | (src.0 & 0x00FFFFFFFFFFFFFF))
    }

    #[inline(always)]
    pub const fn state(&self) -> u8 {
        (self.0 >> 56) as u8
    }
    #[inline(always)]
    pub fn set_state(&mut self, state: u8) {
        self.0 = (self.0 & 0x00FFFFFFFFFFFFFF) | ((state as u64) << 56);
    }

    #[inline(always)]
    pub const fn source(&self) -> Source {
        Source(self.0)
    }
    #[inline(always)]
    pub fn set_source(&mut self, src: Source) {
        self.0 = (self.0 & 0xFF00000000000000) | (src.0 & 0x00FFFFFFFFFFFFFF);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTy(u8);
impl SourceTy {
    pub const NONE: Self = Self(0);
    pub const COPY: Self = Self(1);
    pub const TABLE: Self = Self(2);
    pub const OP: Self = Self(3);
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct CopySource(u64);
impl CopySource {
    #[inline(always)]
    pub const fn new(addr: NodeAddr) -> Self {
        Self(addr.0 as u64)
    }

    #[inline(always)]
    pub const fn addr(&self) -> NodeAddr {
        NodeAddr((self.0 & 0xFFFFFFFF) as u32)
    }

    #[inline(always)]
    pub fn set_addr(&mut self, addr: NodeAddr) {
        self.0 = (self.0 & !0xFFFFFFFF) | addr.0 as u64;
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TruthTableSource(u64);
impl TruthTableSource {
    #[inline(always)]
    pub const fn new(id: TruthTableId, output: u8, inputs: NodeAddr) -> Self {
        Self(((id.0 as u64) << 40) | ((output as u64) << 32) | inputs.0 as u64)
    }

    #[inline(always)]
    pub const fn id(&self) -> TruthTableId {
        TruthTableId(((self.0 >> 40) & 0xFF) as u8)
    }
    #[inline(always)]
    pub const fn output(&self) -> u8 {
        ((self.0 >> 32) & 0xFF) as u8
    }
    #[inline(always)]
    pub const fn inputs(&self) -> NodeAddr {
        NodeAddr(self.0 as u32) // THIS GOOD?
    }

    #[inline(always)]
    pub fn set_id(&mut self, id: TruthTableId) {
        self.0 = (self.0 & 0xFFFF00FFFFFFFFFF) | ((id.0 as u64) << 40);
    }
    #[inline(always)]
    pub fn set_output(&mut self, output: u8) {
        self.0 = (self.0 & 0xFFFFFF00FFFFFFFF) | ((output as u64) << 32);
    }
    #[inline(always)]
    pub fn set_inputs(&mut self, inputs: NodeAddr) {
        self.0 = (self.0 & !0xFFFFFFFF) | inputs.0 as u64;
    }
}

/// ### Representation:
/// - byte 0: padding
/// - byte 1: type: SourceTy
/// - bytes 2..: data: TruthTableSource | NodeAddr
///
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Source(pub u64);
impl Source {
    #[inline(always)]
    pub const fn new_none() -> Self {
        Self(0)
    }
    #[inline(always)]
    pub fn new_table(table: TruthTableSource) -> Self {
        // Safety: TruthTableSource is always a valid u64.
        let data: u64 = unsafe { std::mem::transmute(table) };
        Self(((SourceTy::TABLE.0 as u64) << 48) | (data & 0x0000FFFFFFFFFFFF))
    }
    #[inline(always)]
    pub const fn new_addr(addr: NodeAddr) -> Self {
        Self(((SourceTy::COPY.0 as u64) << 48) | addr.0 as u64)
    }

    #[inline(always)]
    pub const fn ty(&self) -> SourceTy {
        SourceTy(((self.0 & 0x00FF000000000000) >> 48) as u8)
    }

    #[inline(always)]
    pub const fn as_table(&self) -> TruthTableSource {
        unsafe { std::mem::transmute(*self) }
    }
    #[inline(always)]
    pub const fn as_copy(&self) -> CopySource {
        unsafe { std::mem::transmute(*self) }
    }

    #[inline(always)]
    pub fn as_copy_mut(&mut self) -> &mut CopySource {
        unsafe { std::mem::transmute(self) }
    }
    #[inline(always)]
    pub fn as_table_mut(&mut self) -> &mut TruthTableSource {
        unsafe { std::mem::transmute(self) }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct NodeRegion {
    pub min: NodeAddr,
    pub max: NodeAddr,
}
impl NodeRegion {
    pub fn map(&self, addr: NodeAddr) -> NodeAddr {
        NodeAddr(addr.0 + self.min.0)
    }
    pub fn map_src(&self, mut src: Source) -> Source {
        if src.ty() == SourceTy::COPY {
            let copy = src.as_copy_mut();
            copy.set_addr(self.map(copy.addr()));
        }
        if src.ty() == SourceTy::TABLE {
            let table = src.as_table_mut();
            table.set_inputs(self.map(table.inputs()));
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
            SourceTy::NONE => {}
            SourceTy::COPY => {
                out.set_state(nodes[node.source().as_copy().addr().0 as usize].state())
            }
            SourceTy::TABLE => {
                let table_src = node.source().as_table();

                let table = &tables[table_src.id().0 as usize];
                let input_nodes = &nodes[table_src.inputs().0 as usize
                    ..(table_src.inputs().0 as usize + table.num_inputs as usize)];
                let mut input: u32 = 0;
                for (idx, node) in input_nodes.iter().enumerate() {
                    input |= (node.state() as u32) << idx as u32;
                }
                let output = table.map[input as usize];
                let x = table_src.output() as u64;
                let sel_output = ((output & (1u64 << x)) >> x) != 0;
                out.set_state(sel_output as u8);
            }
            x => panic!("encountered invalid source type in sim : {}", x.0),
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
