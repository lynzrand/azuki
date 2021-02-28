use std::collections::HashSet;

use azuki_tac::{builder::FuncEditor, optimizer::FunctionOptimizer, Branch, OpRef, Value};
use petgraph::{graphmap::DiGraphMap, visit};
use visit::{FilterNode, Walker};

pub struct DeadCodeEliminator {
    graph: DiGraphMap<OpRef, ()>,
    find_roots: HashSet<OpRef>,
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
        // Construct instruction reference map
        for (idx, _bb, inst) in func.all_inst_unordered() {
            for source in inst.kind.param_op_iter() {
                self.graph.add_edge(source, idx, ());
            }
        }
        for bb in func.basic_blocks.node_indices() {
            let bb = func.basic_blocks.node_weight(bb).unwrap();
            for br in &bb.jumps {
                if let Branch::Return(Some(Value::Dest(idx))) = br {
                    self.find_roots.include_node(*idx);
                }
            }
        }

        // calculate spanning tree
        let mut retained = HashSet::new();
        let mut dfs = petgraph::visit::Dfs::empty(&self.graph);
        for &root in &self.find_roots {
            dfs.move_to(root);
            for point in (&mut dfs).iter(&self.graph) {
                retained.insert(point);
            }
        }

        let mut editor = FuncEditor::new(func);
        let bbs = editor.func.basic_blocks.node_indices().collect::<Vec<_>>();
        for bb in bbs {
            editor.set_current_bb_start(bb).unwrap();
            while editor.move_forward() {
                if !retained.contains(&editor.current_idx().unwrap()) {
                    editor.remove_current();
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
