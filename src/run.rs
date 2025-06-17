use crate::{
    context::Context,
    error::ApplicationError,
    ir::{Build, BuildId, Rule},
};
use async_recursion::async_recursion;
use futures::future::{FutureExt, Shared, try_join_all};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::{
    io::{AsyncWriteExt, stderr, stdout},
    spawn,
    time::Instant,
    try_join,
};

type RawBuildFuture = Pin<Box<dyn Future<Output = Result<(), ApplicationError>> + Send>>;
pub(crate) type BuildFuture = Shared<RawBuildFuture>;

pub async fn run(context: &Arc<Context>) -> Result<(), ApplicationError> {
    println!("Running with configuration: {:#?}", context.configuration);

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
async fn trigger_build(context: Arc<Context>, build: &Arc<Build>) -> Result<(), ApplicationError> {
    context.build_futures.entry(build.id()).or_insert_with(|| {
        println!(
            "Starting build {}: {:?}",
            build.id(),
            build.rule().and_then(|x| x.description())
        );
        let future: RawBuildFuture = Box::pin(spawn_build(context.clone(), build.clone()));

        future.shared()
    });

    Ok(())
}

async fn spawn_build(context: Arc<Context>, build: Arc<Build>) -> Result<(), ApplicationError> {
    spawn(async move {
        let mut futures = vec![];

        // Make sure we have all our dependencies.
        for input in build.dependencies().iter() {
            futures.push(build_input(context.clone(), input).await?);
        }
        println!(
            "Build {} is waiting for some futures: {:?}",
            build.id(),
            futures
        );

        try_join_all(futures).await?;
        println!("Build {} is now running", build.id());

        // OK, we are ready.
        if let Some(rule) = build.rule() {
            run_rule(&context, rule).await?;
        }

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

async fn run_rule(context: &Context, rule: &Rule) -> Result<(), ApplicationError> {
    let ((output, _duration), _console) = try_join!(
        async {
            let start_time = Instant::now();
            let output = context.run(rule.command()).await?;

            Ok::<_, ApplicationError>((output, Instant::now() - start_time))
        },
        async {
            let console = context.console().lock().await;

            if let Some(description) = rule.description() {
                stderr().write_all(description.as_bytes()).await?;
                stderr().write_all(b"\n").await?;
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
