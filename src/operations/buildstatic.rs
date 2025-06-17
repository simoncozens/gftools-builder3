use std::{process::Output, sync::Arc};

use crate::{error::ApplicationError, operations::Operation, orchestrator::BuildId};

pub(crate) struct BuildStatic {
    pub source: String,
    pub output: String,
    pub dependencies: Vec<Arc<BuildId>>,
}

impl Operation for BuildStatic {
    fn execute(&self) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ttf -u {} --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path {}",
            self.source, self.output
        );
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))
    }

    fn description(&self) -> String {
        format!("Build a static font '{}'", self.source)
    }
    fn outputs(&self) -> Vec<std::sync::Arc<str>> {
        vec![std::sync::Arc::from(self.output.as_str())]
    }

    fn dependencies(&self) -> &[std::sync::Arc<BuildId>] {
        &self.dependencies
    }
}
