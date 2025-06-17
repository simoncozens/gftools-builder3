use std::{process::Output, sync::Arc};

use crate::{error::ApplicationError, ir::BuildId, operations::Operation};

pub(crate) struct BuildVariable {
    pub source: String,
    pub output: String,
    pub dependencies: Vec<Arc<BuildId>>,
}

impl Operation for BuildVariable {
    fn execute(&self) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o variable -m {} --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path {}",
            self.source, self.output
        );
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))
    }

    fn description(&self) -> String {
        format!("Build a variable font '{}'", self.source)
    }
    fn outputs(&self) -> Vec<std::sync::Arc<str>> {
        vec![std::sync::Arc::from(self.output.as_str())]
    }

    fn dependencies(&self) -> &[std::sync::Arc<crate::ir::BuildId>] {
        &self.dependencies
    }
}
