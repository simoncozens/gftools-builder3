use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::Hash,
    sync::Arc,
};

use crate::operations::Operation;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct BuildId(u64);

impl Display for BuildId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl BuildId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Configuration {
    jobs: HashMap<Arc<BuildId>, Arc<Box<dyn Operation>>>,
    final_targets: HashSet<Arc<BuildId>>,
    build_directory: Option<Arc<str>>,
}

impl Configuration {
    pub fn new(final_targets: HashSet<Arc<BuildId>>, build_directory: Option<Arc<str>>) -> Self {
        Self {
            jobs: Default::default(),
            final_targets,
            build_directory,
        }
    }

    pub fn jobs(&self) -> &HashMap<Arc<BuildId>, Arc<Box<dyn Operation>>> {
        &self.jobs
    }

    pub fn final_targets(&self) -> &HashSet<Arc<BuildId>> {
        &self.final_targets
    }

    pub fn add_job(&mut self, build: Box<dyn Operation>) {
        self.jobs.insert(build.id().into(), Arc::new(build));
    }
}
