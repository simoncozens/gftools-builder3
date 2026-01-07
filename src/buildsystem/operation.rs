use crate::buildsystem::OperationOutput;
use crate::error::ApplicationError;
use async_trait::async_trait;
use std::process::Output;

/// Trait representing a build operation
///
/// An operation is a node in the build graph that takes some inputs and produces some outputs.
/// The implementation of the Operation trait determines the behavior of the operation.
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
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        log::debug!("Running shell command: {}", cmd);
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        log::debug!("Outputs: {:?}", outputs);
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
