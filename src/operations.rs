pub mod buildstatic;
pub mod buildvariable;
pub mod glyphs2ufo;

use crate::error::ApplicationError;
use async_trait::async_trait;
use std::{
    os::unix::process::ExitStatusExt,
    process::{ExitStatus, Output},
    sync::{Arc, Mutex, MutexGuard},
};
use tempfile::NamedTempFile;

#[derive(Debug)]
pub enum RawOperationOutput {
    NamedFile(String),
    TemporaryFile(Option<NamedTempFile>),
    InMemoryBytes(Vec<u8>),
}
#[derive(Clone)]
pub struct OperationOutput(Arc<Mutex<RawOperationOutput>>);

impl RawOperationOutput {
    pub fn from_str(s: &str) -> Self {
        Self::NamedFile(s.to_string())
    }

    pub fn to_filename(&self) -> String {
        match self {
            RawOperationOutput::NamedFile(name) => name.clone(),
            RawOperationOutput::TemporaryFile(x) => {
                panic!("Cannot convert TemporaryFile to filename")
            }
            RawOperationOutput::InMemoryBytes(_) => {
                panic!("Cannot convert InMemoryBytes to filename")
            }
        }
    }
}

impl From<&str> for RawOperationOutput {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<RawOperationOutput> for OperationOutput {
    fn from(output: RawOperationOutput) -> Self {
        Self(Arc::new(Mutex::new(output)))
    }
}
impl OperationOutput {
    pub fn lock(&self) -> Result<MutexGuard<RawOperationOutput>, ApplicationError> {
        self.0.lock().map_err(|_| ApplicationError::MutexPoisoned)
    }
}

impl std::fmt::Display for OperationOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw_output = self.lock().map_err(|_| std::fmt::Error)?;
        match &*raw_output {
            RawOperationOutput::NamedFile(name) => write!(f, "{name}"),
            RawOperationOutput::TemporaryFile(_) => write!(f, "<temporary file>"),
            RawOperationOutput::InMemoryBytes(_) => write!(f, "<in-memory bytes>"),
        }
    }
}

impl std::fmt::Debug for OperationOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let raw_output = self.lock().map_err(|_| std::fmt::Error)?;
        match &*raw_output {
            RawOperationOutput::NamedFile(name) => write!(f, "NamedFile({})", name),
            RawOperationOutput::TemporaryFile(_) => write!(f, "TemporaryFile"),
            RawOperationOutput::InMemoryBytes(_) => write!(f, "InMemoryBytes"),
        }
    }
}

#[async_trait]
pub trait Operation: Send + Sync {
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError>;
    fn description(&self) -> String;
    fn shortname(&self) -> &str;
    // fn jobcontext(&self) -> &JobContext;

    fn run_shell_command(
        &self,
        cmd: &str,
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        Ok(output)
        // self.jobcontext().update_from_output(output)?;
        // self.jobcontext().update_from_files_on_disk(outputs)?;
        // self.jobcontext().output().clone()
    }
}

impl std::fmt::Debug for dyn Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.shortname())
    }
}

pub(crate) enum SourceSink {
    Source,
    Sink,
}
impl Operation for SourceSink {
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        Ok(Output {
            status: ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }

    fn description(&self) -> String {
        match self {
            SourceSink::Source => "Source Operation".to_string(),
            SourceSink::Sink => "Sink Operation".to_string(),
        }
    }

    fn shortname(&self) -> &str {
        match self {
            SourceSink::Source => "Source",
            SourceSink::Sink => "Sink",
        }
    }
}
