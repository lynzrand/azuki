use std::hash::Hash;
use std::{collections::HashMap, fmt::Debug};

use petgraph::{
    visit::{GraphRef, IntoNeighborsDirected, VisitMap, Visitable, Walker},
    EdgeDirection::Outgoing,
};

pub struct BiasedRevPostOrderDfs<TNode, TVisit> {
    stack: Vec<TNode>,
    counter: HashMap<TNode, usize>,
    visited: TVisit,
}

impl<TNode, TVisit> BiasedRevPostOrderDfs<TNode, TVisit>
where
    TNode: Clone + Eq + Hash + Debug,
    TVisit: VisitMap<TNode>,
{
    pub fn new<TGraph>(graph: TGraph, start: TNode) -> BiasedRevPostOrderDfs<TNode, TVisit>
    where
        TGraph: GraphRef
            + Visitable<NodeId = TNode, Map = TVisit>
            + IntoNeighborsDirected<NodeId = TNode>,
    {
        let mut map = Self::empty(graph);
        map.move_to(start);
        map
    }

    pub fn empty<TGraph>(graph: TGraph) -> BiasedRevPostOrderDfs<TNode, TVisit>
    where
        TGraph: GraphRef + Visitable<NodeId = TNode, Map = TVisit>,
    {
        BiasedRevPostOrderDfs {
            stack: vec![],
            counter: HashMap::new(),
            visited: graph.visit_map(),
        }
    }

    pub fn move_to(&mut self, node: TNode) {
        self.stack.clear();
        // TODO: Calculate cycles
        self.stack.push(node.clone());
        self.counter.insert(node, 1);
    }

    pub fn next<TGraph>(&mut self, graph: TGraph) -> Option<TNode>
    where
        TGraph: GraphRef
            + Visitable<NodeId = TNode, Map = TVisit>
            + IntoNeighborsDirected<NodeId = TNode>,
    {
        // count == None && visited == false ==>> Not visited yet
        // count == Some && visited == true  ==>> Revisiting
        // count == None && visited == true  ==>> Visited
        while let Some(node) = self.stack.pop() {
            let visited = !self.visited.visit(node.clone());
            let count = if !visited {
                self.counter.entry(node.clone()).or_insert_with(|| {
                    graph
                        .neighbors_directed(node.clone(), petgraph::EdgeDirection::Incoming)
                        .count()
                })
            } else {
                match self.counter.get_mut(&node) {
                    Some(x) => x,
                    // This node is already fully visited. Might be a loop to
                    // start node, anyway we'll ignore it.
                    _ => continue,
                }
            };

            *count -= 1;
            eprintln!("{:?} {}", &node, *count);

            if *count == 0 {
                // this node is completely visited
                self.counter.remove(&node);

                // add its proceeding nodes
                for proceeding in graph.neighbors_directed(node.clone(), Outgoing) {
                    eprintln!("> {:?} ", &proceeding);
                    self.stack.push(proceeding);
                }

                // emit node
                return Some(node);
            }
        }
        None
    }
}

impl<TNode, TVisit, TGraph> Walker<TGraph> for BiasedRevPostOrderDfs<TNode, TVisit>
where
    TNode: Clone + Eq + Hash + Debug,
    TGraph:
        GraphRef + Visitable<NodeId = TNode, Map = TVisit> + IntoNeighborsDirected<NodeId = TNode>,
    TVisit: VisitMap<TNode>,
{
    type Item = TNode;

    fn walk_next(&mut self, context: TGraph) -> Option<Self::Item> {
        self.next(context)
    }
}
