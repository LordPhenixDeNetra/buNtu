# buNtu

[![CI](https://github.com/LordPhenixDeNetra/buNtu/actions/workflows/ci.yml/badge.svg)](https://github.com/LordPhenixDeNetra/buNtu/actions/workflows/ci.yml)

Passerelle IA open source en Rust, sous licence Apache 2.0. Objectif : une API unique
compatible OpenAI devant plus de 20 fournisseurs de modèles, avec bascule automatique,
répartition de charge, cache sémantique, budgets et passerelle MCP — et **toutes** les
fonctionnalités gratuites, y compris celles que les concurrents réservent à une offre
entreprise (SSO, RBAC, audit, clustering, guardrails).

> **Statut : alpha initiale.** La passerelle fonctionne localement : endpoints compatibles
> OpenAI et Anthropic, repli automatique, console web, journal des requêtes, stockage SQLite,
> métriques, chiffrement des secrets. Le cache sémantique, les budgets, les fournisseurs
> cloud natifs, le SSO/RBAC et le clustering restent à livrer (phases 2 et 3 du cahier).
> Aucune release publique n'a encore été publiée ; le pipeline de release est en place (T1.25).

## Démarrage rapide

Guide complet : **[docs/quickstart.md](docs/quickstart.md)** (5 minutes, sans compte fournisseur).

```sh
cd ui && npm ci && npm run build && cd ..
cargo build --release -p buntu-cli -p buntu-mock-provider
./target/release/buntu --version
```

Puis démarrez la passerelle avec le simulateur : voir le guide. La console est servie sur
`http://127.0.0.1:8080/` (fournisseurs, routes, journal), et les endpoints sont ceux d'OpenAI :

```sh
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"chat","messages":[{"role":"user","content":"bonjour"}]}'
curl -s http://127.0.0.1:8080/v1/models
```

## Ce qui fonctionne aujourd'hui

| Domaine | État |
| --- | --- |
| Moteur : pipeline, hooks, clés pondérées, réessais, alias, repli, timeouts | fait (T1.1–T1.7) |
| Fournisseurs : OpenAI, Anthropic, Gemini, compatible OpenAI (vLLM, Groq, OpenRouter, Ollama) | fait (T1.9–T1.12) + kit de conformité |
| API : `/v1/chat/completions` (JSON + SSE), `/v1/embeddings`, `/v1/models`, `/anthropic/v1/messages` | fait (T1.13–T1.15) |
| Administration REST v1 + `openapi.json` généré | fait (T1.16) |
| Stockage SQLite + migrations, journal asynchrone, métriques `/metrics`, auth admin, secrets AES-256-GCM | fait (T1.17–T1.21) |
| Console web embarquée (`rust-embed`), i18n FR/EN, thème, écrans fournisseurs/routes/journal | fait (T1.22–T1.24) |
| Pipeline de release (cargo-dist, Docker multi-arch, `npx`, cosign, SBOM), docs et campagne de benchmark | fait (T1.25–T1.27) |
| Cache sémantique, budgets/coûts, MCP, SSO/RBAC/audit, clustering | phases 2–3, non livrés |

## Documentation

- [Démarrage en 5 minutes](docs/quickstart.md)
- [Référence de configuration](docs/config-reference.md) — **générée** depuis les types Rust
- Guides fournisseurs : [OpenAI](docs/providers/openai.md), [Anthropic](docs/providers/anthropic.md),
  [Gemini](docs/providers/gemini.md), [Ollama](docs/providers/ollama.md),
  [compatible OpenAI](docs/providers/compatible.md)
- [Erreurs et dépannage](docs/faq-errors.md)
- [Campagne de benchmark et limites](docs/benchmarks.md)
- [Suivi des tâches](docs/status.md) · [Cahier des charges](<buNtu — Cahier des charges et cahier des tâches.md>)
  · [Guide d'exécution](<Guide d'exécution (agents IA).md>) · [ADR](docs/adr/)

## Développement

Prérequis : Rust stable (`rust-toolchain.toml`), Node.js ≥ 20, `cargo-nextest`, `cargo-deny`,
`cargo-audit`. L'interface (`ui/build`) est **embarquée dans le binaire** : construisez-la avant
tout `cargo build`/`cargo test` qui compile `buntu-gateway`.

```sh
(cd ui && npm ci && npm run build)
cargo build --workspace
cargo nextest run --workspace --no-tests=pass
```

Vérification complète avant de proposer un changement :

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings \
  && cargo nextest run --no-tests=pass && cargo deny check && cargo audit
```

Le simulateur de fournisseur (OpenAI et Anthropic simulés) sert aux tests et benchmarks :

```sh
MOCK_LISTEN=127.0.0.1:18080 cargo run -p buntu-mock-provider
```

Le banc de benchmark et le protocole sont décrits dans [benchmarks/README.md](benchmarks/README.md)
et [docs/benchmarks.md](docs/benchmarks.md) ; la campagne se lance avec
`cargo xtask bench --help`.

## Contribuer

Lisez [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md). Le
fichier [AGENTS.md](AGENTS.md) fixe les règles que doivent suivre les agents IA.

## Sécurité

Voir [SECURITY.md](SECURITY.md). Ne signalez jamais une vulnérabilité en issue publique.

## Licence

Apache License 2.0 — voir [LICENSE](LICENSE) et [NOTICE](NOTICE). Aucune fonctionnalité ne sera
jamais réservée à une version payante ni conditionnée à une clé de licence.
