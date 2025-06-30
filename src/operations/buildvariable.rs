use crate::{
    error::ApplicationError,
    operations::{Operation, OperationOutput, Output},
};

pub(crate) struct BuildVariable;

impl Operation for BuildVariable {
    fn shortname(&self) -> &str {
        "BuildVariable"
    }
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o variable -g {} --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path {}",
            inputs[0].to_filename()?, outputs[0].to_filename()?
        );
        self.run_shell_command(&cmd, outputs)
    }

    fn description(&self) -> String {
        "Build a variable font".to_string()
    }
}
