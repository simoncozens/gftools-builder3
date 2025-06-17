use std::{process::Output, sync::Arc};

use crate::{error::ApplicationError, operations::Operation, orchestrator::BuildId};

pub(crate) struct Glyphs2UFO {
    pub source: String,
    pub outputs: Vec<String>,
    pub dependencies: Vec<Arc<BuildId>>,
}

impl Operation for Glyphs2UFO {
    fn execute(&self) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ufo --instance-dir instance_ufo -g {}",
            self.source
        );
        std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))
    }
    fn description(&self) -> String {
        format!("Convert glyphs file '{}' to UFO format", self.source)
    }

    fn outputs(&self) -> Vec<std::sync::Arc<str>> {
        self.outputs
            .iter()
            .map(|s| std::sync::Arc::from(s.as_str()))
            .collect()
    }

    fn dependencies(&self) -> &[std::sync::Arc<BuildId>] {
        &self.dependencies
    }
}
