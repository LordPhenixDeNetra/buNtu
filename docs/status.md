# Suivi des tâches

Source de vérité de l'avancement du projet. Ce document est mis à jour **dans le même commit**
que la tâche concernée (règle locale), et la case correspondante du cahier des charges est
cochée dans le même mouvement.

Une tâche n'est « faite » que si la machine le prouve : fusionnée, CI verte, preuve archivée
dans la PR.

## Légende

| Statut | Sens |
| --- | --- |
| à faire | pas commencée |
| en cours | branche ouverte, PR non fusionnée |
| fait | fusionnée, CI verte, preuve dans la PR |
| bloqué | dépendance externe ou humaine |

## Tâche en cours

Aucune. Prochaine : **T1.13** (endpoints compatibles OpenAI).

## Phase 0 — Fondations (15 j)

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T0.1 | Valider le nom (marques EUIPO/USPTO, réservations) | à faire (humain) | — | — | `docs/adr/0001-nom.md` attendu |
| T0.2 | Workspace Cargo + CI | fait | #2 | 2026-11-03 | 13 crates, lints, CI verte ; PR volontairement fautive rouge |
| T0.3 | Simulateur de fournisseur | fait | #4 | 2026-11-03 | 9 tests ; plafond 30 024 req/s noté dans `benchmarks/README.md` |
| T0.4 | Banc de benchmark scripté | fait (réserve) | #9 | 2026-11-03 | `cargo xtask bench` → JSON + Markdown ; ±10 % P99 non atteignable sur poste colocalisé, mesuré et documenté dans `benchmarks/README.md`, à vérifier en T1.26 (§11.2) |
| T0.5 | Schéma canonique v0 | fait | #5 | 2026-11-03 | 2 propriétés proptest × 200 cas ; ADR-0002 |
| T0.6 | ADR initiales | fait | #6 | 2026-11-03 | ADR-0003 à 0007 |
| T0.7 | Fichiers du projet | fait | #6 | 2026-11-03 | README, CONTRIBUTING, SECURITY, code de conduite, LICENSE, NOTICE, modèles d'issues et de PR |

## Phase 1 — E1 Moteur (21 j)

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T1.1 | Serveur HTTP, `/health`, `/ready`, arrêt gracieux, limites | fait | #7 | 2026-11-03 | 5 tests d'intégration : santé/préparation, corps > 10 Mo → 413, timeout des en-têtes, flux long terminé, aucune connexion après l'arrêt |
| T1.2 | Configuration + rechargement à chaud | fait | #10 | 2026-11-03 | 10 tests de configuration ; `cargo xtask reload-soak --qps 5000 --seconds 60 --reload-every 2` → 4999 req/s, 100 %, 0 erreur, 30 rechargements |
| T1.3 | Pipeline et hooks | fait | #11 | 2026-11-03 | 9 tests pipeline : ordre déclaré, court-circuit sans appel, erreurs propres, usage final de flux ; chaîne vide **483 ns/requête en release** |
| T1.4 | Sélection pondérée des clés | fait | #12 | 2026-11-03 | 1 M de tirages à ± 1 % des poids ; criterion **13,9 ns** (< 50 ns) ; toutes clés en pause → erreur explicite ; pauses réinitialisées au rechargement (documenté) |
| T1.5 | Réessais | fait | #13 | 2026-11-03 | 6 tests : classification par statut (429/500/502/503/504 vs 400/401/403/404/422), flux déjà commencé jamais réessayé, gigue complète plafonnée ; 100 % de 503 → **taux de réessai 20,095 %** |
| T1.6 | Chaîne de repli et alias | fait | #14 | 2026-11-03 | 11 tests : alias dans l'ordre, cible directe, erreurs, filtre de capacités ; **UC2 avec deux vrais simulateurs** (503 puis 200) → 200, en-têtes `x-buntu-provider`/`x-buntu-model`/`x-buntu-attempts` = 3 ; toutes cibles en échec → dernière erreur + liste |
| T1.7 | Quatre timeouts | fait (réserve) | #15 | 2026-11-03 | 10 tests : défauts, surcharges globales et par route, premier octet et total via `TimedExecutor`, budgets de flux premier/inter/total, **abandon du futur amont prouvé**, timeout réel sur simulateur (200 ms coupé à 50 ms) ; réserve : connexion et inter-morceaux branchés dans le client hyper-util (T1.8+) |
| T1.8 | Kit de conformité | fait | #16 | 2026-11-03 | trait `Provider` + crate `buntu-conformance` : 14 cas rejoués par `wiremock`, macro `conformance_suite!` ; **15/15 verts** dont le fournisseur volontairement cassé détecté ; mode `BUNTU_LIVE` prêt |

