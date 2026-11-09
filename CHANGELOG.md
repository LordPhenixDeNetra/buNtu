# Journal des modifications

Toutes les évolutions notables de buNtu sont consignées ici.

Le format suit [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/) et le projet respecte
le [versionnage sémantique](https://semver.org/lang/fr/) (avant 1.0, une mineure peut casser :
voir `docs/release.md`).

## [0.1.3] - 2026-11-09

### Corrections

- **npm** : Declare the repository required by npm provenance

## [0.1.2] - 2026-11-09

### Corrections

- **npm** : Read the launcher version from its own package.json

### Maintenance

- **npm** : Publish the launcher under the buntuu npm organisation
- **release** : Publish the npm launcher with npm trusted publishing

## [0.1.1] - 2026-11-09

### Corrections

- **release** : Publish semver image tags and the macOS launcher binary

### Maintenance

- **npm** : Publish the launcher as @buntu/cli

## [0.1.0] - 2026-11-08

Première **alpha publique** (cahier §14) : une passerelle IA auto-hébergée, en un seul binaire,
qui expose une API compatible OpenAI devant plusieurs fournisseurs.

**Points forts**

- Fournisseurs : OpenAI, Anthropic, Gemini, et tout service compatible OpenAI (préréglages vLLM,
  Groq, OpenRouter, Ollama).
- API : `/v1/chat/completions` (JSON et flux SSE), `/v1/embeddings`, `/v1/models`, et l'endpoint
  natif Anthropic `/anthropic/v1/messages`.
- Fiabilité : alias de modèles et chaînes de repli, réessais avec budget global, quatre
  timeouts, sélection pondérée des clés avec pause sur 429, TLS amont.
- Administration : API REST v1 protégée par jeton, console web FR/EN (thèmes clair et sombre),
  configuration en trois couches (base < fichier < environnement), secrets chiffrés au repos.
- Exploitation : journal des requêtes, métriques Prometheus, `buntu health`, image distroless
  non root avec volume `/data`.
- Livraison : binaires Linux (x86-64, ARM64), macOS ARM64 et Windows, image multi-arch, lanceur
  `npx buntu`, sommes signées et SBOM.

**Limites connues**

- Alpha : avant 1.0, une version mineure peut casser l'API ou la configuration.
- Le nom du projet est en cours de validation (T0.1, ADR-0001) et pourrait changer.
- Les crates ne sont pas publiées sur crates.io.
- Les messages d'erreur de l'API mélangent anglais et français (revue §5).
- La vue d'ensemble de la console calcule le trafic sur les 100 dernières requêtes du journal.

### Nouveautés

- **gateway** : Require a bearer token on the admin API
- **store** : Persist the configuration base layer
- **core** : Merge base, file and environment configuration layers
- **gateway** : Persist admin changes in the base configuration layer
- **ui** : Add the console design system and shared CSS tokens
- **ui** : Redesign the console on the design system
- **cli** : Add buntu health and a container HEALTHCHECK

### Corrections

- **store** : Write request log batches in one transaction
- **gateway** : Harden the upstream request path

### Sécurité

- **security** : Create the master key file on first start

### Refonte interne

- **providers** : Move the auth header into the Provider trait
- **gateway** : Share the request path and journal every route

### Documentation

- Add repository review checklist
- Tick review section 1 as done
- Move the specification and agent guide into docs/
- Refresh stale status and descriptions
- Record the configuration source decision (ADR-0008)
- **adr** : Propose ADR-0001 for the project name
- **adr** : Record preliminary trademark findings for the name

### Maintenance

- **release** : Trigger the delivery workflow after the Release workflow
- **release** : Fix cosign image casing, smoke repo context and npm version
- **release** : Download checksums before signing them and filter workflow_run
- **release** : Tag the container image with the release tag
- Remove empty placeholder crates and tidy manifests
- Fix .gitignore encoding and enforce LF line endings
- **ui** : Add @lucide/svelte for SVG icons
- **docker** : Move the image to deploy/ and harden it
- **release** : Add version management and the release procedure

## [0.0.1-rc.1] - 2026-11-04

### Nouveautés

