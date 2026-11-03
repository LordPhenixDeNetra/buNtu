# buNtu

[![CI](https://github.com/LordPhenixDeNetra/buNtu/actions/workflows/ci.yml/badge.svg)](https://github.com/LordPhenixDeNetra/buNtu/actions/workflows/ci.yml)

Passerelle IA open source en Rust, sous licence Apache 2.0. Objectif : une API unique
compatible OpenAI devant plus de 20 fournisseurs de modèles, avec bascule automatique,
répartition de charge, cache sémantique, budgets et passerelle MCP — et **toutes** les
fonctionnalités gratuites, y compris celles que les concurrents réservent à une offre
entreprise (SSO, RBAC, audit, clustering, guardrails).

> **Statut : pré-alpha.** Le produit n'est **pas utilisable** : aucune passerelle HTTP n'existe
> encore, aucun fournisseur n'est branché. Ce dépôt contient le socle technique et les documents
> de référence. Les fonctions listées ci-dessus sont des objectifs du cahier des charges, pas des
> fonctionnalités disponibles.

## Le nom

« buNtu » vient de **« Buntu »**, qui signifie **« porte » en wolof** : une porte d'entrée unique
vers les modèles, plutôt qu'une API par fournisseur.

## Ce qui existe aujourd'hui

| Élément | État |
| --- | --- |
| Cahier des charges et cahier des tâches | [document de référence](<buNtu — Cahier des charges et cahier des tâches.md>) |
| Guide d'exécution humain + agents IA | [guide](<Guide d'exécution (agents IA).md>) |
| Workspace Cargo (13 crates vides) | fait (T0.2) — `buntu --version` répond |
| CI : fmt, clippy, nextest, deny, audit, couverture, builds macOS/Windows | fait (T0.2) |
| Simulateur de fournisseur (tests et benchmarks) | fait (T0.3) — `mock-provider` |
| Schéma canonique v0 | fait (T0.5) — `buntu-schema` |
| ADR, README, CONTRIBUTING, SECURITY, licence | fait (T0.6/T0.7) |

## Démarrage (développement)

Prérequis : Rust stable (`rust-toolchain.toml`), `cargo-nextest`, `cargo-deny`,
`cargo-audit` et le composant `llvm-tools-preview`.

```sh
cargo build --workspace
cargo run --bin buntu                  # affiche « buntu 0.0.0 »
cargo nextest run --workspace --no-tests=pass
```

Le simulateur de fournisseur (OpenAI et Anthropic simulés) sert aux tests et benchmarks :

```sh
MOCK_LISTEN=127.0.0.1:18080 cargo run -p buntu-mock-provider
curl -s http://127.0.0.1:18080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"any","stream":false}'
```

Vérification complète avant de proposer un changement :

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings \
  && cargo nextest run --no-tests=pass && cargo deny check && cargo audit
```

## Documentation

- [Cahier des charges et cahier des tâches](<buNtu — Cahier des charges et cahier des tâches.md>) :
  périmètre, exigences `FR-*`/`NFR-*`, architecture, plan en quatre phases.
- [Guide d'exécution (agents IA)](<Guide d'exécution (agents IA).md>) : boucle de travail,
  règles absolues, fiches de tâches.
- [Décisions d'architecture](docs/adr/) : une fiche par décision.
- [Benchmarks](benchmarks/README.md) : protocole et plafond du simulateur.

## Contribuer

Lisez [CONTRIBUTING.md](CONTRIBUTING.md) et le [code de conduite](CODE_OF_CONDUCT.md). Le
fichier [AGENTS.md](AGENTS.md) fixe les règles que doivent suivre les agents IA.

## Sécurité

Voir [SECURITY.md](SECURITY.md). Ne signalez jamais une vulnérabilité en issue publique.

## Licence

Apache License 2.0 — voir [LICENSE](LICENSE) et [NOTICE](NOTICE). Aucune fonctionnalité ne sera
jamais réservée à une version payante ni conditionnée à une clé de licence.
