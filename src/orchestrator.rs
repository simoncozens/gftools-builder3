//! Build orchestrator module
//!
//! This code was heavily, heavily adopted from aviqqe/turtle-build.
//! Many thanks to Yota Toyama for making this code available under the MIT/Apache licenses.
//! A parallel build system in just under 200 lines of Rust is astonishing.
use crate::{error::ApplicationError, graph::{BuildGraph, BuildStep}, operations::OperationOutput};
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

pub async fn run(context: &Arc<Context>) -> Result<(), ApplicationError> {
    // Work out the final targets.
    let final_targets: HashSet<NodeIndex> =
        HashSet::from_iter(context.configuration.graph().externals(Direction::Outgoing));

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
    context.build_futures.entry(build).or_insert_with(|| {
        let future: RawBuildFuture = Box::pin(spawn_build(context.clone(), build));
        future.shared()
    });

    Ok(())
}

async fn spawn_build(context: Arc<Context>, index: NodeIndex) -> Result<(), ApplicationError> {
    spawn(async move {
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
        let output_files: Vec<OperationOutput> = context
            .configuration
            .graph()
            .edges_directed(index, Direction::Outgoing)
            .map(|edge| edge.weight().clone())
            .collect::<Vec<_>>();
        for input_dependency in in_edges {
            futures.push(build_input(context.clone(), input_dependency.source()).await?);
            input_files.push(input_dependency.weight().clone());
        }
        try_join_all(futures).await?;

        // OK, we are ready.
        run_op(&context, build, &input_files, &output_files).await?;

        Ok(())
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
    let description = format!(
        "{}: {} -> {}",
        op.shortname(),
        inputs.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(", "),
        outputs.iter().map(|x| format!("{x}")).collect::<Vec<_>>().join(", ")
    );
    let ((output, _duration), _console) = try_join!(
        async {
            let start_time = Instant::now();
            if !inputs.is_empty() && !outputs.is_empty() {
                stderr()
                    .write_all(format!("Running {}\n", &description).as_bytes())
                    .await?;
            }
            let output = context
                .run_with_semaphore(|| op.execute(inputs, outputs))
                .await?;

            Ok::<_, ApplicationError>((output, Instant::now() - start_time))
        },
        async {
            let console = context.console().lock().await;
            if !inputs.is_empty() && !outputs.is_empty() {
                stderr()
                    .write_all(format!("{}\n", &description).as_bytes())
                    .await?;
            }
            // debug!(context, console, "command: {}", rule.command());

            Ok(console)
        }
    )?;

    // profile!(context, console, "duration: {}ms", duration.as_millis());

    if !output.status.success() {
        stdout().write_all(&output.stdout).await?;
        stderr().write_all(&output.stderr).await?;
        return Err(ApplicationError::Build);
    }

    Ok(())
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

// #[allow(dead_code)]
// async fn run_cross_platform(command: &str) -> Result<Output, std::io::Error> {
//     if cfg!(target_os = "windows") {
//         let components = command.split_whitespace().collect::<Vec<_>>();
//         Command::new(components[0])
//             .args(&components[1..])
//             .output()
//             .await
//     } else {
//         Command::new("sh").arg("-ec").arg(command).output().await
//     }
// }
