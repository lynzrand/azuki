pub mod err;
pub mod ty;

use std::collections::HashMap;

use err::{Error, TacResult};
use generational_arena::{Arena, Index};
use smol_str::SmolStr;

type OpRef = Index;

/// A function made of TAC instructions.
///
/// The instructions are represented as a doubly linked list inside the
/// `arena` using item indecies. Every basic block holds the start and end index
/// of its instructions.
#[derive(Debug, Clone)]
pub struct TacFunc {
    pub arena: Arena<Tac>,
    pub basic_blocks: HashMap<usize, BasicBlock>,
}

impl TacFunc {
    pub fn new() -> TacFunc {
        TacFunc {
            arena: Arena::new(),
            basic_blocks: HashMap::new(),
        }
    }

    /// Insert a new TAC into arena with no next instruction
    pub fn tac_new(&mut self, inst: Inst) -> OpRef {
        self.arena.insert(Tac::independent(inst))
    }

    /// Insert a new TAC after the given instruction
    pub fn tac_insert_after(&mut self, idx: OpRef, inst: Inst) -> TacResult<OpRef> {
        let target = self
            .arena
            .get_mut(idx)
            .ok_or_else(|| Error::NoSuchTacIdx(idx))?;
        let n = target.next;
        let tac = Tac {
            inst,
            prev: Some(idx),
            next: n,
        };
        let new_idx = self.arena.insert(tac);
        let target = self
            .arena
            .get_mut(idx)
            .ok_or_else(|| Error::NoSuchTacIdx(idx))?;
        target.next = Some(new_idx);
        if let Some(idx) = n {
            let next = self
                .arena
                .get_mut(idx)
                .ok_or_else(|| Error::NoSuchTacIdx(idx))?;
            next.prev = Some(new_idx);
        }
        Ok(new_idx)
    }

    /// Remove the next instruction of the given instruction
    ///
    /// Errors if the given instruction does not exist.
    pub fn tac_remove_at(&mut self, idx: OpRef) -> TacResult<Inst> {
        let target = self
            .arena
            .get(idx)
            .ok_or_else(|| Error::NoSuchTacIdx(idx))?;

        let next_idx = target.next;
        let prev_idx = target.prev;

        if let Some(prev_idx) = prev_idx {
            let prev = self
                .arena
                .get_mut(prev_idx)
                .ok_or_else(|| Error::NoSuchTacIdx(idx))?;
            prev.next = next_idx;
        }
        if let Some(next_idx) = next_idx {
            let next = self
                .arena
                .get_mut(next_idx)
                .ok_or_else(|| Error::NoSuchTacIdx(idx))?;
            next.prev = prev_idx;
        }
        Ok(self.arena.remove(idx).unwrap().inst)
    }

    /// Connect TAC instruction `head` to the place after `tail`.
    ///
    /// Errors if `tail` does not exist or `tail.next` is not `None`. _This function
    /// does not check for availability of `head`._
    pub fn tac_connect(&mut self, tail: OpRef, head: OpRef) -> TacResult<()> {
        let (tail_tac, head_tac) = self.arena.get2_mut(tail, head);
        let tail_tac = tail_tac.ok_or_else(|| Error::NoSuchTacIdx(tail))?;
        let head_tac = head_tac.ok_or_else(|| Error::NoSuchTacIdx(head))?;
        if tail_tac.next.is_some() || head_tac.prev.is_some() {
            return Err(Error::AlreadyConnected);
        }
        tail_tac.next = Some(head);
        head_tac.prev = Some(tail);
        Ok(())
    }

    /// Breaks off TAC chain after position `pos`. Returns the index of head instruction of the
    /// latter chain.
    ///
    /// Errors if `pos` does not exist or there is no code after `tail`.
    pub fn tac_break_off_after(&mut self, pos: OpRef) -> TacResult<OpRef> {
        let tail = self
            .arena
            .get_mut(pos)
            .ok_or_else(|| Error::NoSuchTacIdx(pos))?;
        if tail.next.is_none() {
            return Err(Error::NotConnected);
        }
        Ok(tail.next.take().unwrap())
    }
}
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub op_start: Option<OpRef>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tac {
    pub inst: Inst,
    pub prev: Option<OpRef>,
    pub next: Option<OpRef>,
}

impl Tac {
    pub fn new(inst: Inst, prev: Option<OpRef>, next: Option<OpRef>) -> Self {
        Self { inst, prev, next }
    }

    pub fn independent(inst: Inst) -> Tac {
        Tac {
            inst,
            prev: None,
            next: None,
        }
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
pub enum Inst {
    Binary(BinaryInst),
    FunctionCall(FunctionCall),
    Const(Immediate),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Dest(OpRef),
    Jump(usize),
    CondJump { cond: OpRef, target: usize },
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Immediate {
    Int(i64),
    UInt(u64),
}
