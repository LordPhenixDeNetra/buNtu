//! Registre des traducteurs disponibles, indexés par le `type` de la configuration.

use std::collections::HashMap;
use std::sync::Arc;

use buntu_providers::anthropic::AnthropicProvider;
use buntu_providers::compatible::OpenAiCompatibleProvider;
use buntu_providers::gemini::GeminiProvider;
use buntu_providers::openai::OpenAiProvider;
use buntu_providers::provider::Provider;

/// Traducteurs connus.
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn Provider>>,
}

impl std::fmt::Debug for ProviderRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderRegistry")
            .field("kinds", &self.providers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut providers: HashMap<String, Arc<dyn Provider>> = HashMap::new();
        providers.insert("openai".to_owned(), Arc::new(OpenAiProvider::default()));
        providers.insert(
            "anthropic".to_owned(),
            Arc::new(AnthropicProvider::default()),
        );
        providers.insert("gemini".to_owned(), Arc::new(GeminiProvider::default()));
        providers.insert(
            "compatible".to_owned(),
            Arc::new(OpenAiCompatibleProvider::default()),
        );
        providers.insert(
            "openai_compatible".to_owned(),
            Arc::new(OpenAiCompatibleProvider::default()),
        );
        // Le simulateur T0.3 imite l'API OpenAI : on le branche par le traducteur compatible.
        providers.insert(
            "mock".to_owned(),
            Arc::new(OpenAiCompatibleProvider::default()),
        );
        providers.insert(
            "ollama".to_owned(),
            Arc::new(OpenAiCompatibleProvider::preset(
                buntu_providers::compatible::Compatible::Ollama,
            )),
        );
        Self { providers }
    }

    pub fn with_provider(mut self, kind: impl Into<String>, provider: Arc<dyn Provider>) -> Self {
        self.providers.insert(kind.into(), provider);
        self
    }

    pub fn get(&self, kind: &str) -> Option<Arc<dyn Provider>> {
        self.providers.get(kind).cloned()
    }

    pub fn kinds(&self) -> Vec<&str> {
        let mut kinds: Vec<&str> = self.providers.keys().map(String::as_str).collect();
        kinds.sort_unstable();
        kinds
    }
}
