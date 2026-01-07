use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{
    error::ApplicationError,
    recipe::{Config, Provider, Recipe},
};

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
    fn generate_recipe(mut self) -> Result<Recipe, ApplicationError> {
        // Implementation for rewriting the recipe for Noto fonts
        Ok(Recipe::default())
    }
}
