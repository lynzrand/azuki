use std::collections::HashMap;

use azuki_tac::{OpRef, Program};
use smol_str::SmolStr;

pub struct Vm<'a> {
    program: &'a Program,
    stack: Vec<Frame>,
}

struct Frame {
    func: SmolStr,
    instruction: OpRef,
    vars: HashMap<u32, i64>,
}

impl Vm<'_> {}
