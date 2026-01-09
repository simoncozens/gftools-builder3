//! Build orchestrator module
//!
//! This code was heavily, heavily adopted from aviqqe/turtle-build.
//! Many thanks to Yota Toyama for making this code available under the MIT/Apache licenses.
//! A parallel build system in just under 200 lines of Rust is astonishing.
use crate::{
    buildsystem::{BuildGraph, BuildStep, OperationOutput},
    error::ApplicationError,
};
use async_recursion::async_recursion;
use dashmap::DashMap;
use futures::future::{FutureExt, Shared, try_join_all};
use petgraph::{Direction, graph::NodeIndex, visit::EdgeRef};
use std::{
    collections::HashSet, error::Error, future::Future, pin::Pin, process::Output, sync::Arc,
};
use tokio::{
    io::{AsyncWriteExt, stderr, stdout},
    spawn,
    sync::{Mutex, Semaphore},
    time::Instant,
    try_join,
};
use tracing::{Instrument, debug, info, info_span};

// #[derive(Clone)]
pub struct Configuration {
    graph: BuildGraph,
}

impl Configuration {
    pub fn new(graph: BuildGraph) -> Self {
        Self { graph }
    }

    pub fn graph(&self) -> &BuildGraph {
        &self.graph
    }
}

type RawBuildFuture = Pin<Box<dyn Future<Output = Result<(), ApplicationError>> + Send>>;
pub(crate) type BuildFuture = Shared<RawBuildFuture>;

/// Helper function to get target filenames for a given node index.
/// This traces through outgoing edges to find all named output files associated with this build step.
fn get_target_files(context: &Context, index: NodeIndex) -> Vec<String> {
    let mut targets = vec![];

    // Get immediate outputs
    if let Some(_build) = context.configuration.graph().node_weight(index) {
        for edge in context
            .configuration
            .graph()
            .edges_directed(index, Direction::Outgoing)
        {
            if let Ok(output_lock) = edge.weight().output.lock()
                && let crate::buildsystem::output::RawOperationOutput::NamedFile(name) =
                    &*output_lock
            {
                targets.push(name.clone());
            }
        }
    }

    // If we have no targets yet, look further downstream for named files
    if targets.is_empty() {
        let mut to_visit = vec![index];
        let mut visited = HashSet::new();

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            for edge in context
                .configuration
                .graph()
                .edges_directed(current, Direction::Outgoing)
            {
                if let Ok(output_lock) = edge.weight().output.lock()
                    && let crate::buildsystem::output::RawOperationOutput::NamedFile(name) =
                        &*output_lock
                {
                    targets.push(name.clone());
                    break; // Found a named output, stop for this path
                }
                to_visit.push(edge.target());
            }
        }
    }

    targets
}

