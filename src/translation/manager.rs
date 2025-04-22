use super::backend::TranslationBackend;
use super::error::Error;

pub struct TranslationManager {
    backends: Vec<Box<dyn TranslationBackend>>,
}

impl Default for TranslationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TranslationManager {
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
        }
    }

    pub fn add_backend(&mut self, backend: Box<dyn TranslationBackend>) {
        self.backends.push(backend);
    }

    #[allow(unused)]
    pub fn translate(&self, text: &str, from: &str, to: &str) -> Result<String, Error> {
        // Try each available backend in order
        for backend in &self.backends {
            if backend.is_available() {
                match backend.translate(text, from, to) {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        eprintln!("Backend {} failed: {}", backend.name(), e);
                        continue;
                    }
                }
            }
        }
        Err(Error::NoAvailableBackend)
    }

    #[allow(unused)]
    pub fn list_available_backends(&self) -> Vec<&'static str> {
        self.backends
            .iter()
            .filter(|b| b.is_available())
            .map(|b| b.name())
            .collect()
    }

    pub fn translate_with(&self, backend_name: &str, text: &str, from: &str, to: &str) -> Result<String, Error> {
        for backend in &self.backends {
            if backend.name() == backend_name && backend.is_available() {
                return backend.translate(text, from, to);
            }
        }
        Err(Error::NoAvailableBackend)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBackend {
        name: &'static str,
        available: bool,
        response: Option<String>,
    }

    impl TranslationBackend for MockBackend {
        fn translate(&self, _text: &str, _from: &str, _to: &str) -> Result<String, Error> {
            self.response
                .clone()
                .ok_or_else(|| Error::TranslationFailed("Mock translation failed".to_string()))
        }

        fn name(&self) -> &'static str {
            self.name
        }

        fn is_available(&self) -> bool {
            self.available
        }
    }

    #[test]
    fn test_translation_manager() {
        let mut manager = TranslationManager::new();
        
        let mock1 = MockBackend {
            name: "mock1",
            available: true,
            response: Some("Hello".to_string()),
        };
        
        let mock2 = MockBackend {
            name: "mock2",
            available: true,
            response: Some("Bonjour".to_string()),
        };

        manager.add_backend(Box::new(mock1));
        manager.add_backend(Box::new(mock2));

        assert_eq!(manager.list_available_backends(), vec!["mock1", "mock2"]);
        
        let result = manager.translate("test", "en", "fr").unwrap();
        assert_eq!(result, "Hello");

        let result = manager.translate_with("mock2", "test", "en", "fr").unwrap();
        assert_eq!(result, "Bonjour");
    }
}
