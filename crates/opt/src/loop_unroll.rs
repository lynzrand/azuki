//! Performs loop unrolling to every loop with <=7 instructions per loop.
//!
//!

use std::collections::{BTreeMap, BTreeSet};

use azuki_tac::{optimizer::FunctionOptimizer, BBId, Inst, InstId, InstKind, TacFunc};

pub struct LoopUnroll {}

impl FunctionOptimizer for LoopUnroll {
    fn name(&self) -> std::borrow::Cow<str> {
        "loop_unroll".into()
    }

    fn edits_program(&self) -> bool {
        true
    }

    fn optimize_func(
        &mut self,
        env: &mut azuki_tac::optimizer::OptimizeEnvironment,
        func: &mut azuki_tac::TacFunc,
    ) {
        // we define loop as a series of basic block following this pattern
        //
        // entry
        // \-> compare x to k, others are phis <---\
        //      \-> body --------------------------/
        //      \-> end
        //
        // Where `body` is a consecutive chain of basic blocks that does not
        // branch outwards (e.g. break), and only has backedges at the end.

        const UNROLL_FACTOR: usize = 10;

        // detect loop:
        let loops = detect_loops(&func);
    }
}

fn detect_loops(f: &TacFunc) -> Vec<Loop> {
    if f.starting_block().is_none() {
        return vec![];
    }
    // let mut loops = BTreeMap::new();
    {
        let mut vis = BTreeSet::new();
        let mut stack = vec![(true, f.starting_block().unwrap())];
        'outer: while let Some((first_time, bb_id)) = stack.pop() {
            if first_time {
                let visited = !vis.insert(bb_id);
                stack.push((false, bb_id));

                if visited {
                    // we got a loop! now let's figure out whether this loop
                    // follows our requirements.
                    let header = bb_id;
                    let &(_, backedge_end) = stack.get(stack.len() - 2).unwrap();
                    // here's a backedge: `backedge_end` -> `header`
                    // Figure out if header matches requirements. It should
                    // contain exactly one comparison and all others are phis.
                    // Additionally, the phis should contain one instruction that
                    // has exactly 2 operands and resolves to the comparison's operand
                    let mut phis = BTreeSet::new();
                    let bb = f.bb_get(header);
                    let cmp = match &bb.branch {
                        azuki_tac::Branch::CondJump { cond, .. } => match cond {
                            azuki_tac::Value::Dest(i) => {
                                let inst = f.inst_get(*i);
                                match get_comparison(&inst.kind) {
                                    Some(c) => (c, Some(*i)),
                                    None => {
                                        continue 'outer;
                                    }
                                }
                            }
                            azuki_tac::Value::Imm(i) => (Comparison::from_int(*i), None),
                        },
                        azuki_tac::Branch::Jump(..) => (Comparison::ConstantTrue, None),
                        _ => {
                            continue 'outer; // fail
                        }
                    };
                    for (inst_id, inst) in f.inst_of_bb_iter(header) {
                        if let InstKind::Phi(_) = &inst.kind {
                            phis.insert(inst_id);
                        } else if cmp.1.map_or(false, |c| Some(c) != cmp.1) {
                            continue 'outer; // fail
                        }
                    }
                    todo!("Unroll loop")
                } else {
                    let bb = f.bb_get(bb_id);
                    for target in bb.branch.target_iter() {
                        stack.push((true, target));
                    }
                }
            } else {
                // perform finalization, e.g. deregister
                vis.remove(&bb_id);
            }
        }
    }
    todo!()
}

fn get_comparison(i: &InstKind) -> Option<Comparison> {
    match i {
        azuki_tac::InstKind::Binary(azuki_tac::BinaryInst { op, lhs, rhs }) => {
            if matches!(
                op,
                azuki_tac::BinaryOp::Eq
                    | azuki_tac::BinaryOp::Ge
                    | azuki_tac::BinaryOp::Gt
                    | azuki_tac::BinaryOp::Le
                    | azuki_tac::BinaryOp::Lt
                    | azuki_tac::BinaryOp::Ne
            ) {
                if lhs.is_imm() ^ rhs.is_imm() {
                    Some(Comparison::ToConstant)
                } else if lhs.is_imm() && rhs.is_imm() {
                    let lhs = lhs.get_imm().unwrap();
                    let rhs = rhs.get_imm().unwrap();
                    if lhs == rhs {
                        Some(Comparison::ConstantTrue)
                    } else {
                        Some(Comparison::ConstantFalse)
                    }
                } else {
                    Some(Comparison::ToVariable)
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Debug, Clone)]
struct Loop {
    pub header: BBId,
    pub body: BTreeSet<BBId>,
    pub backedges: BTreeSet<BBId>,
    pub contains_inner: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Comparison {
    ToConstant,
    ToVariable,
    ConstantTrue,
    ConstantFalse,
}

impl Comparison {
    pub fn from_int(i: i64) -> Comparison {
        if i == 0 {
            Self::ConstantFalse
        } else {
            Self::ConstantTrue
        }
    }
}
