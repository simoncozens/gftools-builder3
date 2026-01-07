use crate::buildsystem::OperationOutput;
use crate::error::ApplicationError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
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
    /// Return any machine-readable identifer for this operation and any parameters.
    ///
    /// This function should return an identifier with enough information to
    /// tell you whether this operation
    /// can be reused between targets. For example, we might want to use
    /// the "addSubset" operation to generate fonts B and C from font A.
    /// We can't just check the short name in that case because the parameters
    /// might be different.
    fn identifier(&self) -> String {
        self.shortname().to_string()
    }

    fn set_args(&mut self, _args: Option<String>) {
        // Default implementation does nothing.
    }
    fn set_extra(&mut self, _extra: HashMap<String, Value>) {
        // Default implementation does nothing.
    }
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
impl std::cmp::PartialEq for dyn Operation {
    fn eq(&self, other: &Self) -> bool {
        self.identifier() == other.identifier()
    }
}
