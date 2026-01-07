use std::sync::Arc;

use petgraph::{Graph, graph::NodeIndex, visit::EdgeRef};

use crate::buildsystem::{
    Operation, OperationOutput, output::RawOperationOutput, sourcesink::SourceSink,
};
use crate::error::ApplicationError;

pub type BuildStep = Arc<Box<dyn Operation>>;

pub struct BuildGraph {
    graph: Graph<Arc<Box<dyn Operation + 'static>>, OperationOutput>,
    pub source: NodeIndex,
    pub sinks: Vec<NodeIndex>,
}

impl BuildGraph {
    pub fn new() -> Self {
        let mut g = Graph::new();
        let source_node: Box<dyn Operation + 'static> = Box::new(SourceSink::Source);
        let source = g.add_node(Arc::new(source_node));
        let sinks = vec![];
        Self {
            graph: g,
            source,
            sinks,
        }
    }

    pub fn externals(&self, direction: petgraph::Direction) -> impl Iterator<Item = NodeIndex> {
        self.graph.externals(direction)
    }
    pub fn node_weight(&self, index: NodeIndex) -> Option<&BuildStep> {
        self.graph.node_weight(index)
    }
    pub fn edges_directed(
        &'_ self,
        index: NodeIndex,
        direction: petgraph::Direction,
    ) -> impl Iterator<Item = petgraph::graph::EdgeReference<'_, OperationOutput>> {
        self.graph.edges_directed(index, direction)
    }

    pub fn add_path<S: AsRef<str>>(
        &mut self,
        source_filename: &str,
        operations: Vec<(Option<S>, BuildStep)>,
        sink_filename: &str,
    ) {
        let mut current_node = self.source;
        for (index, (input_filename, op)) in operations.into_iter().enumerate() {
            let output = if let Some(input_filename) = input_filename {
                RawOperationOutput::from(input_filename.as_ref()).into()
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
                // If there's a sink node from this node, use that as the current instead.
                if let Some(sink_node) = self
                    .graph
                    .edges_directed(current_node, petgraph::Direction::Outgoing)
                    .find(|edge| {
                        self.sinks.contains(&edge.target())
                    })
                    .map(|edge| edge.target())
                {
                    current_node = sink_node;
                }
                continue;
            }
            // Otherwise, we add a new node for this operation.
            let next_node = self.graph.add_node(op);
            self.graph.update_edge(current_node, next_node, output);
            current_node = next_node;
        }
        let final_output = RawOperationOutput::from(sink_filename).into();
        // Create a sink node and add it to the list of sinks
        let sink_node = self.graph.add_node(Arc::new(Box::new(
            SourceSink::Sink,
        )));
        self.graph
            .update_edge(current_node, sink_node, final_output);
        self.sinks.push(sink_node);
    }

    pub fn ensure_directories(&self) -> Result<(), ApplicationError> {
        for edge in self.graph.raw_edges() {
            if edge.weight.is_named_file()
                && let Some(parent) = std::path::Path::new(&edge.weight.to_filename()?).parent()
            {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ApplicationError::Other(format!(
                        "Could not create directory {}: {}",
                        parent.display(),
                        e
                    ))
                })?;
            }
        }
        Ok(())
    }

    #[cfg(feature = "graphviz")]
    pub fn draw(&self) -> Result<String, ApplicationError> {
        let contents = format!("{}", petgraph::dot::Dot::new(&self.graph));
        let mut parser = layout::gv::DotParser::new(&contents);
        let tree = parser
            .process()
            .map_err(|e| ApplicationError::Other(format!("Could not parse graph: {e}")))?;
        let mut gb = layout::gv::GraphBuilder::new();
        gb.visit_graph(&tree);
        let mut vg = gb.get();
        let mut svg = layout::backends::svg::SVGWriter::new();
        vg.do_it(false, false, false, &mut svg);
        let svg_contents = svg.finalize();
        Ok(svg_contents)
    }

    pub fn ascii(&self) -> Result<String, ApplicationError> {
        // In ascii_dag we can't put a label on an edge. To get around that,
        // we create another petgraph where as well as the original nodes,
        // each edge in self.graph becomes a node, and we add edges from
        // the source node to the edge node, and from the edge node to the
        // target node.
        let mut graph: Graph<String, ()> = Graph::new();
        // First let's copy what we need to know about the nodes
        for index in self.graph.node_indices() {
            let op = self.graph.node_weight(index).unwrap();
            graph.add_node(op.shortname().to_string());
        }
        // Now let's add nodes for the edges
        for edge in self.graph.raw_edges() {
            let edge_node = graph.add_node(format!("{}", edge.weight));
            graph.add_edge(edge.source(), edge_node, ());
            graph.add_edge(edge_node, edge.target(), ());
        }

        // And now we can create the nodes and edges for ascii_dag.
        let nodes: Vec<(usize, &str)> = graph
            .node_indices()
            .map(|index| (index.index(), graph[index].as_str()))
            .collect();
        let edges: Vec<(usize, usize)> = graph
            .raw_edges()
            .iter()
            .map(|edge| (edge.source().index(), edge.target().index()))
            .collect();
        let dag = ascii_dag::DAG::from_edges(&nodes, &edges);
        let contents = dag.render();
        Ok(contents)
    }
}
