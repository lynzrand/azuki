//! TAC (Three-Address Code) is the intermediate representation for Azuki programs.
//! This code type is similar to [Cranelift IR][cranelift] and [LLVM IR][llvm].
//!
//! This crate contains definition of Azuki TAC, code to serialize and deserialize
//! them, and to construct them from regular control flows in a higher-level
//! programming language.
//!
//! See the details in separate modules.
//!
//! [cranelift]: https://github.com/bytecodealliance/wasmtime
//! [llvm]: https://llvm.org

pub mod builder;
pub mod err;
pub mod formatter;
mod linkedlist;
pub mod optimizer;
pub mod parser;
pub mod ty;
pub mod util;

use std::collections::{BTreeMap, HashMap};

use err::{Error, TacResult};
use indexmap::map::Values;
use petgraph::{
    graph::DiGraph,
    graph::{self, NodeIndex},
};
use smol_str::SmolStr;
use thunderdome::{Arena, Index};

pub use ty::{NumericTy, Ty, TyKind};
use util::VarIter;

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: HashMap<SmolStr, TacFunc>,
}

pub type OpRef = Index;

/// The index of a basic block.
pub type BBId = NodeIndex;

/// A function made of TAC instructions.
///
/// The instructions are represented as an indirect doubly linked list inside the
/// `arena` using item indices. Every basic block holds the start and end index
/// of its instructions.
#[derive(Debug, Clone)]
pub struct TacFunc {
    /// Function name
    pub name: SmolStr,
    /// Function type
    pub ty: Ty,
    /// Mapping between function parameters and instructions
    pub param_map: BTreeMap<usize, OpRef>,
    /// An arena to allocate instructions
    arena: Arena<Tac>,
    /// Basic blocks inside this function
    pub basic_blocks: DiGraph<BasicBlock, ()>,
    /// The initial basic block to start with, usually the smallest index in graph
    pub starting_block: BBId,
}

impl TacFunc {
    pub fn new(name: SmolStr, ty: Ty) -> TacFunc {
        let mut basic_blocks = DiGraph::new();
        let starting_block = basic_blocks.add_node(BasicBlock {
            head: None,
            tail: None,
            jumps: Default::default(),
        });

        TacFunc {
            name,
            ty,
            param_map: BTreeMap::new(),
            arena: Arena::new(),
            basic_blocks,
            starting_block,
        }
    }

    pub fn new_untyped(name: SmolStr) -> TacFunc {
        Self::new(name, Ty::unit())
    }

    pub fn param_map(&self) -> &BTreeMap<usize, OpRef> {
        &self.param_map
    }

    pub fn param_map_mut(&mut self) -> &mut BTreeMap<usize, OpRef> {
        &mut self.param_map
    }

    /// Insert a new TAC into arena with no next instruction, and belongs to Basic Block `bb`.
    ///
    /// Note: The user **MUST** ensure the `bb` field to be correct.
    pub fn tac_new(&mut self, inst: Inst, bb: BBId) -> OpRef {
        self.arena.insert(Tac::independent(inst, bb))
    }

    #[inline]
    pub fn arena_get(&self, idx: OpRef) -> TacResult<&Tac> {
        self.arena.get(idx).ok_or(Error::NoSuchTacIdx(idx))
    }

    #[inline]
    pub fn arena_get_mut(&mut self, idx: OpRef) -> TacResult<&mut Tac> {
        self.arena.get_mut(idx).ok_or(Error::NoSuchTacIdx(idx))
    }

    /// Insert a new TAC after the given instruction
    pub fn tac_set_after(&mut self, after: OpRef, inst: OpRef) -> TacResult<()> {
        let after_inst = self.arena_get(after)?;
        let n = after_inst.next;

        let current_inst = self.arena_get_mut(inst)?;
        current_inst.prev = Some(after);
        current_inst.next = n;

        let after_inst = self.arena_get_mut(after)?;
        after_inst.next = Some(inst);

        if let Some(idx) = n {
            let next = self.arena_get_mut(idx)?;
            next.prev = Some(inst);
        };
        Ok(())
    }

    /// Insert a new TAC before the given instruction
    pub fn tac_set_before(&mut self, before: OpRef, inst: OpRef) -> TacResult<()> {
        let before_inst = self.arena_get(before)?;
        let n = before_inst.prev;

        let current_inst = self.arena_get_mut(inst)?;
        current_inst.next = Some(before);
        current_inst.prev = n;

        let before_inst = self.arena_get_mut(before)?;
        before_inst.prev = Some(inst);

        if let Some(idx) = n {
            let next = self.arena_get_mut(idx)?;
            next.next = Some(inst);
        }
        Ok(())
    }

