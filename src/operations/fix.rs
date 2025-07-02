use crate::{
    error::ApplicationError,
    operations::{Operation, OperationOutput, Output},
};

#[derive(PartialEq, Debug)]
pub(crate) struct Fix;

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
        self.run_shell_command(&cmd, outputs)
    }

    fn description(&self) -> String {
        "Build a static font".to_string()
    }
}
