use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

use super::error::Error;
use super::backend::BackendConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_backend: Option<String>,
    pub backends: HashMap<String, BackendConfig>,
    #[serde(default)]
    pub prompts: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut prompts = HashMap::new();
        prompts.insert(
            "translation".to_string(),
            r#"You are a professional translator. 
Translate the following text accurately while preserving the original meaning and style. 
Only return the translated text without any explanations or additional content.

Text to translate: {text}
Source language: {source}
Target language: {target}"#.to_string(),
        );

        Self {
            default_backend: None,
            backends: HashMap::new(),
            prompts,
        }
    }
}

impl Config {
    pub fn config_path() -> Result<PathBuf, Error> {
        let current_dir = env::current_dir()
            .map_err(|e| Error::ConfigError(format!("Failed to get current directory: {}", e)))?;
        Ok(current_dir.join("rs_translator_config.json"))
    }

    pub fn load() -> Result<Self, Error> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<(), Error> {
        let config_path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    pub fn get_prompt(&self, key: &str) -> Option<&str> {
        self.prompts.get(key).map(|s| s.as_str())
    }
}
