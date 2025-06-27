use std::process::Output;

use crate::{
    error::ApplicationError,
    operations::{JobContext, Operation},
};

pub(crate) struct Glyphs2UFO {
    _jobcontext: JobContext,
    pub source: String,
    pub outputs: Vec<String>,
}

impl Glyphs2UFO {
    pub fn new(source: String, outputs: Vec<String>) -> Self {
        Self {
            _jobcontext: JobContext::new(),
            source,
            outputs,
        }
    }
}

impl Operation for Glyphs2UFO {
    fn shortname(&self) -> &str {
        "Glyphs2UFO"
    }
    fn jobcontext(&self) -> &JobContext {
        &self._jobcontext
    }

    fn execute(&self) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ufo --instance-dir instance_ufo -g {}",
            self.source
        );
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        self._jobcontext.update_from_output(output)?;
        self._jobcontext.update_from_files_on_disk(&self.outputs)?;
        self._jobcontext.output().clone()
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
}
