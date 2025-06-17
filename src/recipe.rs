use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
struct Step {
    operation: String,
    args: String,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct ConfigOperation(Vec<Step>);

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Config {
    #[serde(default)]
    recipe: HashMap<String, ConfigOperation>,
    #[serde(flatten)]
    recipe_provider: Option<RecipeProvider>,
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
        let config = r#"
recipe:
    Nunito.designspace:
        - operation: "glyphs2ufo"
          args: "Nunito.glyphs"
"#;
        let deserialized_map: Config =
            serde_yaml_ng::from_str(config).expect("Failed to deserialize YAML");
        let recipe = HashMap::from([(
            "Nunito.designspace".to_string(),
            ConfigOperation(vec![Step {
                operation: "glyphs2ufo".to_string(),
                args: "Nunito.glyphs".to_string(),
                extra: HashMap::new(),
            }]),
        )]);
        assert_eq!(
            deserialized_map,
            Config {
                recipe,
                recipe_provider: None
            }
        );
    }
}
