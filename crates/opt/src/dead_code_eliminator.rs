use std::collections::{HashSet, VecDeque};

use crate::util::graphs::{cfg, dfg};
use azuki_tac::{builder::FuncEditor, optimizer::FunctionOptimizer, Branch, InstId, Value};
use petgraph::{algo::dominators, graphmap::DiGraphMap, visit};
use tracing::{debug, debug_span, trace};
use visit::Walker;

pub struct DeadCodeEliminator;

impl DeadCodeEliminator {
    pub fn new() -> DeadCodeEliminator {
        DeadCodeEliminator
    }
}

impl Default for DeadCodeEliminator {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionOptimizer for DeadCodeEliminator {
    fn name(&self) -> std::borrow::Cow<str> {
        "dead-code-eliminator".into()
    }

    fn optimize_func(
        &mut self,
        _env: &mut azuki_tac::optimizer::OptimizeEnvironment,
        func: &mut azuki_tac::TacFunc,
    ) {
        let _span = debug_span!("dead-code-eliminator", %func.name).entered();

        if func.starting_block().is_none() {
            debug!("Function does not have a starting block");
            return;
        }

        let mut graph = dfg(func);
        let bb_graph = cfg(func);
        let mut find_roots = VecDeque::new();

        debug!("Constructing reference map");
        // Construct instruction reference map
        for (idx, _, inst) in func.all_inst_unordered() {
            for source in inst.kind.param_op_iter() {
                graph.add_edge(idx, source, ());
            }
        }
        for (_, bb) in func.all_bb_unordered() {
            if let Branch::Return(Some(Value::Dest(idx))) = &bb.branch {
                find_roots.push_back(*idx);
            }
        }

        debug!("Constructing dominator information");

        let dominators = dominators::simple_fast(&bb_graph, func.starting_block().unwrap());

        debug!("Finding variables that can be reached from root");

        let mut retained = HashSet::new();
        let mut vis_bb = HashSet::new();
        let mut dfs = petgraph::visit::Dfs::empty(&graph);
        while let Some(root) = find_roots.pop_front() {
            trace!("Searching from root %{}", root.slot());
            dfs.move_to(root);
            retained.insert(root);
            for point in (&mut dfs).iter(&graph) {
                retained.insert(point);

                // Add all dominator basic blocks into root set
                let bb_id = func.tac_get(point).bb;
                if vis_bb.insert(bb_id) {
                    let dom = dominators.strict_dominators(bb_id);
                    for dom in dom.into_iter().flatten() {
                        let bb = func.bb_get(dom);
                        if let Branch::CondJump {
                            cond: Value::Dest(x),
                            ..
                        } = &bb.branch
                        {
                            trace!(
                                "Adding %{} into root set since its block bb{} dominates bb{}",
                                x.slot(),
                                dom.unique_num(),
                                bb_id.unique_num()
                            );
                            find_roots.push_back(*x);
                        }
                    }
                }
            }
        }

        // Remove unused instruction.

        let mut editor = FuncEditor::new(func);
        let bbs = editor
            .func
            .all_bb_unordered()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for bb in bbs.iter().cloned() {
            editor.set_current_bb(bb);
            let mut has_next = editor.move_forward();
            while has_next {
                if !retained.contains(&editor.current_idx().unwrap()) {
                    trace!("removed %{}", editor.current_idx().unwrap().slot(),);
                    has_next = editor.remove_current().0;
                } else {
                    has_next = editor.move_forward();
                }
            }
        }
    }

    fn edits_program(&self) -> bool {
        true
    }
}
