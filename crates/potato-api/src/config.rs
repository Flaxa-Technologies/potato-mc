use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// High-level configuration manager for PotatoMC plugins, modeled on Bukkit's FileConfiguration.
///
/// Supports nested keys (e.g., `"database.port"`, `"messages.welcome"`), automatic saving,
/// and fallback defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    data: serde_yaml::Value,
}

impl Config {
    /// Creates an empty configuration.
    pub fn new() -> Self {
        Self {
            data: serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        }
    }

    /// Parses configuration from a YAML string.
    pub fn from_yaml_str(yaml_str: &str) -> Result<Self, String> {
        let data: serde_yaml::Value = serde_yaml::from_str(yaml_str)
            .map_err(|e| format!("Failed to parse YAML configuration: {e}"))?;
        Ok(Self { data })
    }

    /// Loads a configuration from a file on disk.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;
        Self::from_yaml_str(&content)
    }

    /// Serializes this configuration to a YAML string.
    pub fn to_yaml_string(&self) -> Result<String, String> {
        serde_yaml::to_string(&self.data)
            .map_err(|e| format!("Failed to serialize config to YAML: {e}"))
    }

    /// Saves this configuration to the specified file path.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let p = path.as_ref();
        if let Some(parent) = p.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directories: {e}"))?;
            }
        }
        let yaml = self.to_yaml_string()?;
        fs::write(p, yaml).map_err(|e| format!("Failed to write config file: {e}"))
    }

    fn resolve_path<'a>(&'a self, path: &str) -> Option<&'a serde_yaml::Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut curr = &self.data;

        for part in parts {
            match curr {
                serde_yaml::Value::Mapping(map) => {
                    let key = serde_yaml::Value::String(part.to_string());
                    curr = map.get(&key)?;
                }
                _ => return None,
            }
        }

        Some(curr)
    }

    fn resolve_path_mut<'a>(&'a mut self, path: &str) -> &'a mut serde_yaml::Value {
        let parts: Vec<String> = path.split('.').map(String::from).collect();
        let mut curr = &mut self.data;

        for part in parts {
            if !curr.is_mapping() {
                *curr = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
            }

            if let serde_yaml::Value::Mapping(map) = curr {
                let key = serde_yaml::Value::String(part);
                if !map.contains_key(&key) {
                    map.insert(key.clone(), serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
                }
                curr = map.get_mut(&key).unwrap();
            }
        }

        curr
    }

    /// Retrieves a string value at the specified key path.
    pub fn get_string(&self, path: &str) -> Option<String> {
        match self.resolve_path(path)? {
            serde_yaml::Value::String(s) => Some(s.clone()),
            serde_yaml::Value::Number(n) => Some(n.to_string()),
            serde_yaml::Value::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// Retrieves a string value with a default fallback.
    pub fn get_string_or(&self, path: &str, default: &str) -> String {
        self.get_string(path).unwrap_or_else(|| default.to_string())
    }

    /// Retrieves a 64-bit integer at the specified key path.
    pub fn get_int(&self, path: &str) -> Option<i64> {
        match self.resolve_path(path)? {
            serde_yaml::Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Retrieves an integer value with a default fallback.
    pub fn get_int_or(&self, path: &str, default: i64) -> i64 {
        self.get_int(path).unwrap_or(default)
    }

    /// Retrieves a floating point number at the specified key path.
    pub fn get_float(&self, path: &str) -> Option<f64> {
        match self.resolve_path(path)? {
            serde_yaml::Value::Number(n) => n.as_f64(),
            _ => None,
        }
    }

    /// Retrieves a float value with a default fallback.
    pub fn get_float_or(&self, path: &str, default: f64) -> f64 {
        self.get_float(path).unwrap_or(default)
    }

    /// Retrieves a boolean value at the specified key path.
    pub fn get_bool(&self, path: &str) -> Option<bool> {
        match self.resolve_path(path)? {
            serde_yaml::Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Retrieves a boolean value with a default fallback.
    pub fn get_bool_or(&self, path: &str, default: bool) -> bool {
        self.get_bool(path).unwrap_or(default)
    }

    /// Retrieves a list of strings at the specified key path.
    pub fn get_string_list(&self, path: &str) -> Vec<String> {
        match self.resolve_path(path) {
            Some(serde_yaml::Value::Sequence(seq)) => seq
                .iter()
                .filter_map(|v| match v {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    serde_yaml::Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Sets a string value at the specified path.
    pub fn set_string(&mut self, path: &str, value: impl Into<String>) {
        let target = self.resolve_path_mut(path);
        *target = serde_yaml::Value::String(value.into());
    }

    /// Sets an integer value at the specified path.
    pub fn set_int(&mut self, path: &str, value: i64) {
        let target = self.resolve_path_mut(path);
        *target = serde_yaml::Value::Number(serde_yaml::Number::from(value));
    }

    /// Sets a float value at the specified path.
    pub fn set_float(&mut self, path: &str, value: f64) {
        let target = self.resolve_path_mut(path);
        *target = serde_yaml::Value::Number(serde_yaml::Number::from(value));
    }

    /// Sets a boolean value at the specified path.
    pub fn set_bool(&mut self, path: &str, value: bool) {
        let target = self.resolve_path_mut(path);
        *target = serde_yaml::Value::Bool(value);
    }

    /// Checks if a key path exists in this configuration.
    pub fn contains(&self, path: &str) -> bool {
        self.resolve_path(path).is_some()
    }
}
