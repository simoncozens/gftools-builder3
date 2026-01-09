use std::fmt::Display;
use std::sync::Arc;

use petgraph::{Graph, graph::NodeIndex, visit::EdgeRef};

use crate::buildsystem::{
    Operation, OperationOutput, output::RawOperationOutput, sourcesink::SourceSink,
};
use crate::error::ApplicationError;

pub type BuildStep = Arc<Box<dyn Operation>>;

/// An edge in the build graph, representing data flow from one operation to another.
/// The edge specifies which output slot from the source operation it consumes.
#[derive(Clone)]
pub struct BuildEdge {
    /// The actual data/file being passed
    pub output: OperationOutput,
    /// Which output slot from the source operation (0-indexed)
    pub output_slot: usize,
}

impl Display for BuildEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.output_slot, self.output)
    }
}

pub struct BuildGraph {
    graph: Graph<Arc<Box<dyn Operation + 'static>>, BuildEdge>,
    pub source: NodeIndex,
    pub sinks: Vec<NodeIndex>,
    /// Maps target names to their final operation node (before the sink)
    target_nodes: std::collections::HashMap<String, NodeIndex>,
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
            target_nodes: std::collections::HashMap::new(),
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
    ) -> impl Iterator<Item = petgraph::graph::EdgeReference<'_, BuildEdge>> {
        self.graph.edges_directed(index, direction)
    }

    pub fn add_path<S: AsRef<str>>(
        &mut self,
        source_filename: &str,
        operations: Vec<(Option<S>, BuildStep)>,
        sink_filename: &str,
    ) -> Vec<NodeIndex> {
        use crate::buildsystem::operation::DataKind;
        use crate::operations::convert::{BytesToTempFile, FileToBytes};
        let mut current_node = self.source;
        // Track the current data kind flowing out of current_node (slot 0)
        let mut current_kind: DataKind = DataKind::Path; // source produces paths
        
        // Track the operation nodes we add (not including converters)
        let mut op_nodes: Vec<NodeIndex> = Vec::new();
        
        for (index, (input_filename, op)) in operations.into_iter().enumerate() {
            // Determine default output placeholder based on current_kind
            let default_output_for_kind = |k: DataKind| -> OperationOutput {
                match k {
                    DataKind::Path => RawOperationOutput::TemporaryFile(None).into(),
                    DataKind::Bytes => RawOperationOutput::InMemoryBytes(Vec::new()).into(),
                    _ => RawOperationOutput::InMemoryBytes(Vec::new()).into(),
                }
            };

            // If this is the first step and an explicit file is provided, override
            let computed_output: OperationOutput = if let Some(input_filename) = input_filename {
                RawOperationOutput::from(input_filename.as_ref()).into()
            } else if index == 0 {
                RawOperationOutput::from(source_filename).into()
            } else {
                default_output_for_kind(current_kind)
            };

            let is_source = current_node == self.source;

            // If this node already has outgoing edges, reuse their output to broadcast
            // to any new downstream operations — except for the Source node, which must
            // keep per-source outputs distinct.
            let broadcast_output = if is_source {
                computed_output.clone()
            } else {
                self.graph
                    .edges_directed(current_node, petgraph::Direction::Outgoing)
                    .next()
                    .map(|edge| edge.weight().output.clone())
                    .unwrap_or_else(|| computed_output.clone())
            };

            // Insert conversion if needed based on op's declared input kind
            let want_kind = op.input_kinds().first().cloned().unwrap_or(DataKind::Any);
            let need_conversion = !(want_kind == DataKind::Any || want_kind == current_kind);
            if need_conversion {
                // Determine a simple conversion path for now
                let conv: Option<(Box<dyn Operation>, DataKind)> = match (current_kind, want_kind) {
                    (DataKind::Path, DataKind::Bytes) => {
                        Some((Box::new(FileToBytes), DataKind::Bytes))
                    }
                    (DataKind::Bytes, DataKind::Path) => {
                        Some((Box::new(BytesToTempFile), DataKind::Path))
                    }
                    _ => None,
                };
                if let Some((conv_op, new_kind)) = conv {
                    // Check if there's already a converter of this type from current_node
                    let existing_conv = self
                        .graph
                        .edges_directed(current_node, petgraph::Direction::Outgoing)
                        .find(|edge| {
                            if let Some(node_op) = self.graph.node_weight(edge.target()) {
                                node_op.shortname() == conv_op.shortname()
                            } else {
                                false
                            }
                        })
                        .map(|edge| edge.target());
                    
                    let conv_node = if let Some(existing) = existing_conv {
                        // Reuse existing converter
                        existing
                    } else {
                        // Add new conversion node
                        let new_conv_node = self.graph.add_node(Arc::new(conv_op));
                        // Edge from current_node to converter uses the broadcast output
                        self.graph.update_edge(
                            current_node,
                            new_conv_node,
                            BuildEdge {
                                output: broadcast_output.clone(),
                                output_slot: 0,
                            },
                        );
                        new_conv_node
                    };
                    
                    // Advance current node and kind
                    current_node = conv_node;
                    current_kind = new_kind;
                }
            }

            // Check if there's already an outgoing edge to the same operation.
            // For the Source node we also require the same output (file) to avoid
            // collapsing different source files.
            if let Some(existing_node) = self
                .graph
                .edges_directed(current_node, petgraph::Direction::Outgoing)
                .find(|edge| {
                    let same_op = self.graph[edge.target()] == op;
                    if is_source {
                        same_op && edge.weight().output == computed_output
                    } else {
                        same_op
                    }
                })
                .map(|edge| edge.target())
            {
                current_node = existing_node;
                // Still track this as an operation node for the path
                op_nodes.push(existing_node);
                
                // Update current_kind to match the existing node's output kind
                if let Some(ok) = self
                    .graph
                    .node_weight(existing_node)
                    .and_then(|op| op.output_kinds().first().cloned())
                {
                    if ok != DataKind::Any {
                        current_kind = ok;
                    }
                }
                
                continue;
            }

            // Otherwise, add a new node for this operation using the chosen output.
            let next_node = self.graph.add_node(op);
            let edge = BuildEdge {
                output: broadcast_output,
                output_slot: 0, // Default to slot 0 for simple cases
            };
            self.graph.update_edge(current_node, next_node, edge);
            current_node = next_node;
            
            // Track this operation node (not a converter)
            op_nodes.push(next_node);
            
            // Update current_kind to this op's first output kind if specified
            if let Some(ok) = self
                .graph
                .node_weight(current_node)
                .and_then(|op| op.output_kinds().get(0).cloned())
            {
                if ok != DataKind::Any {
                    current_kind = ok;
                }
            }
        }

        // When adding a sink edge, force the output to be the named target file
        // and broadcast that same OperationOutput to all existing outgoing edges.
        let final_output: OperationOutput = RawOperationOutput::from(sink_filename).into();

        // If there are existing outgoing edges, update them to use the named output
        // so downstream consumers see the real target filename (not a temp file).
        let outgoing: Vec<_> = self
            .graph
            .edges_directed(current_node, petgraph::Direction::Outgoing)
            .map(|e| (e.target(), e.weight().output_slot))
            .collect();
        for (target, slot) in outgoing {
            let edge = BuildEdge {
                output: final_output.clone(),
                output_slot: slot,
            };
            self.graph.update_edge(current_node, target, edge);
        }

        // Create a sink node and add it to the list of sinks, using slot 0
        let sink_node = self.graph.add_node(Arc::new(Box::new(SourceSink::Sink)));
        let edge = BuildEdge {
            output: final_output,
            output_slot: 0,
        };
        self.graph.update_edge(current_node, sink_node, edge);
        self.sinks.push(sink_node);
        
        // Track this target's final node (before sink) for dependency resolution
        self.target_nodes.insert(sink_filename.to_string(), current_node);
        
        // Return the list of operation nodes added (in order)
        op_nodes
    }

    /// Add a dependency from a target to a node that needs it as an additional input.
    /// This creates edges from the dependency target's producing node to the dependent node.
    /// For operations that need n inputs and produce n outputs (like BuildStat), this also
    /// redirects the dependency's sink to go through the dependent node.
    /// 
    /// # Arguments
    /// * `target_name` - The name of the target that produces the needed file
    /// * `dependent_node` - The node that needs the target as an additional input
    /// * `input_slot` - Which input slot (0-indexed) this dependency fills
    pub fn add_dependency(
        &mut self,
        target_name: &str,
        dependent_node: NodeIndex,
        input_slot: usize,
    ) -> Result<(), ApplicationError> {
        let producer_node = self.target_nodes.get(target_name).ok_or_else(|| {
            ApplicationError::InvalidRecipe(format!(
                "Dependency target '{}' not found. Make sure it appears in the recipe before it's referenced.",
                target_name
            ))
        })?;

        // Get the output that the producer creates (from any existing edge, or construct it)
        let producer_output = self
            .graph
            .edges_directed(*producer_node, petgraph::Direction::Outgoing)
            .next()
            .map(|edge| edge.weight().output.clone())
            .unwrap_or_else(|| RawOperationOutput::from(target_name).into());

        // Add input edge from producer to dependent, specifying the input slot
        let input_edge = BuildEdge {
            output: producer_output.clone(),
            output_slot: input_slot,
        };
        self.graph.update_edge(*producer_node, dependent_node, input_edge);

        // Find the sink node for this dependency target and redirect it through dependent_node
        // We need to find the sink that's writing to this specific target file
        let sink_edges: Vec<_> = self
            .graph
            .edges_directed(*producer_node, petgraph::Direction::Outgoing)
            .filter(|edge| {
                // Check if this edge goes to a Sink node AND has the matching target output
                if let Some(node_weight) = self.graph.node_weight(edge.target()) {
                    if node_weight.shortname() == "Sink" {
                        // Check if the output filename matches our target
                        if let Ok(filename) = edge.weight().output.to_filename() {
                            return filename == target_name;
                        }
                    }
                }
                false
            })
            .map(|e| (e.target(), e.weight().output.clone()))
            .collect();

        // For each sink, redirect it to go through the dependent node
        for (sink_node, output) in sink_edges {
            // Remove the edge from producer to sink
            if let Some(edge_idx) = self.graph.find_edge(*producer_node, sink_node) {
                self.graph.remove_edge(edge_idx);
            }
            
            // Add edge from dependent_node to sink using the same output slot as the input
            let output_edge = BuildEdge {
                output,
                output_slot: input_slot,
            };
            self.graph.update_edge(dependent_node, sink_node, output_edge);
        }

        Ok(())
    }

    pub fn ensure_directories(&self) -> Result<(), ApplicationError> {
        for edge in self.graph.raw_edges() {
            if edge.weight.output.is_named_file()
                && let Some(parent) =
                    std::path::Path::new(&edge.weight.output.to_filename()?).parent()
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
            let edge_node = graph.add_node(format!("{}", edge.weight.output));
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
