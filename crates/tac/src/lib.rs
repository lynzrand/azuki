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

use enum_as_inner::EnumAsInner;
use err::{Error, TacResult};

use petgraph::{graph::DiGraph, graph::NodeIndex};
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
#[derive(Debug, Clone, Default)]
pub struct TacFunc {
    /// Function name
    pub name: SmolStr,
    /// Function type
    pub ty: Ty,

    // The followings are allocating spaces for data types
    /// An arena to allocate instructions
    instructions_arena: Arena<Tac>,
    /// An arena to allocate branch instructions
    branch_inst_arena: Arena<Branch>,
    /// An arena to allocate basic block info
    basic_block_arena: Arena<BasicBlock>,

    /// Basic blocks inside this function
    pub basic_blocks: DiGraph<BasicBlock, ()>,
    /// The sequence of basic blocks
    pub bb_seq: Vec<BBId>,
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
            instructions_arena: Arena::new(),
            branch_inst_arena: Arena::new(),
            basic_block_arena: Arena::new(),
            basic_blocks,
            bb_seq: vec![starting_block],
        }
    }

    pub fn new_untyped(name: SmolStr) -> TacFunc {
        Self::new(name, Ty::unit())
    }

    pub fn starting_block(&self) -> Option<BBId> {
        self.bb_seq.first().cloned()
    }

    /// Insert a new TAC into arena with no next instruction, and belongs to Basic Block `bb`.
    ///
    /// Note: The user **MUST** ensure the `bb` field to be correct.
    pub fn tac_new(&mut self, inst: Inst, bb: BBId) -> OpRef {
        self.instructions_arena.insert(Tac::independent(inst, bb))
    }

    /// Insert a new TAC into arena with no next instruction, without a proper
    /// basic block ID.
    pub fn tac_new_no_bb(&mut self, inst: Inst) -> OpRef {
        self.instructions_arena
            .insert(Tac::independent(inst, BBId::end()))
    }

    /// Set the basic block field of this TAC.
    pub fn tac_set_bb(&mut self, idx: OpRef, bb: BBId) -> TacResult<()> {
        self.arena_get_mut(idx)?.bb = bb;
        Ok(())
    }

    #[inline]
    pub fn arena_get(&self, idx: OpRef) -> TacResult<&Tac> {
        self.instructions_arena
            .get(idx)
            .ok_or(Error::NoSuchTacIdx(idx))
    }

    #[inline]
    pub fn arena_get_mut(&mut self, idx: OpRef) -> TacResult<&mut Tac> {
        self.instructions_arena
            .get_mut(idx)
            .ok_or(Error::NoSuchTacIdx(idx))
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
        Ok(self.instructions_arena.remove(idx).unwrap().inst)
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

        let tail_tac = self.instructions_arena.get_mut(tail);
        let tail_tac = tail_tac.ok_or(Error::NoSuchTacIdx(tail))?;
        if tail_tac.next.is_some() {
            return Err(Error::AlreadyConnected);
        }
        tail_tac.next = Some(head);

        let head_tac = self.instructions_arena.get_mut(head);
        let head_tac = head_tac.ok_or(Error::NoSuchTacIdx(head))?;
        if head_tac.prev.is_some() {
            let tail_tac = self.instructions_arena.get_mut(tail);
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
        self.instructions_arena
            .iter()
            .map(|(idx, inst)| (idx, inst.bb, &inst.inst))
    }
}

/// A single basic block, represented as an indirect doubly linked list of instructions.
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Linked list head
    pub head: Option<OpRef>,
    /// Linked list tail
    pub tail: Option<OpRef>,

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
#[derive(Debug, Clone, Eq, PartialEq, EnumAsInner)]
pub enum InstKind {
    /// A binary operaton, e.g. plus, divide
    Binary(BinaryInst),
    /// A call to another function.
    FunctionCall(FunctionCall),

    /// An assignment from another instruction or constant
    Assign(Value),
    /// A phi instruction
    Phi(BTreeMap<BBId, OpRef>),
    /// A function parameter
    Param(usize),
    /// An unreachable value
    Dead,
}

impl InstKind {
    pub fn params_iter(&self) -> impl Iterator<Item = Value> + '_ {
        match self {
            InstKind::Binary(b) => VarIter::Two(b.lhs, b.rhs),
            InstKind::FunctionCall(f) => {
                VarIter::Iter(Box::new(f.params.iter().cloned()) as Box<dyn Iterator<Item = _>>)
            }
            InstKind::Assign(v) => VarIter::One(*v),
            InstKind::Phi(source) => {
                VarIter::Iter(Box::new(source.iter().map(|(_, &val)| val.into()))
                    as Box<dyn Iterator<Item = _>>)
            }
            InstKind::Param(_) => VarIter::None,
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

    /// Jumps to the given target
    Jump(BBId),

    /// Conditional jump to the given target.
    ///
    /// `cond` must be a boolean or integer.
    CondJump { cond: Value, target: BBId },
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
            Branch::Jump(t) => util::OptionIter::One(*t),
            Branch::CondJump { target, .. } => util::OptionIter::One(*target),
            // Branch::TableJump { target, .. } => util::VarIter::Iter(target.iter().map(|t| t.bb)),
            // Branch::Unreachable => util::VarIter::None,
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
