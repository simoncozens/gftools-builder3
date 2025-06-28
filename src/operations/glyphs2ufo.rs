use std::process::Output;

use crate::{error::ApplicationError, operations::Operation};

pub(crate) struct Glyphs2UFO {}

impl Glyphs2UFO {
    pub fn new() -> Self {
        Self {}
    }
}

impl Operation for Glyphs2UFO {
    fn shortname(&self) -> &str {
        "Glyphs2UFO"
    }
    // fn jobcontext(&self) -> &JobContext {
    //     &self._jobcontext
    // }

    fn execute(&self, inputs: &[String], outputs: &[String]) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ufo -i --instance-dir instance_ufo -g {}",
            inputs[0]
        );
        self.run_shell_command(&cmd, outputs)
    }
    fn description(&self) -> String {
        "Convert glyphs file to UFO format".to_string()
    }
}
