use tracing::info_span;

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
        let span = info_span!("gftools-fix-font").entered();
        let cmd = format!(
            "gftools-fix-font {} -o {} {}",
            inputs[0].to_filename()?,
            outputs[0].to_filename()?,
            self.args.as_deref().unwrap_or("")
        );
        self.run_shell_command(&cmd, outputs)
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
