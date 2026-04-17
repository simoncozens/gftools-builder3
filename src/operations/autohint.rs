use std::{os::unix::process::ExitStatusExt, process::Output};

use crate::{
    buildsystem::{DataKind, Operation, OperationOutput},
    error::ApplicationError,
};
use tilvisan::{autohint, Args};

#[derive(PartialEq, Debug)]
pub(crate) struct Autohint;

impl Autohint {
    pub fn new() -> Self {
        Autohint
    }
}

impl Operation for Autohint {
    fn shortname(&self) -> &str {
        "Autohint"
    }

    fn input_kinds(&self) -> Vec<DataKind> {
        vec![DataKind::Bytes]
    }

    fn output_kinds(&self) -> Vec<DataKind> {
        vec![DataKind::Bytes]
    }

    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        assert!(inputs.len() == outputs.len());
        let font_filename = inputs[0].to_filename(Some(".ttf"))?;
        let mut args = Args::default();
        args.input = font_filename;
        let hinted_font = autohint(&args)
            .map_err(|e| ApplicationError::Other(format!("Autohinting failed: {}", e)))?;
        outputs[0].set_contents(hinted_font)?;
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: vec![],
            stderr: vec![],
        })
    }

    fn description(&self) -> String {
        "Applies autohinting to the font using tilvisan".to_string()
    }
}
