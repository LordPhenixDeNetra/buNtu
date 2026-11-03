# buNtu — consignes pour les agents

Passerelle IA open source en Rust, sous licence Apache 2.0 : une API unique compatible OpenAI
devant plus de 20 fournisseurs de modèles, avec repli automatique, répartition de charge,
cache sémantique, budgets et passerelle MCP. Décisions : `docs/adr/`.

## Documents de référence

- `buNtu — Cahier des charges et cahier des tâches.md` : périmètre, exigences `FR-*`/`NFR-*`,
  architecture, plan de quatre phases.
- `Guide d'exécution (agents IA).md` : règles, fiches de tâches, définition de « fait ».
- `docs/status.md` : suivi des tâches (statut, PR, preuve), mis à jour dans le même commit que
  la tâche ; la case correspondante du cahier des charges est cochée en même temps.

Chaque tâche porte un identifiant `T<phase>.<n>` (`T0.2`, `T1.4`…) et n'est faite que si la
machine le prouve. Une seule tâche en cours à la fois.

## Structure prévue (cahier §6.2)

- `crates/buntu-schema` — types canoniques (messages, outils, usage, erreurs), sérialisation.
- `crates/buntu-core` — pipeline, routage, réessais, disjoncteurs, sélection de clés, hooks.
- `crates/buntu-providers` — un module par fournisseur : requête, réponse, flux, erreurs.
- `crates/buntu-gateway` — serveur HTTP, endpoints compatibles, API d'administration, UI.
- `crates/buntu-store` — configuration et données (SQLite par défaut, PostgreSQL), migrations.
- `crates/buntu-governance` — clés virtuelles, hiérarchie, budgets, limites de débit.
- `crates/buntu-cache` — cache exact et sémantique, index vectoriel.
- `crates/buntu-mcp` — client et passerelle MCP.
- `crates/buntu-security` — authentification, OIDC/SAML/SCIM, RBAC, audit, secrets, guardrails.
- `crates/buntu-observe` — métriques, traces, journal des requêtes, exports.
- `crates/buntu-plugin-wasm` — hôte de plugins WebAssembly.
- `crates/buntu-cluster` — appartenance au cluster, compteurs partagés, diffusion de config.
- `crates/buntu-cli` — binaire `buntu` : `serve`, `import`, `bench`, `doctor`, `config validate`.
- `tools/mock-provider` — fournisseur simulé (T0.3), hors des crates publiées.
- `xtask/` — `cargo xtask bench` : banc de benchmark scripté (T0.4).
- `ui/` — interface SvelteKit statique ; `benchmarks/` — banc et résultats ; `docs/adr/`.

## Commande unique

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo nextest run --no-tests=pass && cargo deny check
```

`--no-tests=pass` reste nécessaire tant qu'aucun test n'existe : `cargo nextest run` échoue
avec zéro test. La CI ajoute `cargo audit` et la couverture (`cargo llvm-cov`). Prérequis :
Rust stable (`rust-toolchain.toml`), `cargo-nextest`, `cargo-deny`, `cargo-audit` et le
composant `llvm-tools-preview`.

## Règles

### Périmètre

1. Ne travaille que sur la tâche donnée (`Tx.y`). Aucun « au passage ».
2. Ne modifie que les fichiers listés dans la fiche. Besoin d'en toucher un autre : arrête-toi et demande.
3. N'ajoute aucune dépendance Cargo ou npm sans accord explicite. Préfère la bibliothèque standard et les crates déjà présentes.
4. Ne crée aucune abstraction (trait, générique, macro, module) qui n'a pas au moins deux usages réels aujourd'hui.

### Vérité

5. N'invente aucune API, aucun champ, aucun endpoint. Pour une API de fournisseur, cite l'URL de la doc officielle consultée dans le commentaire du code et dans la PR.
6. Si tu ne sais pas, écris « je ne sais pas » et propose comment vérifier.
7. N'affirme jamais qu'une chose marche sans avoir exécuté la commande qui le prouve ; colle la sortie.

### Tests

8. Écris d'abord le test qui échoue, montre son échec, puis le code qui le fait passer.
9. Interdit : supprimer, ignorer (`#[ignore]`), affaiblir ou commenter un test ; baisser un seuil de couverture ou de performance ; ajouter `#[allow(...)]` ou désactiver un lint pour passer la CI.
10. Un test doit échouer si la fonctionnalité est cassée. Pas d'assertion vide ni de test qui teste le mock.

### Code Rust

11. Pas de `unsafe` (`unsafe_code = "forbid"`). Pas de `.unwrap()` / `.expect()` / `panic!` hors tests et hors invariants documentés.
12. Erreurs typées avec `thiserror` dans les crates, `anyhow` seulement dans le binaire `buntu-cli`.
13. Aucun secret dans les logs, les erreurs ou les tests : type `Secret<T>`.
14. Chemin chaud : pas d'allocation inutile, pas de verrou bloquant, pas d'appel base de données ; les flux SSE ne sont jamais réassemblés en entier.
15. Pas de `TODO` laissé sans issue liée.

### Avant de rendre la main

16. Lance la commande unique ci-dessus, puis `cargo audit` ; colle les sorties.
17. Résume : fichiers modifiés, tests ajoutés, sortie des commandes, ce qui reste incertain.
18. Un commit par étape logique, message conventionnel : `feat(core): T1.4 sélection pondérée des clés`.

## Règles du projet

- Licence Apache 2.0 ; Rust stable, édition 2024, `resolver = "3"` ; MSRV mise à jour au plus
  tous les 6 mois.
- `#![forbid(unsafe_code)]` dans tous les crates, sans exception.
- Clean room (cahier §2) : aucune copie de code ou de documentation d'un projet tiers ; le nom
  « Bifrost » n'est jamais utilisé.
- Aucune fonctionnalité derrière une clé de licence, jamais (cahier §16.1).
- Aucune panique atteignable par une entrée réseau (NFR-REL-6) ; toute erreur est typée et
  attribuée à un fournisseur.
- Montants et coûts en `rust_decimal`, jamais en `f64`.
- Une fixture de fournisseur est réelle et anonymisée, jamais inventée à la main.

## Cibles

Linux x86-64 et ARM64 (musl statique), macOS ARM64, Windows x86-64 (meilleur effort). La CI de
référence tourne sous Linux ; une régression macOS ou Windows doit être signalée, pas ignorée.

## i18n

L'interface est bilingue FR/EN (cahier FR-UI-6) : aucune chaîne visible en dur dans `ui/`.
Les documents du dépôt sont rédigés en français.

## État et périmètre

**Le socle est en place (T0.2, T0.3, T0.4, T0.5, T0.6/T0.7, T1.1).** Le workspace Cargo
(13 crates), la CI complète, le simulateur de fournisseur (`tools/mock-provider`), le banc de
benchmark (`cargo xtask bench`), le schéma canonique v0 (`buntu-schema`), les ADR et les
fichiers du projet existent, et `buntu serve` démarre un serveur HTTP avec `/health`, `/ready`
et arrêt gracieux. Aucune logique métier n'est encore écrite. T0.4 est livré **avec réserve** :
le critère ± 10 % de P99 n'est pas atteignable sur un poste où le générateur est colocalisé ; il
sera vérifié par la campagne §11.2 (T1.26). Les prochains travaux sont **T0.1** (preuve de
disponibilité du nom, côté humain), **T1.2** (configuration et rechargement à chaud), puis
**T1.3** (pipeline) et **T1.8–T1.11** (fournisseurs). Le suivi est tenu dans `docs/status.md`.

Toute ligne de code écrite avant sa tâche est hors périmètre.
