use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use anyhow::Context;

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
    pub fn config_path() -> anyhow::Result<PathBuf> {
        let current_dir = env::current_dir()
            .context("Failed to get current directory")?;
        Ok(current_dir.join("rs_translator_config.json"))
    }

    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context("Failed to read config file")?;
            let config = serde_json::from_str(&content)
                .context("Failed to parse config JSON")?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::config_path()?;
        let content = serde_json::to_string_pretty(self)
            .context("Failed to serialize config")?;
        fs::write(config_path, content)
            .context("Failed to write config file")?;
        Ok(())
    }

    pub fn get_prompt(&self, key: &str) -> Option<&str> {
        self.prompts.get(key).map(|s| s.as_str())
    }
}
