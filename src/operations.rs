pub mod buildstat;
pub mod fix;
pub mod fontc;
pub mod glyphs2ufo;

use crate::error::ApplicationError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    os::unix::process::ExitStatusExt,
    process::{ExitStatus, Output},
    sync::{Arc, Mutex, MutexGuard},
};
use tempfile::NamedTempFile;

/// An output from an operation
///
/// gftools-builder is essentially a build system like `make` or `ninja`. In these kinds of build systems,
/// the fundamental unit of operation is a process, and processes communicate their outputs via file names.
/// In some cases, the file names aren't important; they are just a way to pass data around between processes.
/// So you could imagine a hypothetical extension to Makefile which does something like:
///
/// ```makefile
/// <temporary file 1>: Foo.glyphs
///     fontc $< -o $@
/// Foo.ttf: <temporary file 1>
///     gftools fix $< -o $@
/// ```
///
/// We're saying "we don't care what the file is called; when you need a name for the purposes of the process,
/// come up with a temporary file name and use it". Now let's go even further. In gftools-builder, the fundamental
/// unit of operation is a Rust thread running a function. If we're just passing data from thread thread, we don't
/// even need our data to hit the disk at all. We can just pass around `Vec<u8>`s in memory between our operations.
/// (The data is stored in the operations graph as edges between the operations.)
/// Some operations, however, do call external processes and need to write and read files, so we need to be able to
/// convert between `Vec<u8>`s and file names.
#[derive(Debug)]
pub enum RawOperationOutput {
    NamedFile(String),
    TemporaryFile(Option<NamedTempFile>),
    InMemoryBytes(Vec<u8>),
}

impl PartialEq for RawOperationOutput {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RawOperationOutput::NamedFile(a), RawOperationOutput::NamedFile(b)) => a == b,
            (RawOperationOutput::TemporaryFile(a), RawOperationOutput::TemporaryFile(b)) => {
                a.as_ref().map(|f| f.path()) == b.as_ref().map(|f| f.path())
            }
            (RawOperationOutput::InMemoryBytes(a), RawOperationOutput::InMemoryBytes(b)) => a == b,
            _ => false,
        }
    }
}
#[derive(Clone)]
pub struct OperationOutput(Arc<Mutex<RawOperationOutput>>);

impl RawOperationOutput {
    pub fn from_str(s: &str) -> Self {
        Self::NamedFile(s.to_string())
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

impl PartialEq for OperationOutput {
    fn eq(&self, other: &Self) -> bool {
        // unwraps here are horrible but this is only used during graph creation
        let self_lock = self.lock().unwrap();
        let other_lock = other.lock().unwrap();
        *self_lock == *other_lock
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
            RawOperationOutput::NamedFile(name) => write!(f, "NamedFile({name})"),
            RawOperationOutput::TemporaryFile(None) => write!(f, "UnnamedTemporaryFile"),
            RawOperationOutput::TemporaryFile(Some(x)) => {
                write!(f, "NamedTemporaryFile({})", x.path().to_string_lossy())
            }
            RawOperationOutput::InMemoryBytes(_) => write!(f, "InMemoryBytes"),
        }
    }
}

impl OperationOutput {
    pub fn to_filename(&self) -> Result<String, ApplicationError> {
        let mut f = self.lock().map_err(|_| ApplicationError::MutexPoisoned)?;
        match &mut *f {
            RawOperationOutput::NamedFile(name) => Ok(name.to_string()),
            RawOperationOutput::TemporaryFile(x) => {
                // if it's none, make one and set it to some
                if let Some(temp_file) = x {
                    Ok(temp_file.path().to_string_lossy().to_string())
                } else {
                    let temp_file =
                        NamedTempFile::new().map_err(|e| ApplicationError::Other(e.to_string()))?;
                    *x = Some(temp_file);
                    Ok(x.as_ref().unwrap().path().to_string_lossy().to_string())
                }
            }
            RawOperationOutput::InMemoryBytes(bytes) => {
                // Convert in-memory bytes to a temp file by writing it
                let temp_file =
                    NamedTempFile::new().map_err(|e| ApplicationError::Other(e.to_string()))?;
                // write
                let temp_path = temp_file.path();
                let temp_path_string = temp_path.to_string_lossy().to_string();
                std::fs::write(temp_path, bytes)
                    .map_err(|e| ApplicationError::Other(e.to_string()))?;
                *f = RawOperationOutput::TemporaryFile(Some(temp_file));
                Ok(temp_path_string)
            }
        }
    }
    pub fn set_bytes(&self, bytes: Vec<u8>) -> Result<(), ApplicationError> {
        let mut f = self.lock().map_err(|_| ApplicationError::MutexPoisoned)?;
        *f = RawOperationOutput::InMemoryBytes(bytes);
        Ok(())
    }
    pub fn is_named_file(&self) -> bool {
        let f = self.lock().unwrap();
        matches!(&*f, RawOperationOutput::NamedFile(_))
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>, ApplicationError> {
        let f = self.lock().map_err(|_| ApplicationError::MutexPoisoned)?;
        match &*f {
            RawOperationOutput::NamedFile(name) => {
                // Read the file contents
                let bytes =
                    std::fs::read(name).map_err(|e| ApplicationError::Other(e.to_string()))?;
                Ok(bytes)
            }
            RawOperationOutput::TemporaryFile(Some(temp_file)) => {
                // Read the temp file contents
                let bytes = std::fs::read(temp_file.path())
                    .map_err(|e| ApplicationError::Other(e.to_string()))?;
                Ok(bytes)
            }
            RawOperationOutput::TemporaryFile(None) => Err(ApplicationError::Other(
                "Temporary file is not set".to_string(),
            )),
            RawOperationOutput::InMemoryBytes(bytes) => Ok(bytes.clone()),
        }
    }
    pub fn set_contents(&self, bytes: Vec<u8>) -> Result<(), ApplicationError> {
        if self.is_named_file() {
            // OK, we write it
            let output_path = self.to_filename()?;
            Ok(std::fs::write(output_path, bytes)?)
        } else {
            self.set_bytes(bytes)
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

    fn run_shell_command(
        &self,
        cmd: &str,
        _outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        Ok(output)
    }
}

impl std::fmt::Debug for dyn Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.shortname())
    }
}

impl std::fmt::Display for dyn Operation {
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
        _inputs: &[OperationOutput],
        _outputs: &[OperationOutput],
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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum OpStep {
    #[serde(rename = "glyphs2ufo")]
    Glyphs2UFO,
    #[serde(rename = "fontc")]
    Fontc,
    #[serde(rename = "fix")]
    Fix,
    #[serde(rename = "buildStat")]
    BuildStat,
}

impl OpStep {
    pub fn operation(&self) -> Box<dyn Operation> {
        match self {
            OpStep::Fix => Box::new(fix::Fix),
            OpStep::Fontc => Box::new(fontc::Fontc),
            OpStep::Glyphs2UFO => Box::new(glyphs2ufo::Glyphs2UFO),
            OpStep::BuildStat => Box::new(buildstat::BuildStat),
        }
    }
}
