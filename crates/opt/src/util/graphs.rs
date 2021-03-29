//! Graphs

use azuki_tac::{BBId, InstId, TacFunc};
use petgraph::graphmap::DiGraphMap;

/// Generate control flow graph of the given function.
pub fn cfg(f: &TacFunc) -> DiGraphMap<BBId, ()> {
    let mut bb_graph = DiGraphMap::new();

    for (id, bb) in f.all_bb_unordered() {
        for next in bb.branch.target_iter() {
            bb_graph.add_edge(id, next, ());
        }
    }

    bb_graph
}

/// Generate data flow graph of the given function.
pub fn dfg(f: &TacFunc) -> DiGraphMap<InstId, ()> {
    let mut graph = DiGraphMap::new();

    // Construct instruction reference map
    for (idx, _bb, inst) in f.all_inst_unordered() {
        for source in inst.kind.param_op_iter() {
            graph.add_edge(idx, source, ());
        }
    }

    graph
}
