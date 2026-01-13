use std::collections::HashMap;

use crate::{
    buildsystem::Operation,
    operations::{fix::FixConfig, fontc::FontcConfig},
    recipe::{ConfigOperation, Step},
};
use serde::{Deserialize, Serialize};

pub mod addsubset;
pub mod buildstat;
pub mod compress;
pub mod convert;
pub mod fix;
pub mod fontc;
pub mod glyphs2ufo;

/// Enum representing the different operation steps available
///
/// This is used during recipe deserialization to map step names to operation implementations.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub(crate) enum OpStep {
    #[serde(rename = "glyphs2ufo")]
    Glyphs2UFO,
    #[serde(rename = "fontc")]
    Fontc,
    #[serde(rename = "fix")]
    Fix,
    #[serde(rename = "buildStat")]
    BuildStat,
    #[serde(rename = "compress")]
    Compress,
    #[serde(rename = "addSubset")]
    AddSubset,
}

impl OpStep {
    /// Convert the OpStep enum variant to its corresponding Operation implementation
    pub fn operation(&self) -> Box<dyn Operation> {
        match self {
            OpStep::Fix => Box::new(fix::Fix::new()),
            OpStep::Fontc => Box::new(fontc::Fontc::new()),
            OpStep::Glyphs2UFO => Box::new(glyphs2ufo::Glyphs2UFO),
            OpStep::BuildStat => Box::new(buildstat::BuildStat),
            OpStep::Compress => Box::new(compress::Compress),
            OpStep::AddSubset => Box::new(addsubset::AddSubset::new()),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct ConfigOperationBuilder {
    steps: Vec<Step>,
}
impl ConfigOperationBuilder {
    pub fn new() -> Self {
        ConfigOperationBuilder { steps: vec![] }
    }

    pub fn build(self) -> ConfigOperation {
        ConfigOperation(self.steps)
    }

    pub fn source(mut self, s: String) -> Self {
        self.steps.push(Step::SourceStep {
            source: s,
            extra: HashMap::new(),
        });
        self
    }

    pub fn fix(mut self, config: &FixConfig) -> Self {
        // Serialize FixConfig to a HashMap<String, serde_json::Value>
        let extra = serde_json::to_value(config)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<String, serde_json::Value>>();
        self.steps.push(Step::OperationStep {
            operation: OpStep::Fix,
            extra,
            args: None,
            input_file: None,
            needs: vec![],
        });
        self
    }

    pub fn compile(mut self, config: &FontcConfig) -> Self {
        let extra = serde_json::to_value(config)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<HashMap<String, serde_json::Value>>();
        self.steps.push(Step::OperationStep {
            operation: OpStep::Fontc,
            extra,
            args: None,
            input_file: None,
            needs: vec![],
        });
        self
    }

    pub fn compress(mut self) -> Self {
        self.steps.push(Step::OperationStep {
            operation: OpStep::Compress,
            extra: HashMap::new(),
            args: None,
            input_file: None,
            needs: vec![],
        });
        self
    }

    pub fn buildstat(mut self, others: &[String]) -> Self {
        self.steps.push(Step::OperationStep {
            operation: OpStep::BuildStat,
            extra: HashMap::new(),
            args: None,
            input_file: None,
            needs: others.to_vec(),
        });
        self
    }
}
