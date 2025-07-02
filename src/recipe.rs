use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use crate::{
    error::ApplicationError,
    graph::{BuildGraph, BuildStep},
    operations::{OpStep, Operation},
};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct GoogleFontsOptions {
    sources: Vec<String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
    #[serde(default)]
    extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct NotoFontsOptions {
    sources: Vec<String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
    #[serde(default)]
    extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(untagged)]
enum RecipeProvider {
    TaggedGoogleFonts {
        #[serde(rename = "recipeProvider")]
        _recipe_provider: monostate::MustBe!("googlefonts"),
        #[serde(flatten)]
        options: GoogleFontsOptions,
    },
    Noto {
        #[serde(rename = "recipeProvider")]
        _recipe_provider: monostate::MustBe!("noto"),
        #[serde(flatten)]
        options: NotoFontsOptions,
    },
    Other {
        #[serde(rename = "recipeProvider")]
        recipe_provider: String,
        options: HashMap<String, Value>,
    },
    UntaggedGoogleFonts {
        #[serde(flatten)]
        options: GoogleFontsOptions,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(untagged)]
enum Step {
    OperationStep {
        operation: OpStep,
        #[serde(default)]
        args: Option<String>,
        #[serde(default)]
        input_file: Option<String>,
        #[serde(flatten)]
        extra: HashMap<String, Value>,
    },
    SourceStep {
        source: String,
        #[serde(flatten)]
        extra: HashMap<String, Value>,
    },
}

impl Step {
    fn to_operation(&self) -> Result<(Option<String>, BuildStep), ApplicationError> {
        match self {
            Step::OperationStep {
                operation,
                args: _,
                extra: _,
                input_file,
            } => {
                let op = operation.operation();
                // Here you can handle args and extra if needed
                Ok((input_file.clone(), Arc::new(op)))
            }
            Step::SourceStep { source, extra: _ } => {
                // Handle source step, possibly creating a Source operation
                Err(ApplicationError::InvalidRecipe(format!(
                    "Source step not implemented: {source}"
                )))
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ConfigOperation(Vec<Step>);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub(crate) struct Config {
    #[serde(default)]
    recipe: HashMap<String, ConfigOperation>,
    #[serde(flatten)]
    recipe_provider: Option<RecipeProvider>,
}

impl Config {
    pub(crate) fn to_graph(&mut self) -> Result<BuildGraph, ApplicationError> {
        let mut graph = BuildGraph::new();
        if let Some(_provider) = &self.recipe_provider {
            // provider.rewrite_recipe(&mut self)?;
        }
        for (target, operation) in &self.recipe {
            // First operation must be a source step
            let source = operation.0.first().ok_or_else(|| {
                ApplicationError::InvalidRecipe(format!("No steps found for target '{target}'"))
            })?;
            let source_filename = if let Step::SourceStep { source, .. } = source {
                Ok(source)
            } else {
                Err(ApplicationError::InvalidRecipe(format!(
                    "First step for target '{target}' must be a source step"
                )))
            }?;
            let operations: Vec<(Option<String>, BuildStep)> = operation
                .0
                .iter()
                .skip(1)
                .map(|step| step.to_operation())
                .collect::<Result<Vec<_>, ApplicationError>>()?;
            graph.add_path(source_filename, operations, target);
        }
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let config = r#"
sources:
    - "Nunito.glyphs"
"#;
        let deserialized_map: Config =
            serde_yaml_ng::from_str(config).expect("Failed to deserialize YAML");
        assert_eq!(
            deserialized_map,
            Config {
                recipe: HashMap::new(),
                recipe_provider: Some(RecipeProvider::UntaggedGoogleFonts {
                    options: GoogleFontsOptions {
                        outputs: HashMap::new(),
                        extra: HashMap::new(),
                        sources: vec!["Nunito.glyphs".to_string()]
                    }
                })
            }
        );
    }

    #[test]
    fn test_deserialize_explicit_provider() {
        let config = r#"
recipeProvider: googlefonts
sources:
    - "Nunito.glyphs"
"#;
        let deserialized_map: Config =
            serde_yaml_ng::from_str(config).expect("Failed to deserialize YAML");
        assert_eq!(
            deserialized_map,
            Config {
                recipe: HashMap::new(),
                recipe_provider: Some(RecipeProvider::TaggedGoogleFonts {
                    _recipe_provider: monostate::MustBe!("googlefonts"),
                    options: GoogleFontsOptions {
                        outputs: HashMap::new(),
                        extra: HashMap::new(),
                        sources: vec!["Nunito.glyphs".to_string()]
                    }
                })
            }
        );
    }

    #[test]
    fn test_deserialize_explicit_recipe() {
        let recipe = HashMap::from([(
            "Nunito.designspace".to_string(),
            ConfigOperation(vec![
                Step::SourceStep {
                    source: "Nunito.glyphs".to_string(),
                    extra: HashMap::new(),
                },
                Step::OperationStep {
                    operation: OpStep::Glyphs2UFO,
                    extra: HashMap::new(),
                    args: None,
                    input_file: None,
                },
            ]),
        )]);

        let config = r#"
recipe:
    Nunito.designspace:
        - source: "Nunito.glyphs"
        - operation: "glyphs2ufo"
"#;
        let deserialized_map: Config =
            serde_yaml_ng::from_str(config).expect("Failed to deserialize YAML");

        assert_eq!(
            deserialized_map,
            Config {
                recipe,
                recipe_provider: None
            }
        );
    }
}