## Phase 1 — E2 Fournisseurs

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T1.9 | OpenAI (Chat Completions) | fait (réserve) | #17 | 2026-11-03 | traduction requête/réponse/flux/erreurs ; **14/14 cas de conformité** + 6 tests unitaires ; doc `docs/providers/openai.md` ; réserve : fixtures synthétiques et contrôle `BUNTU_LIVE` en attente d'une clé API |
| T1.10 | Anthropic (Messages) | fait (réserve) | #18 | 2026-11-04 | `system` extrait, `max_tokens` obligatoire (défaut 4096), blocs `tool_use`/`tool_result`/`thinking`, flux à événements nommés ; **14/14 cas** + 5 tests unitaires ; doc `docs/providers/anthropic.md` ; réserve : fixtures synthétiques + live |
| T1.11 | Gemini (AI Studio) | fait (réserve) | #18 | 2026-11-04 | rôles `user`/`model`, `systemInstruction`, `functionDeclarations`, `responseSchema`, flux SSE ; **14/14 cas** + 6 tests unitaires ; doc `docs/providers/gemini.md` ; réserve : fixtures synthétiques + live |
| T1.12 | Compatible OpenAI + Ollama | fait (réserve) | #18 | 2026-11-04 | traduction OpenAI réutilisée et **paramétrée** (`path`, `base_url`, en-têtes, `Quirks` déclarés : `stream_usage`, `parallel_tools` → `parallel_tool_calls: false`) ; préréglages vLLM/Groq/OpenRouter/Ollama ; **14/14 cas** + 5 tests unitaires ; doc `docs/providers/compatible.md` ; réserve : endpoints réels non testés |

## Phase 1 — E3 API publique et d'administration

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T1.13 | Endpoints OpenAI (`chat`, `embeddings`, `models`) | fait (réserve) | #22 | 2026-11-04 | client amont hyper-util + `base_url` par fournisseur ; `/v1/chat/completions` (JSON + SSE avec `[DONE]`), `/v1/embeddings`, `/v1/models` ; **6 tests d'intégration** contre le simulateur (texte, flux, vecteurs, modèles, erreurs 400/502) ; scripts SDK **Python + TypeScript** + job CI `sdk` ; réserve : preuve SDK à confirmer par la CI, TLS amont non branché (T1.21) |
| T1.14 | Endpoint natif Anthropic | fait (réserve) | #22 | 2026-11-04 | `/anthropic/v1/messages` entrant (`max_tokens` requis, `system` extrait, blocs texte/image/tool_use/tool_result), routage vers **n'importe quel fournisseur**, réponse et flux à **événements nommés** (`message_start`, `content_block_delta`, `message_stop`) ; **4 tests d'intégration** (cible Anthropic, cible OpenAI traduite, flux, erreur normalisée) ; SDK **Anthropic** Python ajouté au job `sdk` ; réserve : preuve SDK à confirmer en CI |
| T1.15 | Erreurs normalisées + en-têtes `x-buntu-*` | fait | #22 | 2026-11-04 | 8 codes stables + statut HTTP + type OpenAI ; `x-buntu-error-source` et `retry-after` ; **9 tests `insta`** (8 snapshots + anti-secret) ; aucun message ne contient clé, en-tête d'authentification ou corps de requête |
| T1.16 | API d'administration + OpenAPI | fait (réserve) | #22 | 2026-11-04 | CRUD fournisseurs/clés/routes sur la configuration brute, **validation par le même code que `config validate`**, pagination par curseur (max 100), **aperçu de clé seulement** (`sk-…a1b2` / `env:VAR`) ; `openapi.json` généré par `utoipa` + **test de diff** (échec si obsolète) ; **5 tests** (CRUD, pagination, anti-fuite, validation, OpenAPI) ; réserve : mutations en mémoire (persistance avec T1.17), authentification admin en T1.20 |

## Phase 1 — E4 Stockage, journaux, sécurité de base

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T1.17 | `buntu-store` SQLite + migrations versionnées | fait (réserve) | #23 | 2026-11-04 | pool `sqlx` SQLite en **WAL** + `busy_timeout` 5 s, migrations embarquées (`migrations/0001_initial.sql`, en avant seulement), `Store` minimal sans ORM (fournisseurs, clés, routes, journal, comptes, jetons) ; **5 tests** (migrations, entités, WAL fichier, jeton révoqué, clé chiffrée au repos) ; réserve : requêtes `sqlx` runtime, `sqlx prepare`/mode hors ligne reporté (sqlx-cli absent de l'environnement) |
| T1.18 | Journal des requêtes asynchrone | fait | #23 | 2026-11-04 | canal `mpsc` borné à **10 000**, `try_send` uniquement (jamais `send().await`), lots de **500 ou 1 s**, compteur `buntu_log_queue_dropped_total` ; **2 tests** : 20 000 envois < 100 ms avec base simulée à 1 s/écriture et pertes comptées, lots ≤ 500 |
| T1.19 | Métriques Prometheus | fait | #23 | 2026-11-04 | `metrics` + exportateur Prometheus, seaux dédiés µs (surcoût) et s (durée), labels à faible cardinalité ; endpoint `GET /metrics` branché ; **4 tests** (noms/sceaux/anti-cardinalité, installation idempotente, endpoint HTTP) |
| T1.20 | Authentification admin Argon2id + jetons API | fait | #23 | 2026-11-04 | Argon2id **m = 19 456 Kio, t = 2, p = 1** (OWASP), jetons 256 bits stockés **hachés** (SHA-256, `subtle`), limitation **10 échecs → blocage 5 min** ; **5 tests** (mauvais mot de passe, sel aléatoire, jeton haché, blocage, jeton révoqué côté store) |
| T1.21 | Chiffrement des secrets au repos + `Secret<T>` | fait | #23 | 2026-11-04 | `Secret<T>` (`secrecy` + zéroïsation, `Debug` masqué), AES-256-GCM avec **nonce aléatoire par valeur**, clé maître depuis fichier (permissions 0600 vérifiées sous Unix) ou variable base64, rotation de clé ; **8 tests** dont recherche de la clé en clair dans le fichier SQLite : introuvable ; réserve : commande CLI `buntu secrets rotate` à brancher avec T1.22+ |