    /// Remove the next instruction of the given instruction
    ///
    /// Errors if the given instruction does not exist.
    pub fn tac_remove_at(&mut self, idx: OpRef) -> TacResult<Inst> {
        let target = self.arena_get(idx)?;

        let next_idx = target.next;
        let prev_idx = target.prev;

        let bb = target.bb;
        let bb = self.basic_blocks.node_weight_mut(bb).unwrap();
        if bb.head == Some(idx) {
            bb.head = next_idx;
        }
        if bb.tail == Some(idx) {
            bb.tail = prev_idx;
        }

        if let Some(prev_idx) = prev_idx {
            let prev = self.arena_get_mut(prev_idx)?;
            prev.next = next_idx;
        }
        if let Some(next_idx) = next_idx {
            let next = self.arena_get_mut(next_idx)?;
            next.prev = prev_idx;
        }
        Ok(self.arena.remove(idx).unwrap().inst)
    }

    /// Connect TAC instruction `head` to the place after `tail`.
    ///
    /// Errors if `tail` does not exist or `tail.next` is not `None`. _This function
    /// does not check for availability of `head`._
    ///
    /// # Panics
    ///
    /// This function requires `head` and `tail` to be different. Panics if
    /// `head == tail`.
    pub fn tac_connect(&mut self, tail: OpRef, head: OpRef) -> TacResult<()> {
        assert_ne!(tail, head, "Can't connect one same instruction!");

        let tail_tac = self.arena.get_mut(tail);
        let tail_tac = tail_tac.ok_or(Error::NoSuchTacIdx(tail))?;
        if tail_tac.next.is_some() {
            return Err(Error::AlreadyConnected);
        }
        tail_tac.next = Some(head);

        let head_tac = self.arena.get_mut(head);
        let head_tac = head_tac.ok_or(Error::NoSuchTacIdx(head))?;
        if head_tac.prev.is_some() {
            let tail_tac = self.arena.get_mut(tail);
            let tail_tac = tail_tac.ok_or(Error::NoSuchTacIdx(tail))?;
            tail_tac.next = None;
            return Err(Error::AlreadyConnected);
        }
        head_tac.prev = Some(tail);

        Ok(())
    }

    /// Breaks off TAC chain after position `pos`. Returns the index of head instruction of the
    /// latter chain.
    ///
    /// Errors if `pos` does not exist or there is no code after `tail`.
    pub fn tac_break_off_after(&mut self, pos: OpRef) -> TacResult<OpRef> {
        let tail = self.arena_get_mut(pos)?;
        if tail.next.is_none() {
            return Err(Error::NotConnected);
        }
        Ok(tail.next.take().unwrap())
    }

    pub fn all_inst_unordered(&self) -> impl Iterator<Item = (OpRef, BBId, &Inst)> {
        self.arena
            .iter()
            .map(|(idx, inst)| (idx, inst.bb, &inst.inst))
    }
}

/// A single basic block, represented as an indirect doubly linked list of instructions.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Linked list head
    pub(crate) head: Option<OpRef>,
    /// Linked list tail
    pub(crate) tail: Option<OpRef>,

    /// The branch instruction at the end of this basic block
    pub jumps: Vec<Branch>,
}

/// Represents a single TAC instruction inside an indirect doubly linked list of instructions.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tac {
    /// The actual instruction.
    pub inst: Inst,

    /// The basic block this instruction is in.
    pub bb: BBId,

    /// The previous instruction in this list.
    pub prev: Option<OpRef>,
    /// The next instruction in this list.
    pub next: Option<OpRef>,
}

impl Tac {
    pub fn new(inst: Inst, prev: Option<OpRef>, next: Option<OpRef>, bb: BBId) -> Self {
        Self {
            inst,
            prev,
            next,
            bb,
        }
    }

    pub fn independent(inst: Inst, bb: BBId) -> Tac {
        Tac {
            inst,
            bb,
            prev: None,
            next: None,
        }
    }
}

impl linkedlist::SinglyLinkedList for Tac {
    type Key = OpRef;

    type Context = Arena<Tac>;

    fn next_value_key(&self) -> Option<Self::Key> {
        self.next
    }

    fn get_value(ctx: &Self::Context, key: Self::Key) -> &Self {
        ctx.get(key).unwrap()
    }

    fn get_value_mut(ctx: &mut Self::Context, key: Self::Key) -> &mut Self {
        ctx.get_mut(key).unwrap()
    }

    fn insert_value_after(_ctx: &mut Self::Context, _value: Self) -> Self::Key {
        todo!()
    }

    fn set_next_value_key(&mut self) {
        todo!()
    }
}

impl linkedlist::DoublyLinkedList for Tac {
    fn prev_value_key(&self) -> Option<Self::Key> {
        self.prev
    }

