use crate::{
    error::ApplicationError,
    operations::{JobContext, Operation, Output},
};

pub(crate) struct BuildStatic {
    _jobcontext: JobContext,
}

impl BuildStatic {
    pub fn new() -> Self {
        Self {
            _jobcontext: JobContext::new(),
        }
    }
}

impl Operation for BuildStatic {
    fn shortname(&self) -> &str {
        "BuildStatic"
    }
    // fn jobcontext(&self) -> &JobContext {
    //     &self._jobcontext
    // }
    fn execute(&self, inputs: &[String], outputs: &[String]) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ttf -u {} --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path {}",
            inputs[0], outputs[0]
        );
        self.run_shell_command(&cmd, outputs)
    }

    fn description(&self) -> String {
        "Build a static font".to_string()
    }
}