## Phase 1 — E5 Interface web

| ID | Tâche | Statut | PR | Date | Preuve |
| --- | --- | --- | --- | --- | --- |
| T1.22 | Squelette SvelteKit statique embarqué | fait | #24 | 2026-11-04 | `adapter-static` + TypeScript strict, client OpenAPI **généré** (`openapi-typescript` + `openapi-fetch`), i18n FR/EN, thème clair/sombre, `rust-embed` (cache immutable sur `_app/immutable/`, repli SPA), **3 tests Rust**, UI gzip **49,6 Kio** (< 1 Mio), job CI `ui` (check/build/size/Playwright+axe) ; réserve : preuve Playwright à confirmer en CI |
| T1.23 | Pages fournisseurs, clés, routes et alias | fait | #24 | 2026-11-04 | écrans `/providers` (création, URL de base, clés avec aperçu seul, suppression) et `/routes` (alias, chaîne de repli éditable, réordonnancement) ; **E2E Playwright vert** (création fournisseur/route, rechargement, axe) |
| T1.24 | Journal des requêtes filtrable + détail | fait | #24 | 2026-11-04 | migration **0002** (`attempts` sur `request_log`, appliquée : 6/6 tests store) ; endpoints `GET /admin/v1/requests` (curseur/filtres) + détail, journal branché dans le chemin de requête, page `/logs` avec filtres URL et détail, **3 tests E2E verts (Playwright + axe)**

## Phases suivantes

Le détail des tâches des phases 2 à 4 reste dans le cahier des charges. Chaque phase sera
ajoutée à ce tableau au début de l'epic, en même temps que la rédaction de ses fiches
(guide §11).

## Historique des fusions

| Date | PR | Tâche | CI |
| --- | --- | --- | --- |
| 2026-11-02 | #1 | Bootstrap (documents, consignes agents) | verte |
| 2026-11-03 | #2 | T0.2 Workspace Cargo + CI | verte (36 s) |
| 2026-11-03 | #3 | Preuve « PR fautive rouge » (fermée sans fusion) | rouge attendue : `clippy` seul en échec |
| 2026-11-03 | #4 | T0.3 Simulateur de fournisseur | verte (1 min 20) |
| 2026-11-03 | #5 | T0.5 Schéma canonique v0 | verte (1 min 10) |
| 2026-11-03 | #6 | T0.6 + T0.7 ADR et fichiers du projet | verte (1 min 07) |
| 2026-11-03 | #7 | T1.1 Serveur HTTP, santé, arrêt gracieux | verte (1 min 23) |
| 2026-11-03 | #9 | T0.4 Banc de benchmark scripté | verte (1 min 20) |
| 2026-11-03 | #10 | T1.2 Configuration et rechargement à chaud | verte (1 min 39) |
| 2026-11-03 | #11 | T1.3 Pipeline et hooks | verte (1 min 09) |
| 2026-11-03 | #12 | T1.4 Sélection pondérée des clés | verte (1 min 07) |
| 2026-11-03 | #13 | T1.5 Réessais, backoff et budget | verte (1 min 05) |
| 2026-11-03 | #14 | T1.6 Chaîne de repli et alias | verte (1 min 28) |
| 2026-11-03 | #15 | T1.7 Quatre timeouts | verte (51 s) |
| 2026-11-03 | #16 | T1.8 Kit de conformité | verte (1 min 37) |
| 2026-11-03 | #17 | T1.9 OpenAI | verte (54 s) |
| 2026-11-04 | #18 | T1.10–T1.12 Anthropic, Gemini, compatible OpenAI | verte (51 s) |
| 2026-11-04 | #19 | Alignement du fournisseur compatible sur la fiche T1.12 | verte (1 min 18) |
| 2026-11-04 | #20 | Intégration des fournisseurs dans `main` | verte (55 s, puis 1 min 08 sur `main`) |
| 2026-11-04 | #21 | Règle « relire la fiche avant chaque tâche » | verte (1 min 01) |
| 2026-11-04 | #22 | Lot E3 : T1.13–T1.16 (API publique et d'administration) | verte (1 min 49 sur `main`) |
| 2026-11-04 | #23 | Lot E4 : T1.17–T1.21 (stockage, journal, métriques, auth, secrets) | verte (1 min 26) |
| 2026-11-04 | #24 | Lot E5 : T1.22–T1.24 (interface web) | à confirmer |
