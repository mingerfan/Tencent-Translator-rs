use std::collections::HashMap;

use super::backend::TranslationBackend;
use super::error::TranslatorError;
use anyhow::Context;

pub struct TranslationManager {
    backends: HashMap<String, Box<dyn TranslationBackend>>,
}

impl TranslationManager {
    pub fn new() -> Self {
        Self {
            backends: HashMap::new(),
        }
    }

    pub fn add_backend(&mut self, backend: Box<dyn TranslationBackend>) {
        self.backends.insert(backend.name().to_string(), backend);
    }

    pub fn translate_with(
        &self,
        backend_name: &str,
        text: &str,
        from: &str,
        to: &str,
    ) -> anyhow::Result<String> {
        let backend = self.backends.get(backend_name)
            .ok_or_else(|| TranslatorError::NoAvailableBackend)?;

        if !backend.is_available() {
            return Err(TranslatorError::NoAvailableBackend.into());
        }

        backend.translate(text, from, to)
            .context(format!("Translation failed with backend {}", backend_name))
    }
}
