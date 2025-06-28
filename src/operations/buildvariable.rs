use crate::{
    error::ApplicationError,
    operations::{JobContext, Operation, Output},
};

pub(crate) struct BuildVariable {
    _jobcontext: JobContext,
}

impl BuildVariable {
    pub fn new() -> Self {
        Self {
            _jobcontext: JobContext::new(),
        }
    }
}
impl Operation for BuildVariable {
    fn shortname(&self) -> &str {
        "BuildVariable"
    }
    // fn jobcontext(&self) -> &JobContext {
    //     &self._jobcontext
    // }
    fn execute(&self, inputs: &[String], outputs: &[String]) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o variable -g {} --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path {}",
            inputs[0], outputs[0]
        );
        self.run_shell_command(&cmd, outputs)
    }

    fn description(&self) -> String {
        "Build a variable font".to_string()
    }
}