    fn set_prev_value_key(&mut self) {
        todo!()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BinaryInst {
    pub op: BinaryOp,
    pub lhs: Value,
    pub rhs: Value,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FunctionCall {
    pub name: SmolStr,
    pub params: Vec<Value>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Inst {
    pub kind: InstKind,
    pub ty: Ty,
}

/// Kinds of an instruction
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum InstKind {
    /// A binary operaton, e.g. plus, divide
    Binary(BinaryInst),
    /// A call to another function.
    FunctionCall(FunctionCall),
    /// A constant value.
    Const(Immediate),
    /// An assignment from another value
    Assign(Index),
    /// A parameter
    Param,
    /// An unreachable value
    Dead,
}

impl InstKind {
    pub fn params_iter(&self) -> impl Iterator<Item = Value> + '_ {
        match self {
            InstKind::Binary(b) => VarIter::Two(b.lhs.clone(), b.rhs.clone()),
            InstKind::FunctionCall(f) => VarIter::Iter(f.params.iter().cloned()),
            InstKind::Const(v) => VarIter::One(Value::Imm(*v)),
            InstKind::Assign(v) => VarIter::One((*v).into()),
            InstKind::Param => VarIter::None,
            InstKind::Dead => VarIter::None,
        }
    }

    pub fn param_op_iter(&self) -> impl Iterator<Item = OpRef> + '_ {
        self.params_iter().filter_map(|x| x.get_inst())
    }
}

/// Represents a branch instruction.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Branch {
    /// Returns the given value.
    Return(Option<Value>),

    /// Jumps to the given target with given parameters.
    Jump(BranchTarget),

    /// Conditional jump to the given targets.
    ///
    /// `cond` must be a boolean or integer.
    CondJump { cond: Value, target: BranchTarget },
}

// impl Default for Branch {
//     fn default() -> Self {
//         Self::Unreachable
//     }
// }

impl Branch {
    pub fn target_iter(&self) -> impl Iterator<Item = BBId> + '_ {
        match self {
            Branch::Return(_) => util::OptionIter::<BBId>::None,
            Branch::Jump(t) => util::OptionIter::One(t.bb),
            Branch::CondJump { target, .. } => util::OptionIter::One(target.bb),
            // Branch::TableJump { target, .. } => util::VarIter::Iter(target.iter().map(|t| t.bb)),
            // Branch::Unreachable => util::VarIter::None,
        }
    }

    pub fn target(&self) -> Option<&BranchTarget> {
        match self {
            Branch::Return(_) => None,
            Branch::Jump(j) => Some(j),
            Branch::CondJump { target, .. } => Some(target),
        }
    }

    pub fn target_mut(&mut self) -> Option<&mut BranchTarget> {
        match self {
            Branch::Return(_) => None,
            Branch::Jump(j) => Some(j),
            Branch::CondJump { target, .. } => Some(target),
        }
    }

    pub fn add_param(&mut self, bb_id: BBId, param: Index, source_var: Index) {
        match self {
            Branch::Return(_) => {}
            Branch::Jump(target) => {
                target.add_param_if_bb(bb_id, param, source_var);
            }
            Branch::CondJump { target, .. } => {
                target.add_param_if_bb(bb_id, param, source_var);
            } // Branch::TableJump { target, .. } => {
              //     for branch_target in target {
              //         branch_target.add_param_if_bb(bb_id, param, source_var);
              //     }
              // } // Branch::Unreachable => {}
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchTarget {
    pub bb: BBId,
    /// Basic block parameters, described as a Index-Index mapping (similar to phi)
    pub params: BTreeMap<Index, Index>,
}

impl BranchTarget {
    pub fn add_param(&mut self, param: Index, source: Index) {
        self.params.insert(param, source);
    }

    pub fn add_param_if_bb(&mut self, bb_id: BBId, param: Index, source: Index) {
        if self.bb == bb_id {
            self.add_param(param, source)
        }
    }

    pub fn remove_param(&mut self, param: Index) {
        self.params.remove(&param);
    }

    pub fn remove_param_if_bb(&mut self, bb_id: BBId, param: Index) {
        if self.bb == bb_id {
            self.remove_param(param)
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Dest(OpRef),
    Imm(Immediate),
}

impl Value {
    pub fn get_imm(&self) -> Option<Immediate> {
        match self {
            Value::Dest(_) => None,
            Value::Imm(i) => Some(*i),
        }
    }

    pub fn get_inst(&self) -> Option<OpRef> {
        match self {
            Value::Dest(o) => Some(*o),
            _ => None,
        }
    }
}

impl From<OpRef> for Value {
    fn from(x: OpRef) -> Self {
        Value::Dest(x)
    }
}

type Immediate = i64;