- **mock** : Add deterministic provider endpoints and control headers
- **schema** : Add the canonical v0 types
- **gateway** : Add the HTTP server, health and graceful shutdown
- **cli** : Add the serve command
- **bench** : Add the scripted benchmark harness
- **core** : Add compiled configuration with hot reload
- **cli** : Add serve --config and config validate
- **bench** : Add the reload soak proof
- **schema** : Add the canonical response type
- **core** : Add request context, hooks and pipeline
- **core** : Declare the hook chain in the configuration
- **core** : Add weighted key selection with 429 pauses
- **core** : Add retry classification, backoff and budget
- **schema** : Let the all-targets failure carry the last error
- **core** : Add alias routing and the fallback chain
- **core** : Add the four request timeouts
- **schema** : Share capabilities between routing and providers
- **providers** : Add the provider trait
- **conformance** : Add the provider conformance kit
- **providers** : Add the OpenAI chat completions provider
- **providers** : Add Anthropic, Gemini and OpenAI-compatible providers
- **schema** : Add embeddings types and stable error metadata
- **providers** : Add embeddings translation and provider base urls
- **gateway** : Add the upstream client, normalized errors and OpenAI endpoints
- **gateway** : Add the native Anthropic messages endpoint
- **gateway** : Add the admin REST API v1 and generated OpenAPI
- **store** : Add SQLite storage with versioned migrations
- **observe** : Add the bounded asynchronous request log
- **observe** : Add Prometheus metrics and the /metrics endpoint
- **gateway** : Embed and serve the SvelteKit console
- **ui** : Add the SvelteKit console with i18n, theme, providers, routes and logs
- **gateway** : Record and expose the request log with cursor pagination
- **release** : Add cargo-dist, multi-arch image and verified npx launcher
- **core** : Generate the configuration JSON Schema and reference
- **bench** : Measure real gateway overhead and publish the first campaign

### Corrections

- **core** : Avoid the deprecated atomic fetch_update
- **providers** : Align the OpenAI-compatible provider with T1.12

### Sécurité

- **security** : Add Argon2id admin authentication and hashed API tokens
- **security** : Encrypt secrets at rest with AES-256-GCM and Secret<T>

### Documentation

- Add reference documents for buNtu
- Add agent instructions
- Add pull request template
- Point CLAUDE.md to AGENTS.md and reflect T0.2 state
- Record mock provider ceiling and protocol
- Add ADR-0002 canonical schema
- Add ADR-0003 to ADR-0007
- Add README and project governance files
- Explain the name buNtu
- Add the task tracking board
- Tick completed tasks in the task book
- Document the benchmark protocol and the local campaign
- Close T0.4 in the task tracking
- Close T1.2 in the task tracking
- Close T1.3 in the task tracking
- Close T1.4 in the task tracking
- Close T1.5 in the task tracking
- Close T1.6 in the task tracking
- Close T1.7 in the task tracking
- Close T1.8 in the task tracking
- **providers** : Document the OpenAI mapping
- Close T1.9 in the task tracking
- **providers** : Document the Anthropic, Gemini and compatible mappings
- Close T1.10 to T1.12 in the task tracking
- Record T1.13 to T1.16 and close E3 in the task tracking
- Require re-reading the task sheet before each task
- Record the provider pull request history
- Record T1.17 to T1.21 and close E4 in the task tracking
- Confirm the green CI for E3 and E4 in the task tracking
- Record T1.22 to T1.24 and close E5 in the task tracking
- Confirm the green CI for E5 in the task tracking
- Add the five-minute quickstart, provider guides and error FAQ
- Record T1.25 to T1.27 and close E6 in the task tracking

### Tests

- **schema** : Add OpenAI round-trip and passthrough tests
- **gateway** : Cover health, limits and graceful shutdown
- **core** : Cover configuration merge, validation and reload
- **core** : Cover pipeline order, short-circuit and hook errors
- **core** : Cover key selection statistics and pauses
- **core** : Cover retry classification and twenty percent budget
- **core** : Cover routing, fallback and the UC2 chaos scenario
- **core** : Cover timeouts, stream budgets and configuration overrides
- **conformance** : Add the fake provider fixtures
- **providers** : Cover OpenAI with unit tests and the conformance kit
- **conformance** : Generalise the battery to other providers
- **providers** : Cover Anthropic, Gemini and compatible providers
- **gateway** : Cover normalized errors and the OpenAI endpoints
- **gateway** : Cover the Anthropic endpoint, including cross-provider routing
- **gateway** : Cover the admin API and the OpenAPI diff
- **sdk** : Prove the endpoints with the official OpenAI, TypeScript and Anthropic SDKs
- **security** : Set 0600 on the master-key fixture for Linux
- **e2e** : Prove the console against a real gateway instance with Playwright and axe
- **e2e** : Replace secret-like test literals with explicit test data

### Maintenance

- Initialize repository
- Add gitignore and documentation checks
- Add Cargo workspace with 13 empty crates
- Add deny and nextest configuration
- Add GitHub Actions pipeline
- Add mock provider workspace member and dependencies
- Add Apache-2.0 license and notice
- Add issue templates and extend the pull request template
- Build the UI before Rust jobs and run the Playwright suite
- Validate docs, npx launcher and the Docker image
- **release** : Build the embedded console before dist build
- **release** : Build the embedded console before dist build
- **release** : Declare the console build through the dist build-setup hook
- **release** : Cd into ui inside the dist build-setup step

[0.1.0]: https://github.com/LordPhenixDeNetra/buNtu/compare/v0.0.1-rc.1...v0.1.0
[0.0.1-rc.1]: https://github.com/LordPhenixDeNetra/buNtu/releases/tag/v0.0.1-rc.1

