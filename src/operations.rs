pub mod buildstatic;
pub mod buildvariable;
pub mod glyphs2ufo;

use std::{
    hash::{Hash, Hasher},
    process::Output,
    sync::Arc,
};

use async_trait::async_trait;

use crate::{error::ApplicationError, ir::BuildId};

#[async_trait]
pub trait Operation: Send + Sync {
    fn execute(&self) -> Result<Output, ApplicationError>;
    fn description(&self) -> String;
    fn outputs(&self) -> Vec<Arc<str>>;
    fn id(&self) -> BuildId {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.outputs().hash(&mut hasher);
        BuildId::new(hasher.finish())
    }
    fn dependencies(&self) -> &[Arc<BuildId>];
}

impl PartialEq for dyn Operation {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}
impl Eq for dyn Operation {}
