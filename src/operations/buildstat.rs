use std::os::unix::process::ExitStatusExt;

use crate::{
    error::ApplicationError,
    operations::{Operation, OperationOutput, Output},
};
use fontations::read::FontRef;
use google_fonts_axisregistry::build_stat;

#[derive(PartialEq, Debug)]
pub(crate) struct BuildStat;

impl Operation for BuildStat {
    fn shortname(&self) -> &str {
        "BuildStat"
    }
    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        assert!(inputs.len() == outputs.len());
        let all_siblings_bytes = inputs
            .iter()
            .map(|input| input.to_bytes())
            .collect::<Result<Vec<_>, _>>()?;
        for index in 0..inputs.len() {
            let font = FontRef::new(&all_siblings_bytes[index])?;
            let others: Vec<FontRef> = all_siblings_bytes
                .iter()
                .enumerate()
                .filter_map(|(i, sibling)| {
                    if i != index {
                        FontRef::new(sibling).ok()
                    } else {
                        None
                    }
                })
                .collect();
            let with_stat =
                build_stat(font, &others).map_err(|e| ApplicationError::Other(e.to_string()))?;
            outputs[index].set_contents(with_stat)?;
        }
        Ok(Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: vec![],
            stderr: vec![],
        })
    }

    fn description(&self) -> String {
        "Add STAT tables".to_string()
    }
}
