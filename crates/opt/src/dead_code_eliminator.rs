use std::collections::{HashSet, VecDeque};

use azuki_tac::{builder::FuncEditor, optimizer::FunctionOptimizer, Branch, InstId, Value};
use petgraph::{algo::dominators, graphmap::DiGraphMap, visit};
use tracing::{debug, debug_span, trace};
use visit::Walker;

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

        debug!("Constructing reference map");
        // Construct instruction reference map
        for (idx, _bb, inst) in func.all_inst_unordered() {
            for source in inst.kind.param_op_iter() {
                self.graph.add_edge(idx, source, ());
            }
        }
        for (_, bb) in func.all_bb_unordered() {
            if let Branch::Return(Some(Value::Dest(idx))) = &bb.branch {
                self.find_roots.push_back(*idx);
            } else if let Branch::CondJump {
                cond: Value::Dest(x),
                ..
            } = &bb.branch
            {
                // TODO: Add condition to find root only if it contributes to return value
                self.find_roots.push_back(*x);
            }
        }

        debug!("Finding reachable variables");
        // calculate spanning tree
        let mut retained = HashSet::new();
        let mut dfs = petgraph::visit::Dfs::empty(&self.graph);
        while let Some(root) = self.find_roots.pop_front() {
            dfs.move_to(root);
            retained.insert(root);
            for point in (&mut dfs).iter(&self.graph) {
                retained.insert(point);
            }
        }

        // Remove unused instruction.

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
    }

    fn reset(&mut self) {
        self.graph.clear();
        self.find_roots.clear();
    }

    fn edits_program(&self) -> bool {
        true
    }
}
