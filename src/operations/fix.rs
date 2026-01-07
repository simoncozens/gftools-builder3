use crate::{
    buildsystem::{Operation, OperationOutput},
    error::ApplicationError,
};
use std::{os::unix::process::ExitStatusExt as _, process::Output};

#[derive(PartialEq, Debug)]
pub(crate) struct Fix {
    args: Option<String>,
}
impl Fix {
    pub fn new() -> Self {
        Fix { args: None }
    }
}

impl Operation for Fix {
    fn shortname(&self) -> &str {
        "Fix"
    }
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "gftools-fix-font {} -o {}",
            inputs[0].to_filename()?,
            outputs[0].to_filename()?
        );
        self.run_shell_command(&cmd, outputs)?;
        // All outputs now need to have the same contents as the first output
        for output in &outputs[1..] {
            let contents = inputs[0].to_bytes()?;
            output.set_contents(contents)?;
        }
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: vec![],
            stderr: vec![],
        })
    }

    fn description(&self) -> String {
        "Apply gftools-fix-font".to_string()
    }

    fn set_args(&mut self, args: Option<String>) {
        self.args = args;
    }

    fn identifier(&self) -> String {
        format!("Fix-{}", self.args.as_deref().unwrap_or(""))
    }
}
