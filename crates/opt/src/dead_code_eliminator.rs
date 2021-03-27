use std::collections::{BTreeSet, HashSet, VecDeque};

use azuki_tac::{builder::FuncEditor, optimizer::FunctionOptimizer, BBId, Branch, InstId, Value};
use petgraph::{
    algo::dominators,
    graphmap::{DiGraphMap, GraphMap},
    visit::{self, GraphBase, GraphRef},
    Directed,
    EdgeDirection::Incoming,
};
use tracing::{debug, debug_span, info, info_span, trace};
use visit::{FilterNode, Walker};

pub struct DeadCodeEliminator {
    graph: DiGraphMap<InstId, ()>,
    find_roots: VecDeque<InstId>,
}

impl DeadCodeEliminator {
    pub fn new() -> DeadCodeEliminator {
        DeadCodeEliminator {
            graph: DiGraphMap::new(),
            find_roots: VecDeque::new(),
        }
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

        let mut bb_graph = DiGraphMap::new();

        debug!("Constructing reference map");
        // Construct instruction reference map
        for (idx, _bb, inst) in func.all_inst_unordered() {
            for source in inst.kind.param_op_iter() {
                self.graph.add_edge(idx, source, ());
            }
        }
        for (id, bb) in func.all_bb_unordered() {
            for next in bb.branch.target_iter() {
                bb_graph.add_edge(id, next, ());
            }
            if let Branch::Return(Some(Value::Dest(idx))) = &bb.branch {
                self.find_roots.push_back(*idx);
            }
        }

        debug!("Constructing dominator information");

        let dominators = dominators::simple_fast(&bb_graph, func.starting_block().unwrap());

        debug!("Finding variables that can be reached from root");

        let mut retained = HashSet::new();
        let mut vis_bb = HashSet::new();
        let mut dfs = petgraph::visit::Dfs::empty(&self.graph);
        while let Some(root) = self.find_roots.pop_front() {
            dfs.move_to(root);
            retained.insert(root);
            for point in (&mut dfs).iter(&self.graph) {
                retained.insert(point);

                // Add basic block to root if it affects return code.
                let bb_id = func.tac_get(point).bb;
                if vis_bb.insert(bb_id) {
                    let dom = dominators.strict_dominators(bb_id);
                    for dom in dom.into_iter().flatten() {
                        trace!(
                            "Adding bb{} into root set since it dominates bb{}",
                            dom.unique_num(),
                            bb_id.unique_num()
                        );
                        for pred in bb_graph.neighbors_directed(dom, Incoming) {
                            let bb = func.bb_get(pred);
                            if let Branch::CondJump {
                                cond: Value::Dest(x),
                                ..
                            } = &bb.branch
                            {
                                self.find_roots.push_back(*x);
                            }
                        }
                    }
                }
            }
        }

        // Remove unused instruction
        // Note: this part may remove the condition variable of some basic block.
        // This is intended, and the basic blocks having invalid conditions

        let mut editor = FuncEditor::new(func);
        let bbs = editor
            .func
            .all_bb_unordered()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        for bb in bbs {
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

        // Compact bas
    }

    fn reset(&mut self) {
        self.graph.clear();
        self.find_roots.clear();
    }

    fn edits_program(&self) -> bool {
        true
    }
}
