use std::error::Error;
use std::process::Output;
use std::sync::Arc;

use crate::ir::{BuildId, Configuration};
use crate::run::BuildFuture;
use dashmap::DashMap;
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore};

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

    pub async fn run(&self, command: &str) -> Result<Output, Box<dyn Error>> {
        let permit = self.command_semaphore.acquire().await?;

        let output = if cfg!(target_os = "windows") {
            let components = command.split_whitespace().collect::<Vec<_>>();
            Command::new(components[0])
                .args(&components[1..])
                .output()
                .await?
        } else {
            Command::new("sh").arg("-ec").arg(command).output().await?
        };

        drop(permit);

        Ok(output)
    }
}
