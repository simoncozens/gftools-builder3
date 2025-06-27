pub mod buildstatic;
pub mod buildvariable;
pub mod glyphs2ufo;

use crate::error::ApplicationError;
use async_trait::async_trait;
use dashmap::DashMap;
use std::{
    hash::{Hash, Hasher},
    os::unix::process::ExitStatusExt,
    process::{ExitStatus, Output},
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
enum File {
    OnDisk(String),
    InMemory(Vec<u8>),
}

#[derive(Clone, Debug)]
pub(crate) struct Filesystem(DashMap<String, File>);
impl Filesystem {
    fn new() -> Self {
        Self(DashMap::new())
    }

    fn from_files_on_disk(files: &[String]) -> Self {
        let map = files
            .iter()
            .map(|file| (file.clone(), File::OnDisk(file.to_string())))
            .collect();
        Self(map)
    }

    fn update_from_files_on_disk(&self, files: &[String]) {
        for file in files {
            self.0.insert(file.clone(), File::OnDisk(file.clone()));
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct JobContextInner {
    pub(crate) filesystem: Filesystem,
    pub(crate) output: Output,
}

#[derive(Clone, Debug)]
pub(crate) struct JobContext {
    inner: Arc<Mutex<JobContextInner>>,
}

impl JobContext {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(JobContextInner {
                filesystem: Filesystem::new(),
                output: Output {
                    status: ExitStatus::from_raw(0),
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                },
            })),
        }
    }

    pub fn update_from_output(&self, output: Output) -> Result<(), ApplicationError> {
        let mut inner = self.inner.lock()?;
        inner.output = output;
        Ok(())
    }

    pub fn update_from_files_on_disk(&self, files: &[String]) -> Result<(), ApplicationError> {
        let inner = self.inner.lock()?;
        inner.filesystem.update_from_files_on_disk(files);
        Ok(())
    }

    pub fn output(&self) -> Result<Output, ApplicationError> {
        Ok(self.inner.lock()?.output.clone())
    }
}

#[async_trait]
pub trait Operation: Send + Sync {
    fn execute(&self) -> Result<Output, ApplicationError>;
    fn description(&self) -> String;
    fn shortname(&self) -> &str;
    fn jobcontext(&self) -> &JobContext;
    fn outputs(&self) -> Vec<Arc<str>>;

    fn run_shell_command(&self, cmd: &str, outputs: &[String]) -> Result<Output, ApplicationError> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        self.jobcontext().update_from_output(output)?;
        self.jobcontext().update_from_files_on_disk(outputs)?;
        self.jobcontext().output().clone()
    }
}

impl std::fmt::Debug for dyn Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.shortname())
    }
}
