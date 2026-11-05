//! En-tête d'authentification porté par chaque fournisseur (revue 2026-11-05, section 2).

use buntu_providers::anthropic::AnthropicProvider;
use buntu_providers::compatible::OpenAiCompatibleProvider;
use buntu_providers::gemini::GeminiProvider;
use buntu_providers::openai::OpenAiProvider;
use buntu_providers::provider::Provider;

fn header(provider: &dyn Provider) -> Result<(String, String), Box<dyn std::error::Error>> {
    let (name, value) = provider.auth_header("sk-test")?;
    assert!(value.is_sensitive(), "la valeur doit être marquée sensible");
    Ok((name.as_str().to_owned(), value.to_str()?.to_owned()))
}

#[test]
fn each_provider_names_its_own_auth_header() -> Result<(), Box<dyn std::error::Error>> {
    let bearer = ("authorization".to_owned(), "Bearer sk-test".to_owned());
    assert_eq!(header(&OpenAiProvider::default())?, bearer);
    assert_eq!(header(&OpenAiCompatibleProvider::default())?, bearer);
    assert_eq!(
        header(&AnthropicProvider::default())?,
        ("x-api-key".to_owned(), "sk-test".to_owned())
    );
    assert_eq!(
        header(&GeminiProvider::default())?,
        ("x-goog-api-key".to_owned(), "sk-test".to_owned())
    );
    Ok(())
}

#[test]
fn invalid_secret_is_an_error_not_a_panic() {
    assert!(
        OpenAiProvider::default()
            .auth_header("bad\nsecret")
            .is_err()
    );
}
