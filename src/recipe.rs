use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};

use crate::{
    buildsystem::{BuildGraph, BuildStep},
    error::ApplicationError,
    operations::OpStep,
    recipe_providers::{
        googlefonts::{GoogleFontsOptions, GoogleFontsProvider},
        noto::{NotoFontsOptions, NotoProvider},
    },
};

/// Enum representing different recipe providers as stored in the config file
///
/// This is used during recipe deserialization to handle different provider-specific options.
/// We handle the case where the provider is explicitly tagged - either as "googlefonts" or "noto" -
/// as well as the untagged case where we assume "googlefonts" by default.

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

pub(crate) trait Provider {
    fn generate_recipe(self) -> Result<Recipe, ApplicationError>;
}

impl RecipeProvider {
    pub fn generate_recipe(&self) -> Result<Recipe, ApplicationError> {
        match self {
            RecipeProvider::TaggedGoogleFonts { options, .. } => {
                let provider = GoogleFontsProvider::new(options.clone());
                provider.generate_recipe()
            }
            RecipeProvider::UntaggedGoogleFonts { options } => {
                let provider = GoogleFontsProvider::new(options.clone());
                provider.generate_recipe()
            }
            RecipeProvider::Noto { options, .. } => {
                let provider = NotoProvider::new(options.clone());
                provider.generate_recipe()
            }
            RecipeProvider::Other {
                recipe_provider, ..
            } => Err(ApplicationError::InvalidRecipe(format!(
                "Unknown recipe provider: {recipe_provider}"
            ))),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(untagged)]
pub(crate) enum Step {
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
                args,
                extra,
                input_file,
            } => {
                let mut op = operation.operation();
                op.set_extra(extra.clone());
                op.set_args(args.clone());
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

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ConfigOperation(pub(crate) Vec<Step>);

pub type Recipe = HashMap<String, ConfigOperation>;

#[derive(Serialize, PartialEq, Debug)]
pub(crate) struct Config {
    #[serde(default)]
    recipe: Recipe,
    #[serde(flatten)]
    recipe_provider: Option<RecipeProvider>,
}

// We need to deserialize manually because: If there is a recipe but no recipe provider,
// the recipe provider is None. But if there is no recipe and no recipe provider, we want to
// default to googlefonts.
impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            #[serde(default)]
            recipe: Recipe,
            #[serde(flatten)]
            recipe_provider: Option<RecipeProvider>,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let nullify_provider = !helper.recipe.is_empty()
            && matches!(
                helper.recipe_provider,
                Some(RecipeProvider::UntaggedGoogleFonts { .. })
            );
        Ok(Config {
            recipe: helper.recipe,
            recipe_provider: if nullify_provider {
                None
            } else {
                helper.recipe_provider
            },
        })
    }
}

impl Config {
    pub(crate) fn recipe(&self) -> Result<Recipe, ApplicationError> {
        let mut recipe = if let Some(provider) = &self.recipe_provider {
            provider.generate_recipe()?
        } else {
            Recipe::new()
        };
        // If the user provided a recipe in the config, overlay it on top.
        recipe.extend(self.recipe.clone());
        Ok(recipe)
    }

    pub(crate) fn to_graph(&self) -> Result<BuildGraph, ApplicationError> {
        let mut graph = BuildGraph::new();
        let recipe = self.recipe()?;
        for (target, operation) in recipe {
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
            graph.add_path(source_filename, operations, &target);
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
                        sources: vec!["Nunito.glyphs".to_string()],
                        ..Default::default()
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
                        sources: vec!["Nunito.glyphs".to_string()],
                        ..Default::default()
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
