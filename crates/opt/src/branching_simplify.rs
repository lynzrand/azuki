//! Simplifies branches and removes empty basic blocks.
//!
//! # Optimizations
//!
//! Branching simplify performs the follow optimizations:
//!
//! - Replaces `brif _ bbI bbI` into `br bbI`.
//!
//! - Replaces `brif 0 bbI bbJ` into `br bbJ`; replaces `brIf x bbI bbJ` into
//!   `br bbI` where `x` is an immediate and `x != 0`.
//!
//! - Connects basic blocks `bbI` and `bbJ` if `bbI` ends in `br bbJ` and `bbJ`
//!   has only one predecessor.
//!
//! - Removes `bbI` if `bbI` has an empty body and ends in `br bbJ`
//!   (unconditional branch). Replaces all jumps into `bbI` to `bbJ`.
//!
//! - Removes `bbI` if `bbI` has an empty body and all predesessors of `bbI`
//!   is `br bbI` (unconditional branch). Replaces all jumps to `bbI` into
//!   the branching instruction of `bbI`.
//!
//! # Relationship
//!
//! This optimization should follow `const-folding` and be followed by
//! `dead-code-elimination` for best effects.

use std::{
    borrow::Borrow,
    collections::{HashSet, VecDeque},
};

use crate::util::graphs::cfg;
use azuki_tac::{optimizer::FunctionOptimizer, BBId, Branch, InstId, InstKind, TacFunc, Value};
use multimap::MultiMap;
use petgraph::EdgeDirection::{Incoming, Outgoing};
use tracing::trace;

/// Performs branching simplify. See [module documents](crate::branching_simplify).
pub struct BranchingSimplify;

impl FunctionOptimizer for BranchingSimplify {
    fn name(&self) -> std::borrow::Cow<str> {
        "branching-simplify".into()
    }

    fn edits_program(&self) -> bool {
        true
    }

    fn optimize_func(
        &mut self,
        env: &mut azuki_tac::optimizer::OptimizeEnvironment,
        func: &mut azuki_tac::TacFunc,
    ) {
        if func.first_block.is_none() {
            return;
        }

        let mut phis = MultiMap::new();
        let mut cfg = cfg(func);
        let mut vis = HashSet::new();

        // Collect all phi sources
        for (i, _, _) in func.all_inst_unordered() {
            let tac = func.tac_get(i);
            if let InstKind::Phi(phi) = &tac.inst.kind {
                for &bb in phi.keys() {
                    phis.insert(bb, i);
                }
            }
        }

        let mut pending = VecDeque::new();
        pending.push_back(func.first_block.unwrap());

        while let Some(bb_id) = pending.pop_front() {
            let bb = func.bb_get(bb_id);
            match &bb.branch {
                // Same branch simplification
                Branch::CondJump {
                    if_true, if_false, ..
                } if *if_true == *if_false => {
                    trace!("brif _ {} {} ==>> br {}", if_true, if_false, if_true);
                    func.bb_get_mut(bb_id).branch = Branch::Jump(*if_true);
                    pending.push_back(bb_id);
                }

                // Condition simplification
                &Branch::CondJump {
                    cond: Value::Imm(x),
                    if_true,
                    if_false,
                } => {
                    if x == 0 {
                        trace!("brif x {} {} ==>> br {}", if_true, if_false, if_false);

                        func.bb_get_mut(bb_id).branch = Branch::Jump(if_false);
                        cfg.remove_edge(bb_id, if_true);
                    } else {
                        trace!("brif x {} {} ==>> br {}", if_true, if_false, if_true);

                        func.bb_get_mut(bb_id).branch = Branch::Jump(if_true);
                        cfg.remove_edge(bb_id, if_false);
                    }
                    pending.push_back(bb_id);
                }

                // Connect bbs
                &Branch::Jump(next) if cfg.neighbors_directed(next, Incoming).count() == 1 => {
                    trace!("connect: {} <<== {}", bb_id, next);

                    func.bb_connect(bb_id, next);
                    func.bb_detach(next);
                    pending.push_back(bb_id);

                    replace_phis(&phis, func, next, bb_id);

                    cfg.remove_edge(bb_id, next);
                    let next_neighbors = cfg.neighbors_directed(next, Outgoing).collect::<Vec<_>>();
                    for n in next_neighbors {
                        cfg.remove_edge(next, n);
                        cfg.add_edge(bb_id, n, ());
                    }
                }

                // Collapse empty jump
                &Branch::Jump(next) if bb.is_empty() => {
                    let pred = cfg.neighbors_directed(bb_id, Incoming).collect::<Vec<_>>();
                    cfg.remove_edge(bb_id, next);
                    for p in pred.iter().cloned() {
                        trace!("replace jump: {}::{} ==>> {}", p, bb_id, next);
                        func.bb_get_mut(p).branch.replace_target(bb_id, next);
                        cfg.add_edge(p, next, ());
                        pending.push_back(p);
                    }
                    for &id in phis.get_vec(&bb_id).into_iter().flatten() {
                        if let InstKind::Phi(phi) = &mut func.inst_get_mut(id).kind {
                            let source = phi.remove(&bb_id).unwrap();
                            phi.extend(pred.iter().map(|&x| (x, source)));
                        }
                    }
                    func.bb_detach(bb_id);
                }

                br if bb.is_empty() => {
                    let br = br.clone();
                }

                _ => {}
            }

            if vis.insert(bb_id) {
                let bb = func.bb_get(bb_id);
                for target in bb.branch.target_iter() {
                    pending.push_back(target);
                }
            }
        }
    }
}

fn replace_phis(phis: &MultiMap<BBId, InstId>, func: &mut TacFunc, replace: BBId, with: BBId) {
    for &id in phis.get_vec(&replace).into_iter().flatten() {
        func.inst_get_mut(id).kind.replace_phi_source(replace, with);
    }
}
