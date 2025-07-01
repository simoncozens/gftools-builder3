use std::{os::unix::process::ExitStatusExt, path::PathBuf, process::ExitStatus};

use crate::{
    error::ApplicationError,
    operations::{Operation, OperationOutput, Output},
};
use fontc::generate_font;
use fontc::Flags;
use tempfile::tempdir;

pub(crate) struct Fontc;

impl Operation for Fontc {
    fn shortname(&self) -> &str {
        "Fontc"
    }
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let input_file = inputs
            .first()
            .ok_or_else(|| ApplicationError::WrongInputs("No input file provided".to_string()))?
            .to_filename()?;
        let input = fontc::Input::new(&PathBuf::from(input_file))
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        let font = generate_font(&input, tempdir()?.path(), None, Flags::default(), false)
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        outputs[0].set_contents(font)?;
        Ok(Output {
            status: ExitStatus::from_raw(0),
            stdout: vec![],
            stderr: vec![],
        })
    }

    fn description(&self) -> String {
        "Build a variable font".to_string()
    }
}
