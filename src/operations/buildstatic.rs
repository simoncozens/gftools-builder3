use crate::{
    error::ApplicationError,
    operations::{Operation, OperationOutput, Output},
};

pub(crate) struct BuildStatic;

impl Operation for BuildStatic {
    fn shortname(&self) -> &str {
        "BuildStatic"
    }
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ttf -u {} --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path {}",
            inputs[0].to_filename()?, outputs[0].to_filename()?
        );
        self.run_shell_command(&cmd, outputs)
    }

    fn description(&self) -> String {
        "Build a static font".to_string()
    }
}
