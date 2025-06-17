//! Build orchestrator module
//!
//! This code was heavily, heavily adopted from aviqqe/turtle-build.
//! Many thanks to Yota Toyama for making this code available under the MIT/Apache licenses.
//! A parallel build system in just under 200 lines of Rust is astonishing.
use crate::{error::ApplicationError, operations::Operation};
use async_recursion::async_recursion;
use dashmap::DashMap;
use futures::future::{FutureExt, Shared, try_join_all};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    future::Future,
    hash::Hash,
    pin::Pin,
    process::Output,
    sync::Arc,
};
use tokio::{
    io::{AsyncWriteExt, stderr, stdout},
    process::Command,
    spawn,
    sync::{Mutex, Semaphore},
    time::Instant,
    try_join,
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BuildId(u64);

impl BuildId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Configuration {
    jobs: HashMap<Arc<BuildId>, Arc<Box<dyn Operation>>>,
    final_targets: HashSet<Arc<BuildId>>,
    build_directory: Option<Arc<str>>,
}

impl Configuration {
    pub fn new(final_targets: HashSet<Arc<BuildId>>, build_directory: Option<Arc<str>>) -> Self {
        Self {
            jobs: Default::default(),
            final_targets,
            build_directory,
        }
    }

    pub fn jobs(&self) -> &HashMap<Arc<BuildId>, Arc<Box<dyn Operation>>> {
        &self.jobs
    }

    pub fn final_targets(&self) -> &HashSet<Arc<BuildId>> {
        &self.final_targets
    }

    pub fn add_job(&mut self, build: Box<dyn Operation>) {
        self.jobs.insert(build.id().into(), Arc::new(build));
    }
}

type BuildStep = Arc<Box<dyn Operation>>;

type RawBuildFuture = Pin<Box<dyn Future<Output = Result<(), ApplicationError>> + Send>>;
pub(crate) type BuildFuture = Shared<RawBuildFuture>;

pub async fn run(context: &Arc<Context>) -> Result<(), ApplicationError> {
    for target in context.configuration.final_targets() {
        trigger_build(
            context.clone(),
            context
                .configuration
                .jobs()
                .get(target.as_ref())
                .ok_or(ApplicationError::DefaultOutputNotFound)?,
        )
        .await?;
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
async fn trigger_build(context: Arc<Context>, build: &BuildStep) -> Result<(), ApplicationError> {
    context.build_futures.entry(build.id()).or_insert_with(|| {
        let future: RawBuildFuture = Box::pin(spawn_build(context.clone(), build.clone()));
        future.shared()
    });

    Ok(())
}

async fn spawn_build(context: Arc<Context>, build: BuildStep) -> Result<(), ApplicationError> {
    spawn(async move {
        let mut futures = vec![];

        // Make sure we have all our dependencies.
        for input in build.dependencies().iter() {
            futures.push(build_input(context.clone(), input).await?);
        }
        try_join_all(futures).await?;
        println!("Build {:?} is now running", build.description());

        // OK, we are ready.
        run_op(&context, &build).await?;

        Ok(())
    })
    .await?
}

async fn build_input(
    context: Arc<Context>,
    input: &BuildId,
) -> Result<BuildFuture, ApplicationError> {
    Ok(
        if let Some(build) = context.configuration.jobs().get(input) {
            trigger_build(context.clone(), build).await?;

            context.build_futures.get(&build.id()).unwrap().clone()
        } else {
            panic!("Dependencies needed but I can't find it");
        },
    )
}

async fn run_op(context: &Context, op: &BuildStep) -> Result<(), ApplicationError> {
    let ((output, _duration), _console) = try_join!(
        async {
            let start_time = Instant::now();
            let output = context.run_with_semaphore(|| op.execute()).await?;

            Ok::<_, ApplicationError>((output, Instant::now() - start_time))
        },
        async {
            let console = context.console().lock().await;

            stderr().write_all(op.description().as_bytes()).await?;
            stderr().write_all(b" done\n").await?;

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
    pub build_futures: DashMap<BuildId, BuildFuture>,
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

#[allow(dead_code)]
async fn run_cross_platform(command: &str) -> Result<Output, std::io::Error> {
    if cfg!(target_os = "windows") {
        let components = command.split_whitespace().collect::<Vec<_>>();
        Command::new(components[0])
            .args(&components[1..])
            .output()
            .await
    } else {
        Command::new("sh").arg("-ec").arg(command).output().await
    }
}
