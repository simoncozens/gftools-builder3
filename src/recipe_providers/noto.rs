use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    error::ApplicationError,
    recipe::{Provider, Recipe},
};

#[allow(dead_code)] // We haven't implemented all functionality yet
pub struct NotoProvider(pub NotoFontsOptions);

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub(crate) struct NotoFontsOptions {
    sources: Vec<String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
    #[serde(default)]
    extra: HashMap<String, Value>,
}

impl NotoProvider {
    pub fn new(options: NotoFontsOptions) -> Self {
        NotoProvider(options)
    }
}

impl Provider for NotoProvider {
    fn generate_recipe(&self) -> Result<Recipe, ApplicationError> {
        // Implementation for rewriting the recipe for Noto fonts
        Ok(Recipe::default())
    }
}
