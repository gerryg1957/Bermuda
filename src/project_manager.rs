use std::path::Path;

use crate::project::Project;

#[derive(Debug, Default)]
pub struct ProjectManager;

impl ProjectManager {
    pub fn new() -> Self {
        Self
    }

    pub fn create(&self, name: impl Into<String>, root: impl AsRef<Path>) -> Project {
        Project::new(name, root.as_ref())
    }
}
