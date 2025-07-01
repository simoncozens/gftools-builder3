use std::sync::Arc;

use petgraph::dot::Dot;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph};

use crate::{
    operations::{Operation, OperationOutput, RawOperationOutput},
    SourceSink,
};

pub type BuildStep = Arc<Box<dyn Operation>>;

pub struct BuildGraph {
    graph: Graph<Arc<Box<dyn Operation + 'static>>, OperationOutput>,
    pub source: NodeIndex,
    pub sink: NodeIndex,
}

impl BuildGraph {
    pub fn new() -> Self {
        let mut g = Graph::new();
        let source_node: Box<dyn Operation + 'static> = Box::new(SourceSink::Source);
        let sink_node: Box<dyn Operation + 'static> = Box::new(SourceSink::Sink);
        let source = g.add_node(Arc::new(source_node));
        let sink = g.add_node(Arc::new(sink_node));
        Self {
            graph: g,
            source,
            sink,
        }
    }

    pub fn externals(&self, direction: petgraph::Direction) -> impl Iterator<Item = NodeIndex> {
        self.graph.externals(direction)
    }
    pub fn node_weight(&self, index: NodeIndex) -> Option<&BuildStep> {
        self.graph.node_weight(index)
    }
    pub fn edges_directed(
        &self,
        index: NodeIndex,
        direction: petgraph::Direction,
    ) -> impl Iterator<Item = petgraph::graph::EdgeReference<OperationOutput>> {
        self.graph.edges_directed(index, direction)
    }

    pub fn add_path(
        &mut self,
        source_filename: &str,
        operations: Vec<(BuildStep, Option<&str>)>,
        sink_filename: &str,
    ) {
        let mut current_node = self.source;
        for (index, (op, intermediate_filename)) in operations.into_iter().enumerate() {
            let output = if let Some(intermediate_filename) = intermediate_filename {
                RawOperationOutput::from(intermediate_filename).into()
            } else if index == 0 {
                RawOperationOutput::from(source_filename).into()
            } else {
                RawOperationOutput::TemporaryFile(None).into()
            };
            // If there is an outgoing edge into the same operation with the same output, we use that node.
            if let Some(existing_node) = self
                .graph
                .edges_directed(current_node, petgraph::Direction::Outgoing)
                .find(|edge| {
                    edge.weight() == &output
                        && self.graph[edge.target()].shortname() == op.shortname()
                    // XXX This assumes that the operation's shortname is unique. (No parameters, etc.)
                })
                .map(|edge| edge.target())
            {
                current_node = existing_node;
                continue;
            }
            // Otherwise, we add a new node for this operation.
            let next_node = self.graph.add_node(op);
            self.graph.update_edge(current_node, next_node, output);
            current_node = next_node;
        }
        let final_output = RawOperationOutput::from(sink_filename).into();
        self.graph
            .update_edge(current_node, self.sink, final_output);
    }

    pub fn draw(&self) -> String {
        format!("{}", Dot::new(&self.graph))
    }
}
