use std::{
    collections::{HashMap, HashSet, hash_map::DefaultHasher},
    fmt::Display,
    hash::{Hash, Hasher},
    sync::Arc,
};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Build {
    // IDs are persistent across different builds so that they can be used for,
    // for example, caching.
    id: BuildId,
    outputs: Vec<Arc<str>>,
    rule: Option<Rule>,
    dependencies: Vec<Arc<BuildId>>,
}

impl Build {
    pub fn new(
        outputs: Vec<Arc<str>>,
        rule: Option<Rule>,
        dependencies: Vec<Arc<BuildId>>,
    ) -> Self {
        Self {
            id: Self::calculate_id(&outputs),
            outputs,
            rule,
            dependencies,
        }
    }

    pub fn id(&self) -> BuildId {
        self.id
    }

    pub fn rule(&self) -> Option<&Rule> {
        self.rule.as_ref()
    }

    pub fn dependencies(&self) -> &[Arc<BuildId>] {
        &self.dependencies
    }

    fn calculate_id(outputs: &[Arc<str>]) -> BuildId {
        let mut hasher = DefaultHasher::new();

        outputs.hash(&mut hasher);

        BuildId::new(hasher.finish())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configuration {
    jobs: HashMap<Arc<BuildId>, Arc<Build>>,
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

    pub fn jobs(&self) -> &HashMap<Arc<BuildId>, Arc<Build>> {
        &self.jobs
    }

    pub fn final_targets(&self) -> &HashSet<Arc<BuildId>> {
        &self.final_targets
    }

    pub fn add_job(&mut self, build: Arc<Build>) {
        self.jobs.insert(build.id().into(), build);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    command: String,
    description: Option<String>,
}

impl Rule {
    pub fn new(command: impl Into<String>, description: Option<String>) -> Self {
        Self {
            command: command.into(),
            description,
        }
    }

    pub fn command(&self) -> &str {
        &self.command
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}
