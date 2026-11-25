//! Kit de conformité en réel (T1.8 étape 4, T1.9 à T1.12) : ignoré sans `BUNTU_LIVE=1`.
//!
//! ```bash
//! BUNTU_LIVE=1 BUNTU_LIVE_OPENAI_API_KEY=… BUNTU_LIVE_OPENAI_MODEL=gpt-4o-mini \
//!   cargo test -p buntu-conformance --test live -- --nocapture
//! ```
//!
//! Préfixes : `OPENAI`, `ANTHROPIC`, `GEMINI` (clé exigée) et `COMPATIBLE` (`BASE_URL` exigée,
//! clé facultative : `http://127.0.0.1:11434` pour Ollama).

use buntu_conformance::live::{LiveTarget, run_live};
use buntu_providers::anthropic::AnthropicProvider;
use buntu_providers::compatible::OpenAiCompatibleProvider;
use buntu_providers::gemini::GeminiProvider;
use buntu_providers::openai::OpenAiProvider;
use buntu_providers::provider::Provider;

fn play<P: Provider>(
    provider: &P,
    prefix: &str,
    base_url: &str,
    key_required: bool,
) -> Result<(), String> {
    let Some(target) = LiveTarget::from_env(prefix, base_url, key_required) else {
        eprintln!("{prefix}: skipped (BUNTU_LIVE=1 and BUNTU_LIVE_{prefix}_* not set)");
        return Ok(());
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    let results = runtime.block_on(run_live(provider, &target));
    let mut failures = Vec::new();
    for (name, result) in &results {
        match result {
            Ok(()) => eprintln!("{prefix} {} live {name}: ok", target.model),
            Err(error) => {
                eprintln!("{prefix} {} live {name}: FAILED {error}", target.model);
                failures.push(format!("{name}: {error}"));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("\n"))
    }
}

#[test]
fn openai_live() -> Result<(), String> {
    play(
        &OpenAiProvider::default(),
        "OPENAI",
        "https://api.openai.com",
        true,
    )
}

#[test]
fn anthropic_live() -> Result<(), String> {
    play(
        &AnthropicProvider::default(),
        "ANTHROPIC",
        "https://api.anthropic.com",
        true,
    )
}

#[test]
fn gemini_live() -> Result<(), String> {
    play(
        &GeminiProvider::default(),
        "GEMINI",
        "https://generativelanguage.googleapis.com",
        true,
    )
}

#[test]
fn compatible_live() -> Result<(), String> {
    play(
        &OpenAiCompatibleProvider::default(),
        "COMPATIBLE",
        "",
        false,
    )
}
