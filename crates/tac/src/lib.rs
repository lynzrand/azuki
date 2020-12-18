pub mod ty;

use std::collections::HashMap;

use generational_arena::{Arena, Index};
use smol_str::SmolStr;

type OpRef = Index;

pub struct TacFunc {
    pub arena: Arena<Tac>,
    pub basic_blocks: HashMap<usize, BasicBlock>,
}

pub struct BasicBlock {
    pub op_start: Option<OpRef>,
}

pub struct Tac {
    pub op: Inst,
    pub next: Option<OpRef>,
}

pub struct BinaryInst {
    pub op: BinaryOp,
    pub lhs: Value,
    pub rhs: Value,
}

pub struct FunctionCall {
    pub name: SmolStr,
    pub params: Vec<Value>,
}

pub enum Inst {
    Binary(BinaryInst),
    FunctionCall(FunctionCall),
    Const(Immediate),
}

pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub enum Value {
    Dest(OpRef),
    Jump(usize),
    CondJump { cond: OpRef, target: usize },
}

pub enum Immediate {
    Int(i64),
    UInt(u64),
}
