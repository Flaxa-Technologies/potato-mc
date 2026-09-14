pub use crate::context::PluginContext;

/// Metadata describing a plugin's identity and dependencies.
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: String,
    pub dependencies: Vec<String>,
}

impl PluginMetadata {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            authors: Vec::new(),
            description: String::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn dependency(mut self, dependency: impl Into<String>) -> Self {
        self.dependencies.push(dependency.into());
        self
    }
}

/// The core trait that all Potato native plugins implement.
pub trait Plugin: Send + Sync + 'static {
    /// Returns the metadata for this plugin.
    fn metadata(&self) -> PluginMetadata;

    /// Lifecycle hook called immediately when the plugin library is loaded.
    fn on_load(&self, _ctx: &PluginContext) -> Result<(), String> {
        Ok(())
    }

    /// Lifecycle hook called when the plugin is enabled and the server is ready.
    fn on_enable(&self, _ctx: &PluginContext) -> Result<(), String> {
        Ok(())
    }

    /// Lifecycle hook called when the plugin is disabled prior to server shutdown or unload.
    fn on_disable(&self, _ctx: &PluginContext) -> Result<(), String> {
        Ok(())
    }
}
