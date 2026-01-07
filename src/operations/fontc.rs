use std::process::Output;
use std::{os::unix::process::ExitStatusExt, path::PathBuf, process::ExitStatus};

use crate::{
    buildsystem::{Operation, OperationOutput},
    error::ApplicationError,
};
use fontc::generate_font;

#[derive(PartialEq, Debug)]
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
            .map_err(|e| ApplicationError::Other(e.to_string()))?
            .create_source()
            .map_err(|e| ApplicationError::Other(e.to_string()))?;
        let font = generate_font(input, fontc::Options::default())
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