pub async fn run(graph: BuildGraph, job_limit: usize) -> Result<(), ApplicationError> {
    let configuration = Configuration::new(graph);
    let context = Arc::new(Context::new(job_limit, Arc::new(configuration)));
    // Work out the final targets.
    let final_targets: HashSet<NodeIndex> =
        HashSet::from_iter(context.configuration.graph().externals(Direction::Outgoing));

    log::info!("Starting build for {} targets", final_targets.len());

    for target in final_targets {
        trigger_build(context.clone(), target).await?;
    }

    // Do not inline this to avoid borrowing a lock of builds.
    let futures = context
        .build_futures
        .iter()
        .map(|r#ref| r#ref.value().clone())
        .collect::<Vec<_>>();

    let result = try_join_all(futures).await;

    result.map(|_| ())
}

#[async_recursion]
async fn trigger_build(context: Arc<Context>, build: NodeIndex) -> Result<(), ApplicationError> {
    let targets = get_target_files(&context, build);
    let targets_str = targets.join(", ");
    let span = info_span!("trigger_build", targets = %targets_str);

    context.build_futures.entry(build).or_insert_with(|| {
        let context_clone = context.clone();
        let future: RawBuildFuture =
            Box::pin(spawn_build(context_clone, build).instrument(span.clone()));
        future.shared()
    });

    Ok(())
}

async fn spawn_build(context: Arc<Context>, index: NodeIndex) -> Result<(), ApplicationError> {
    spawn(async move {
        let targets = get_target_files(&context, index);
        let targets_str = targets.join(", ");
        let span = info_span!("Building",
            operation = %context.configuration.graph().node_weight(index).map(|op| op.shortname()).unwrap_or("unknown"),
            targets = %targets_str
        );

        async {
            let build = context
                .configuration
                .graph()
                .node_weight(index)
                .expect("Build step not found in graph");
            let mut futures = vec![];

            // Make sure we have all our dependencies. (in-edges of this index)
            let in_edges = context
                .configuration
                .graph()
                .edges_directed(index, Direction::Incoming);
            let mut input_files = vec![];
                        // Collect outputs by slot. Multiple edges may reference the same slot (broadcasting).
            // We need to build a Vec where outputs[slot] contains the OperationOutput for that slot.
            let out_edges: Vec<_> = context
                .configuration
                .graph()
                .edges_directed(index, Direction::Outgoing)
                .collect();
            // Find the maximum slot number to size our output vector
            let max_slot = out_edges.iter().map(|e| e.weight().output_slot).max().unwrap_or(0);
            let mut output_files = vec![None; max_slot + 1];
            // Fill in the output slots - if multiple edges use the same slot, they share the same OperationOutput
            for edge in out_edges {
                let slot = edge.weight().output_slot;
                if output_files[slot].is_none() {
                    output_files[slot] = Some(edge.weight().output.clone());
                }
            }

            // Convert to non-Option vec (all slots should be filled)
            let output_files: Vec<OperationOutput> = output_files.into_iter().flatten().collect();
            for input_dependency in in_edges {
                futures.push(build_input(context.clone(), input_dependency.source()).await?);
                input_files.push(input_dependency.weight().output.clone());
            }
            try_join_all(futures).await?;

            // OK, we are ready.
            run_op(&context, build, &input_files, &output_files).await?;

            Ok::<(), ApplicationError>(())
        }
        .instrument(span)
        .await
    })
    .await?
}

async fn build_input(
    context: Arc<Context>,
    input: NodeIndex,
) -> Result<BuildFuture, ApplicationError> {
    trigger_build(context.clone(), input).await?;
    context
        .build_futures
        .get(&input)
        .ok_or(ApplicationError::Build)
        .map(|f| f.clone())
        .map_err(|_| ApplicationError::Build)
}

async fn run_op(
    context: &Context,
    op: &BuildStep,
    inputs: &[OperationOutput],
    outputs: &[OperationOutput],
) -> Result<(), ApplicationError> {
    let output_strs: Vec<String> = outputs.iter().map(|o| o.to_string()).collect();
    let outputs_str = output_strs.join(", ");

    let span = info_span!(
        "run_op",
        operation = %op.shortname(),
        targets = %outputs_str
    );

    let description = format!(
        "{}: {} -> {} ({})",
        op.shortname(),
        inputs
            .iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(", "),
        outputs
            .iter()
            .map(|x| format!("{x}"))
            .collect::<Vec<_>>()
            .join(", "),
        op.description()
    );

    let inner = async {
        let ((output, duration), _console) = try_join!(
            async {
                let start_time = Instant::now();
                if !inputs.is_empty() && !outputs.is_empty() && !op.hidden() {
                    println!("{}", &description);
                }
                let output = context
                    .run_with_semaphore(|| op.execute(inputs, outputs))
                    .await?;

                let elapsed = Instant::now() - start_time;
                Ok::<_, ApplicationError>((output, elapsed))
            },
            async {
                let console = context.console().lock().await;
                // if !inputs.is_empty() && !outputs.is_empty() && !op.hidden() {
                //     stderr()
                //         .write_all(format!("Completed {}\n", &description).as_bytes())
                //         .await?;
                // }
                // debug!(context, console, "command: {}", rule.command());

                Ok(console)
            }
        )?;

        // Emit profiling event with duration for trace analysis
        info!(
            duration_ms = duration.as_millis() as u64,
            "Operation completed: {}", &description
        );

        if !output.status.success() {
            stdout().write_all(&output.stdout).await?;
            stderr().write_all(&output.stderr).await?;
            return Err(ApplicationError::Build);
        }

        Ok::<(), ApplicationError>(())
    };

    inner.instrument(span).await
}

pub struct Context {
    command_semaphore: Semaphore,
    /// Just a thing that you lock to print to the console.
    console: Mutex<()>,
    pub configuration: Arc<Configuration>,
    pub build_futures: DashMap<NodeIndex, BuildFuture>,
}

impl Context {
    pub fn new(job_limit: usize, configuration: Arc<Configuration>) -> Self {
        Self {
            command_semaphore: Semaphore::new(job_limit),
            console: Mutex::new(()),
            configuration,
            build_futures: DashMap::new(),
        }
    }

    pub fn console(&self) -> &Mutex<()> {
        &self.console
    }

    pub async fn run_with_semaphore(
        &self,
        fnc: impl Fn() -> Result<Output, ApplicationError>,
    ) -> Result<Output, Box<dyn Error>> {
        let permit = self.command_semaphore.acquire().await?;
        let output = fnc()?;

        drop(permit);

        Ok(output)
    }
}
