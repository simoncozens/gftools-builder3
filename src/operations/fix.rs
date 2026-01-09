use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::info_span;

use crate::{
    buildsystem::{DataKind, Operation, OperationOutput},
    error::ApplicationError,
};
use std::process::Output;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FixConfig {
    #[serde(default)]
    pub include_source_fixes: bool,

    #[serde(default)]
    pub fvar_instance_axis_dflts: HashMap<String, f32>,
}

#[derive(PartialEq, Debug)]
pub(crate) struct Fix {
    args: Option<String>,
    config: FixConfig,
}

impl Fix {
    pub fn new() -> Self {
        Fix {
            args: None,
            config: FixConfig::default(),
        }
    }
}

impl Operation for Fix {
    fn shortname(&self) -> &str {
        "Fix"
    }

    fn input_kinds(&self) -> Vec<DataKind> {
        vec![DataKind::Path]
    }

    fn output_kinds(&self) -> Vec<DataKind> {
        vec![DataKind::Path]
    }

    fn execute(
        &self,
        inputs: &[OperationOutput],
        outputs: &[OperationOutput],
    ) -> Result<Output, ApplicationError> {
        let _span = info_span!("gftools-fix-font").entered();

        // Build command with base arguments
        let mut cmd_parts = vec![
            "gftools-fix-font".to_string(),
            inputs[0].to_filename()?,
            "-o".to_string(),
            outputs[0].to_filename()?,
        ];

        if self.config.include_source_fixes {
            cmd_parts.push("--include-source-fixes".to_string());
        }
        for (axis, value) in &self.config.fvar_instance_axis_dflts {
            cmd_parts.push(format!("--fvar-instance-axis-dflt={}:{}", axis, value));
        }

        // Add any additional args
        if let Some(ref args) = self.args {
            cmd_parts.push(args.clone());
        }

        let cmd = cmd_parts.join(" ");
        self.run_shell_command(&cmd, outputs)
    }

    fn description(&self) -> String {
        "Apply gftools-fix-font".to_string()
    }

    fn set_args(&mut self, args: Option<String>) {
        self.args = args;
    }

    fn set_extra(&mut self, extra: HashMap<String, Value>) {
        // Deserialize the extra map into our typed config
        let value = Value::Object(extra.into_iter().collect());
        self.config = serde_json::from_value(value).unwrap_or_else(|e| {
            log::warn!("Failed to deserialize Fix config: {}. Using defaults.", e);
            FixConfig::default()
        });
    }

    fn identifier(&self) -> String {
        format!(
            "Fix-{}-{:?}",
            self.args.as_deref().unwrap_or(""),
            self.config
        )
    }
}
