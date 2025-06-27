use crate::{
    error::ApplicationError,
    operations::{JobContext, Operation, Output},
};

pub(crate) struct BuildStatic {
    pub source: String,
    pub output: String,
    _jobcontext: JobContext,
}

impl BuildStatic {
    pub fn new(source: String, output: String) -> Self {
        Self {
            _jobcontext: JobContext::new(),
            source,
            output,
        }
    }
}

impl Operation for BuildStatic {
    fn shortname(&self) -> &str {
        "BuildStatic"
    }
    fn jobcontext(&self) -> &JobContext {
        &self._jobcontext
    }
    fn execute(&self) -> Result<Output, ApplicationError> {
        let cmd = format!(
            "fontmake -o ttf -u {} --filter ... --filter FlattenComponentsFilter --filter DecomposeTransformedComponentsFilter --output-path {}",
            self.source, self.output
        );
        self.run_shell_command(&cmd, std::slice::from_ref(&self.output))
    }

    fn description(&self) -> String {
        format!("Build a static font '{}'", self.source)
    }
    fn outputs(&self) -> Vec<std::sync::Arc<str>> {
        vec![std::sync::Arc::from(self.output.as_str())]
    }
}
