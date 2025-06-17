use std::{error::Error, process::Output, sync::Arc};

use crate::{
    error::ApplicationError,
    ir::{BuildId, Configuration},
    run::BuildFuture,
};
use dashmap::DashMap;
use tokio::{
    process::Command,
    sync::{Mutex, Semaphore},
};

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
