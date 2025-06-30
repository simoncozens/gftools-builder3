use std::process::Output;

use crate::{
    error::ApplicationError,
    operations::{Operation, OperationOutput},
};

pub(crate) struct Glyphs2UFO;

impl Operation for Glyphs2UFO {
    fn shortname(&self) -> &str {
        "Glyphs2UFO"
    }
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ufo -i --instance-dir instance_ufo -g {}",
            inputs[0].to_filename()?
        );
        self.run_shell_command(&cmd, outputs)
    }
    fn description(&self) -> String {
        "Convert glyphs file to UFO format".to_string()
    }
}
