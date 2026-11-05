# Guide d'exécution des tâches — humain et agents IA

## 1. Pourquoi ce guide

Un agent IA code vite mais dérive de façon prévisible. Il invente une API qui n'existe pas, désactive un test pour obtenir du vert, élargit le périmètre, ajoute des dépendances ou des abstractions inutiles, et affirme que c'est fini sans avoir rien exécuté. Ce guide ferme ces portes par trois mécanismes : des règles permanentes dans le dépôt, une fiche précise par tâche, et des preuves vérifiables par la machine plutôt que par la parole de l'agent.

**Comment l'utiliser.**

1. Copiez la section 2 dans `AGENTS.md` (et un lien depuis `CLAUDE.md`, `.cursorrules` ou équivalent) à la racine du dépôt, dès T0.2.
2. Pour chaque tâche, collez la fiche correspondante dans le modèle de prompt de la section 4.
3. Relisez avec la checklist de la section 5 avant toute fusion. Vous restez le seul à fusionner sur `main`.

Les fiches couvrent en détail les phases 0 et 1. Celles des phases 2 à 4 se rédigent juste avant de commencer chaque epic (section 11), car le code existant changera leur contenu.

## 2. Règles absolues pour tout agent (`AGENTS.md`)

Ce bloc se copie tel quel dans le dépôt. Chaque règle bloque un défaut observé chez les agents.

```markdown
# AGENTS.md — règles pour tout agent travaillant sur buNtu

## Périmètre
1. Ne travaille que sur la tâche donnée (identifiant Tx.y). Aucun « au passage ».
2. Ne modifie que les fichiers listés dans la fiche. Besoin d'en toucher un autre : arrête-toi et demande.
3. N'ajoute aucune dépendance Cargo ou npm sans accord explicite. Préfère la bibliothèque standard et les crates déjà présentes.
4. Ne crée aucune abstraction (trait, générique, macro, module) qui n'a pas au moins deux usages réels aujourd'hui.

## Vérité
5. N'invente aucune API, aucun champ, aucun endpoint. Pour une API de fournisseur, cite l'URL de la doc officielle consultée dans le commentaire du code et dans la PR.
6. Si tu ne sais pas, écris « je ne sais pas » et propose comment vérifier.
7. N'affirme jamais qu'une chose marche sans avoir exécuté la commande qui le prouve ; colle la sortie.

## Tests
8. Écris d'abord le test qui échoue, montre son échec, puis le code qui le fait passer.
9. Interdit : supprimer, ignorer (#[ignore]), affaiblir ou commenter un test ; baisser un seuil de couverture ou de performance ; ajouter #[allow(...)] ou désactiver un lint pour passer la CI.
10. Un test doit échouer si la fonctionnalité est cassée. Pas d'assertion vide ni de test qui teste le mock.

## Code Rust
11. Pas de `unsafe`. Pas de `.unwrap()` / `.expect()` / `panic!` hors tests et hors invariants documentés.
12. Erreurs typées avec `thiserror` dans les crates, `anyhow` seulement dans le binaire `buntu-cli`.
13. Aucun secret dans les logs, les erreurs ou les tests : type `Secret<T>`.
14. Chemin chaud : pas d'allocation inutile, pas de verrou bloquant, pas d'appel base de données.
15. Pas de `TODO` laissé sans issue liée.

## Avant de rendre la main
16. Lance : `cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo nextest run && cargo deny check`.
17. Résume : fichiers modifiés, tests ajoutés, sortie des commandes, ce qui reste incertain.
18. Un commit par étape logique, message conventionnel : `feat(core): T1.4 sélection pondérée des clés`.
```

## 3. Boucle de travail d'une tâche

Sept étapes, toujours dans cet ordre. Les étapes 1, 2 et 7 restent à vous ; l'agent fait 3 à 6.

1. **Cadrer (vous, 10 min).** Relire la fiche, vérifier que les tâches préalables sont fusionnées, créer la branche `tx.y-nom-court`.
2. **Plan (agent, validé par vous).** L'agent lit le code concerné et propose un plan en 5 à 10 lignes : fichiers touchés, tests prévus, risques. Vous validez ou corrigez avant toute ligne de code. C'est l'étape qui évite 80 % du n'importe quoi.
3. **Tests d'abord.** L'agent écrit les tests d'acceptation de la fiche et montre qu'ils échouent pour la bonne raison.
4. **Implémentation minimale.** Le plus petit code qui fait passer les tests, en petits commits.
5. **Vérification machine.** La commande de la règle 16, plus les commandes spécifiques de la fiche (benchmark, test de charge), sorties collées dans la PR.
6. **Rapport.** Ce qui a été fait, ce qui ne l'a pas été, ce qui est incertain, sources consultées.
7. **Revue et fusion (vous, 20 à 40 min).** Checklist de la section 5. En cas de doute, demandez à un second agent, dans une session neuve, de chercher les défauts de la PR.

**Taille.** Si le plan dépasse 400 lignes de diff ou 2 jours, découpez la tâche en sous-tâches `Tx.y.a`, `Tx.y.b` avant de commencer.

## 4. Modèle de prompt

Copiez ce modèle, remplacez les champs entre chevrons, collez la fiche de la tâche. Un prompt par tâche, dans une session neuve.

```markdown
Tu travailles sur buNtu, passerelle IA en Rust. Lis d'abord AGENTS.md et respecte-le strictement.

TÂCHE : <Tx.y — titre>
FICHE :
<coller la fiche complète : objectif, fichiers, étapes, tests, pièges, interdits, preuves>

CONTEXTE DÉJÀ FAIT : <tâches fusionnées utiles, ex. T1.1, T1.2>

DÉROULÉ IMPOSÉ :
1. Lis les fichiers listés et le code qu'ils appellent. Ne code rien.
2. Propose un plan (fichiers, tests, risques) et ATTENDS ma validation.
3. Après validation : tests d'abord (montre l'échec), puis code minimal.
4. Exécute les commandes de vérification et colle les sorties.
5. Termine par le rapport : fait / pas fait / incertain / sources.

Si une instruction de la fiche te semble fausse ou impossible, dis-le au lieu de la contourner.
```

## 5. Définition de « fait » et checklist de revue

Une tâche est faite quand la machine le prouve, pas quand l'agent le dit. Ces contrôles prennent 20 à 40 minutes et attrapent l'essentiel des dérives.

**Preuves obligatoires (bloquant)**

- [ ] CI verte : fmt, clippy `-D warnings`, nextest, deny, audit, couverture ≥ seuil.
- [ ] Chaque critère d'acceptation de la fiche a un test nommé qui le vérifie.
- [ ] Les commandes spécifiques de la fiche ont été exécutées, sorties dans la PR.
- [ ] Benchmarks micro sans régression > 10 %.

**Signaux de travail bâclé à chercher dans le diff**

| Signal | Commande ou geste pour le trouver |
| --- | --- |
| Test supprimé, ignoré ou affaibli | `git diff main -- '*test*' '*tests/*'` ; chercher `#[ignore]`, assertions retirées |
| Lint désactivé | `git diff main \| grep -n 'allow('` |
| Panique possible | `git diff main \| grep -nE 'unwrap\(\)\|expect\(\|panic!\|todo!\|unimplemented!'` |
| Dépendance ajoutée | `git diff main -- '**/Cargo.toml' ui/package.json` |
| Fichiers hors périmètre | `git diff --stat main` comparé à la liste de la fiche |
| API inventée | Chaque champ fournisseur a un lien vers la doc officielle ; ouvrir 2 liens au hasard |
| Abstraction inutile | Nouveau trait ou générique avec un seul implémenteur |
| Secret exposé | ` grep -rn 'Debug'  ` sur les types contenant des clés ; logs de test |
| Test qui teste le mock | Casser volontairement le code (inverser une condition) : le test doit échouer |
| Code mort ou dupliqué | Fonction quasi identique à une existante, `cargo udeps` pour les dépendances inutilisées |

**Le test du sabotage.** Pour les tâches critiques (routage, budgets, sécurité), modifiez une ligne clé à la main et relancez les tests. S'ils restent verts, la PR est refusée, quel que soit le reste. `cargo-mutants` automatise ce contrôle : lancez-le sur le crate touché à chaque fin d'epic.

## 6. Fiches Phase 0 — Fondations

Chaque fiche suit le même plan : objectif, prérequis, fichiers autorisés, étapes, preuves, pièges, interdits. Un agent n'a le droit de toucher que les fichiers listés.

### T0.1 — Valider le nom (1 j, vous seul)

- **Objectif :** éviter de devoir renommer après publication.
- **Étapes :**
  1. Rechercher « buNtu » et variantes (Buntu, buntu) dans les classes 9 et 42 sur [TMview](https://www.tmdn.org/tmview/) (couvre l'EUIPO et les offices nationaux) et la base USPTO.
  2. Vérifier la disponibilité : crates.io (`buntu`, `buntu-core`), organisation GitHub, npm, Docker Hub, domaine `.dev` ou `.io`.
  3. Réserver tout ce qui est libre le jour même (crates avec une version 0.0.0 vide, organisation, domaine).
- **Preuve :** capture ou lien de chaque réservation dans `docs/adr/0001-nom.md`.
- **Piège :** un agent ne peut pas juger un risque de confusion de marque ; en cas de doute réel, choisir un autre nom maintenant coûte 1 heure, plus tard plusieurs jours.

### T0.2 — Workspace Cargo et CI (2 j)

- **Objectif :** un squelette qui refuse automatiquement le mauvais code.
- **Fichiers :** `Cargo.toml` (workspace), `crates/*/Cargo.toml` + `src/lib.rs`, `rust-toolchain.toml`, `deny.toml`, `.github/workflows/ci.yml`, `AGENTS.md`, `.config/nextest.toml`.
- **Étapes :**
  1. Workspace avec les 13 crates de la section 6.2 du cahier, chacun avec un `lib.rs` vide sauf `buntu-cli` (`main.rs` qui affiche la version).
  2. `[workspace.lints]` : `unsafe_code = "forbid"`, `clippy::unwrap_used = "deny"`, `clippy::expect_used = "deny"`, hérités par chaque crate (`lints.workspace = true`).
  3. `[workspace.dependencies]` pour centraliser les versions ; profil release de la section 10.4.
  4. CI : jobs fmt, clippy, nextest, `cargo deny check`, `cargo audit`, couverture `cargo llvm-cov`, sur Linux ; build de vérification macOS et Windows.
  5. Copier `AGENTS.md` (section 2 de ce guide).
- **Preuves :** une PR volontairement fautive (un `unwrap()`) est rouge ; la PR propre est verte en moins de 10 minutes.
- **Pièges :** les agents ajoutent souvent des dépendances « utiles plus tard » : refuser, chaque crate commence vide.
- **Interdits :** aucune logique métier dans cette tâche.

### T0.3 — Simulateur de fournisseur (3 j)

- **Objectif :** un faux fournisseur rapide et déterministe, base de tous les tests et benchmarks.
- **Fichiers :** `tools/mock-provider/` (binaire séparé, hors des crates publiées).
- **Étapes :**
  1. Serveur axum qui imite `/v1/chat/completions` (JSON et SSE), `/v1/embeddings` et `/v1/messages` (format Anthropic).
  2. Comportement piloté par en-têtes : `x-mock-latency-ms`, `x-mock-tokens`, `x-mock-token-interval-ms`, `x-mock-status` (429, 500, 503), `x-mock-drop-after-tokens` (coupure TCP).
  3. Réponses construites une fois au démarrage et réutilisées (`Bytes`), pour que le simulateur ne soit jamais le goulot.
- **Preuves :** test d'intégration par comportement ; `oha` à 50 000 req/s sur 4 vCPU avec 0 erreur, sortie collée.
- **Piège :** un simulateur lent fausse tous les benchmarks : mesurer son plafond et le noter dans `benchmarks/README.md`.

### T0.4 — Banc de benchmark (3 j)

- **Objectif :** des chiffres reproductibles en une commande, dès le premier jour.
- **Fichiers :** `benchmarks/` (scripts, configurations, `README.md`), `xtask/` ou `justfile`.
- **Étapes :**
  1. Script qui déploie simulateur, passerelle et générateur (local en Docker, ou 3 VM via Terraform minimal).
  2. Charge en boucle ouverte avec `oha` (`--qps`) par paliers ; mesure directe vers le simulateur puis via la passerelle ; surcoût = différence.
  3. Collecte RSS (`/proc/<pid>/status`), CPU, percentiles ; résultat en JSON + tableau Markdown.
  4. Mesure de référence d'une passerelle existante avec sa configuration recommandée.
- **Preuves :** deux exécutions successives donnent des P99 à ± 10 %.
- **Pièges :** boucle fermée (omission coordonnée) qui masque les pics ; machine de charge plus faible que la passerelle ; oubli de la chauffe.

### T0.5 — Schéma canonique v0 (4 j)

- **Objectif :** le format interne unique ; s'il est mal conçu, chaque fournisseur le paiera.
- **Fichiers :** `crates/buntu-schema/src/**`.
- **Étapes :**
  1. Lire les références officielles OpenAI Chat Completions, Anthropic Messages et Gemini `generateContent` ; lister dans `docs/adr/0002-schema.md` chaque concept et sa correspondance (rôles, contenus multimodaux, outils, choix d'outil, raisons d'arrêt, usage, réflexion, cache de prompt).
  2. Types Rust : `Request`, `Message`, `ContentPart` (enum), `Tool`, `ToolCall`, `Usage`, `StopReason`, `StreamEvent`, `GatewayError`.
  3. Champs inconnus d'un fournisseur conservés dans un champ `extra` (passthrough) plutôt que perdus.
  4. Tests `proptest` d'aller-retour OpenAI ↔ canonique sur 200 cas générés + jeux réels anonymisés.
- **Pièges :** copier le format OpenAI tel quel (Anthropic et Gemini ne rentrent alors pas) ; `f64` pour l'usage ; `String` partout au lieu d'enums.
- **Interdits :** aucun code de fournisseur ici, seulement les types.

### T0.6 et T0.7 — ADR et fichiers du projet (2 j)

- **ADR :** un fichier par décision (`docs/adr/000N-titre.md`) selon l'annexe B du cahier : runtime tokio, hyper/axum, SQLite + PostgreSQL, UI, licence.
- **Fichiers projet :** README (pitch, démarrage, statut « alpha »), CONTRIBUTING (DCO, commits conventionnels, lien vers AGENTS.md), SECURITY, CODE\_OF\_CONDUCT (Contributor Covenant), LICENSE Apache 2.0 et NOTICE, modèles d'issues et de PR avec la checklist de la section 5.
- **Piège :** un README qui promet des fonctions inexistantes ; n'écrire que ce qui marche.

## 7. Fiches Phase 1 — E1 Moteur

Le moteur est la partie où un agent peut faire le plus de dégâts invisibles : un verrou mal placé ou une annulation mal gérée ne casse aucun test unitaire mais ruine le P99. Chaque fiche impose donc une preuve sous charge.

### T1.1 — Serveur HTTP, santé, arrêt gracieux (3 j)

- **Prérequis :** T0.2, T0.3.
- **Fichiers :** `crates/buntu-gateway/src/{server.rs,health.rs,shutdown.rs}`, `crates/buntu-cli/src/main.rs`.
- **Étapes :**
  1. `axum::serve` sur `TcpListener` tokio, HTTP/1.1 + HTTP/2 (h2c en option).
  2. `/health` renvoie 200 sans rien vérifier ; `/ready` renvoie 200 seulement quand la config est chargée.
  3. Couches `tower` : limite de corps (10 Mo par défaut), timeout de lecture des en-têtes, limite de connexions simultanées.
  4. Arrêt gracieux sur SIGTERM/SIGINT : `/ready` passe à 503, plus de nouvelles connexions, attente des flux jusqu'à 30 s, puis sortie.
- **Preuves :** test qui lance un flux long, envoie SIGTERM et vérifie que le flux se termine complet ; test corps > 10 Mo → 413.
- **Pièges :** `/ready` qui appelle les fournisseurs à chaque sonde (tempête d'appels sous Kubernetes) ; arrêt qui coupe les flux en cours.

### T1.2 — Configuration et rechargement à chaud (4 j)

- **Prérequis :** T1.1.
- **Fichiers :** `crates/buntu-core/src/config/{mod.rs,file.rs,env.rs,compiled.rs}`, `crates/buntu-cli/src/config_cmd.rs`.
- **Étapes :**
  1. Deux niveaux de types : `RawConfig` (désérialisé du TOML, tolérant) et `CompiledConfig` (validé, indexé, prêt pour le chemin chaud).
  2. Fusion fichier + `BUNTU_*` + résolution `env:VAR` ; erreurs qui citent le chemin du champ (`providers[1].keys[0].weight`).
  3. `ArcSwap<CompiledConfig>` global ; rechargement sur SIGHUP, sur modification du fichier (`notify`, avec anti-rebond 500 ms) et via l'API.
  4. Une config invalide au rechargement est rejetée, l'ancienne reste active, l'erreur est journalisée.
  5. `buntu config validate <fichier>`.
- **Preuves :** test de charge 5 000 req/s pendant 60 s avec un rechargement toutes les 2 s : 0 erreur ; test de config invalide qui garde l'ancienne.
- **Pièges :** `RwLock` au lieu d'`ArcSwap` (contention) ; garder un `Guard` d'ArcSwap à travers un `.await` (cloner l'`Arc` à la place).

### T1.3 — Pipeline et hooks (4 j)

- **Prérequis :** T0.5, T1.2.
- **Fichiers :** `crates/buntu-core/src/{pipeline.rs,hooks.rs,context.rs}`.
- **Étapes :**
  1. `RequestContext` : identifiant, instant de début, clé virtuelle résolue, métadonnées, extensions typées.
  2. Traits `PreHook` (peut modifier la requête ou court-circuiter avec une réponse, ex. cache) et `PostHook` (voit la réponse ou le flux terminé, jamais bloquant).
  3. Ordre des hooks fixe et déclaré à la compilation de la config ; liste stockée dans `CompiledConfig`.
  4. Pour les flux : les `PostHook` reçoivent l'usage final via un événement de fin, pas chaque morceau.
- **Preuves :** tests : court-circuit, ordre, erreur dans un hook = requête en erreur propre (pas de panique) ; micro-bench de la chaîne vide < 1 µs.
- **Pièges :** `Box<dyn Fn>` alloué à chaque requête ; hooks qui clonent tout le corps de la requête.
- **Interdits :** système de plugins dynamiques (c'est T3.12).

### T1.4 — Sélection pondérée des clés (2 j)

- **Fichiers :** `crates/buntu-core/src/keys.rs`.
- **Étapes :** table d'alias de Walker (ou somme cumulée + recherche binaire si < 16 clés) construite dans `CompiledConfig` ; filtre des clés en pause (429) par un `AtomicU64` « pause jusqu'à » ; générateur aléatoire rapide par thread (`fastrand`).
- **Preuves :** test statistique sur 1 M de tirages : chaque clé à ± 1 % de son poids ; `criterion` < 50 ns ; test où toutes les clés sont en pause → erreur claire.
- **Piège :** recalculer la table à chaque requête.

### T1.5 — Réessais (3 j)

- **Fichiers :** `crates/buntu-core/src/retry.rs`.
- **Étapes :** classification des erreurs (récupérable : 429, 500, 502, 503, 504, timeout, reset ; non récupérable : 400, 401, 403, 404, 422) ; backoff exponentiel avec gigue complète, plafond ; budget global de réessais (au plus 20 % des requêtes sur 10 s) pour éviter l'effet boule de neige.
- **Preuves :** tests par code d'erreur avec le simulateur ; test : 100 % d'erreurs 503 → le taux de réessais plafonne à 20 %.
- **Piège crucial :** ne jamais réessayer un flux déjà commencé côté client (le client recevrait deux débuts de réponse) ; après le premier octet envoyé, une erreur se transmet telle quelle.

### T1.6 — Chaîne de repli et alias (3 j)

- **Prérequis :** T1.4, T1.5.
- **Fichiers :** `crates/buntu-core/src/{routing.rs,fallback.rs}`.
- **Étapes :** résolution `fournisseur/modèle` ou alias → liste ordonnée de cibles ; pour chaque cible, réessais (T1.5) puis passage à la suivante ; en-têtes de réponse `x-buntu-provider`, `x-buntu-model`, `x-buntu-attempts`.
- **Preuves :** scénario UC2 en test d'intégration (premier simulateur en 503, second OK) : réponse 200, en-têtes corrects ; test : toutes les cibles en échec → dernière erreur renvoyée avec la liste des tentatives.
- **Piège :** repli vers un modèle qui ne supporte pas une capacité demandée (outils, vision) : filtrer par capacités ou échouer clairement.

### T1.7 — Quatre timeouts (2 j)

- **Fichiers :** `crates/buntu-core/src/timeouts.rs`, intégration dans le client amont.
- **Étapes :** connexion (5 s), premier octet (60 s, réglable par modèle : les modèles à réflexion sont lents), inter-morceaux (30 s), total (10 min) ; chacun configurable par route.
- **Preuves :** un test par timeout avec `x-mock-latency-ms` ; vérifier que la connexion amont est bien fermée (pas de fuite de tâche) après expiration.
- **Piège :** `tokio::time::timeout` sur toute la requête seulement, qui tue les longs flux légitimes.

## 8. Fiches Phase 1 — E2 Fournisseurs

C'est le terrain où les agents hallucinent le plus : ils « connaissent » des API qui ont changé depuis leur entraînement. Règle spéciale pour cet epic : chaque champ traduit cite sa source officielle, et chaque fournisseur est validé contre la vraie API au moins une fois avant fusion.

### T1.8 — Kit de conformité (4 j, à faire avant tout fournisseur)

- **Fichiers :** `crates/buntu-providers/src/provider.rs` (trait), `crates/buntu-conformance/` (crate de test, non publié), `tests/fixtures/<fournisseur>/`.
- **Étapes :**
  1. Trait `Provider` : `build_request(&CanonicalRequest) -> http::Request`, `parse_response(Bytes) -> CanonicalResponse`, `parse_stream_chunk(&mut StreamState, Bytes) -> Vec<StreamEvent>`, `map_error(status, body) -> GatewayError`, `capabilities(model)`.
  2. Batterie de cas commune : texte simple, système + multi-tours, flux, outil appelé, outil en parallèle, résultat d'outil, image en entrée, sortie JSON structurée, arrêt par longueur, filtre de contenu, 400, 401, 429 avec `retry-after`, 500.
  3. Fixtures : réponses réelles enregistrées (clés et données personnelles retirées), rejouées par `wiremock` ; macro `conformance_suite!(MonFournisseur)` qui génère tous les tests.
  4. Mode `--live` (variable `BUNTU_LIVE=1`) qui joue la même batterie contre la vraie API, utilisé la nuit.
- **Preuves :** la batterie tourne contre un faux fournisseur trivial et détecte une erreur volontaire de traduction.

### T1.9 à T1.11 — OpenAI, Anthropic, Gemini (5 j chacun)

Même procédure pour les trois, un fournisseur par PR.

- **Fichiers :** `crates/buntu-providers/src/<fournisseur>/{mod.rs,request.rs,response.rs,stream.rs,errors.rs}`, fixtures.
- **Étapes :**
  1. Ouvrir la référence officielle à jour ; recopier dans `docs/providers/<fournisseur>.md` le tableau de correspondance canonique ↔ fournisseur, avec l'URL et la date de consultation.
  2. Enregistrer les fixtures réelles pour tous les cas du kit (coût : quelques centimes avec un petit modèle).
  3. Implémenter requête, réponse, flux, erreurs jusqu'à 100 % de la batterie.
  4. Lancer une fois en `--live`, coller le résultat dans la PR.
- **Points spécifiques :**
  - **OpenAI :** `stream_options.include_usage` pour obtenir l'usage en flux ; arguments d'outils reçus en fragments à recoller par index.
  - **Anthropic :** `system` hors des messages ; `max_tokens` obligatoire (valeur par défaut configurable) ; flux à événements nommés (`message_start`, `content_block_delta`, `message_delta`…) ; usage d'entrée au début, de sortie à la fin ; blocs de réflexion et de cache de prompt à préserver.
  - **Gemini :** rôles `user` / `model`, `systemInstruction` séparé ; clé en en-tête plutôt qu'en paramètre d'URL (pour ne pas la journaliser) ; schémas d'outils limités à un sous-ensemble d'OpenAPI.
- **Pièges :** tamponner tout le flux avant de le traduire ; perdre l'usage d'un flux ; confondre raison d'arrêt « outil » et « fin ».
- **Interdits :** tout champ sans source ; toute fixture inventée à la main présentée comme réelle.

### T1.12 — Compatible OpenAI générique et Ollama (3 j)

- **Objectif :** couvrir des dizaines de services avec un seul traducteur paramétré.
- **Étapes :** fournisseur `openai_compatible` qui réutilise T1.9 avec `base_url`, en-têtes et chemin configurables, et une liste de « quirks » déclarés (pas d'usage en flux, pas d'outils parallèles…) ; Ollama par ce biais via son endpoint compatible.
- **Preuves :** kit en `--live` contre Ollama local (petit modèle), vLLM en conteneur, et au moins un service hébergé (Groq ou OpenRouter).
- **Piège :** dupliquer le code OpenAI au lieu de le paramétrer.

## 9. Fiches Phase 1 — E3 API et E4 Stockage, sécurité

Ici, le juge de paix est le SDK officiel : si le SDK OpenAI ou Anthropic fonctionne sans modification, l'API est bonne ; un test écrit à la main par l'agent ne suffit pas.

### T1.13 — Endpoints compatibles OpenAI (4 j)

- **Prérequis :** T1.6, T1.9.
- **Fichiers :** `crates/buntu-gateway/src/routes/openai/{chat.rs,embeddings.rs,models.rs}`, `tests/sdk/python/`, `tests/sdk/ts/`.
- **Étapes :** désérialiser vers le canonique, exécuter le pipeline, sérialiser ; flux SSE avec `data: [DONE]` final ; `/v1/models` liste les modèles et alias configurés.
- **Preuves :** scripts de tests avec les SDK officiels Python et TypeScript (texte, flux, outils) contre buNtu + simulateur, exécutés en CI.
- **Piège :** oublier `Content-Type: text/event-stream` et le vidage immédiat de chaque morceau (sinon le client reçoit tout d'un bloc).

### T1.14 — Endpoint natif Anthropic (3 j)

- **Fichiers :** `crates/buntu-gateway/src/routes/anthropic/messages.rs`.
- **Étapes :** accepter le format Messages entrant (avec `x-api-key` contenant la clé virtuelle), router vers n'importe quel fournisseur, répondre au format Anthropic, événements de flux compris.
- **Preuves :** SDK Anthropic officiel avec `base_url` vers buNtu, vers une cible Anthropic et vers une cible OpenAI.

### T1.15 — Erreurs normalisées (2 j)

- **Fichiers :** `crates/buntu-schema/src/error.rs`, `crates/buntu-gateway/src/error_response.rs`.
- **Étapes :** chaque `GatewayError` a un code stable (`provider_rate_limited`, `all_targets_failed`, `budget_exceeded`…), un statut HTTP, un message utile ; format OpenAI `{"error":{"message","type","code"}}` ; en-têtes `x-buntu-error-source`.
- **Preuves :** test instantané (`insta`) de chaque erreur ; aucune erreur ne contient de clé ni de corps de requête.

### T1.16 — API d'administration (5 j)

- **Fichiers :** `crates/buntu-gateway/src/admin/**`, `openapi.json` généré.
- **Étapes :** CRUD fournisseurs, clés, routes ; pagination par curseur ; validation identique à `config validate` (même code) ; OpenAPI généré par `utoipa` et vérifié en CI (diff = échec si non régénéré).
- **Piège :** une API d'admin qui renvoie les secrets ; ne jamais renvoyer qu'un aperçu (`sk-...a1b2`).

### T1.17 — Stockage SQLite et migrations (4 j)

- **Fichiers :** `crates/buntu-store/**`, `migrations/`.
- **Étapes :** `sqlx` avec requêtes vérifiées (`sqlx prepare`, mode hors ligne en CI) ; SQLite en WAL ; migrations en avant seulement ; interface `Store` minimale (pas d'ORM).
- **Preuves :** test qui part d'une base vide, applique toutes les migrations, écrit et relit chaque entité.
- **Piège :** SQLite partagé entre threads sans pool ni `busy_timeout` → erreurs « database is locked » sous charge.

### T1.18 — Journal des requêtes asynchrone (3 j)

- **Fichiers :** `crates/buntu-observe/src/request_log.rs`.
- **Étapes :** `tokio::sync::mpsc` borné (10 000) ; `try_send` depuis le chemin chaud (jamais `send().await`) ; tâche d'écriture par lots (500 lignes ou 1 s) ; compteur `buntu_log_queue_dropped_total`.
- **Preuves :** test où la base est ralentie artificiellement à 1 s par écriture : la latence des requêtes ne bouge pas, les pertes sont comptées.

### T1.19 — Métriques Prometheus (2 j)

- **Fichiers :** `crates/buntu-observe/src/metrics.rs`.
- **Étapes :** métriques de la section 9.1 du cahier avec la crate `metrics` et son exportateur Prometheus ; histogrammes à seaux définis pour les µs (surcoût) et pour les secondes (durée totale).
- **Piège :** label à forte cardinalité (identifiant de requête, clé virtuelle) qui fait exploser la mémoire.

### T1.20 — Authentification admin (3 j)

- **Fichiers :** `crates/buntu-security/src/{password.rs,tokens.rs,session.rs}`.
- **Étapes :** mot de passe initial généré et affiché une fois au premier démarrage (ou fourni par variable) ; Argon2id avec paramètres OWASP ; jetons API aléatoires 256 bits stockés hachés ; comparaison en temps constant (`subtle`) ; limitation des tentatives de connexion.
- **Preuves :** tests : mauvais mot de passe, jeton révoqué, 10 échecs → blocage temporaire.
- **Interdits :** écrire sa propre cryptographie ; stocker un jeton en clair.

### T1.21 — Chiffrement des secrets (2 j)

- **Fichiers :** `crates/buntu-security/src/secrets.rs`, `crates/buntu-schema/src/secret.rs`.
- **Étapes :** type `Secret<T>` (crate `secrecy`) sans `Debug` ni `Display` lisibles ; AES-256-GCM (`aes-gcm` ou `ring`), nonce aléatoire par valeur ; clé maître lue depuis un fichier 0600 ou une variable, jamais en base ; rotation de clé maître via `buntu secrets rotate`.
- **Preuves :** test qui écrit une clé fournisseur puis cherche sa valeur en clair dans le fichier SQLite et dans les journaux : introuvable.

## 10. Fiches Phase 1 — E5 Interface et E6 Livraison

Pour l'interface, les agents produisent vite des écrans qui ont l'air finis mais mentent : données fictives codées en dur, boutons sans effet. Chaque écran doit être testé de bout en bout contre une vraie instance.

### T1.22 — Squelette de l'interface (3 j)

- **Fichiers :** `ui/**`, `crates/buntu-gateway/src/ui.rs`.
- **Étapes :** SvelteKit avec `adapter-static`, TypeScript strict ; client d'API généré depuis `openapi.json` (`openapi-typescript`), jamais écrit à la main ; i18n FR/EN dès le départ ; thème clair/sombre ; `rust-embed` sert les fichiers compilés avec cache long sur les fichiers hachés.
- **Preuves :** binaire release qui sert l'UI sans Node installé ; poids gzip < 1 Mo mesuré en CI.
- **Interdit :** bibliothèque de composants lourde ajoutée sans accord.

### T1.23 et T1.24 — Écrans de configuration et journal (9 j)

- **Étapes :** fournisseurs, clés (secret saisi une fois, jamais réaffiché), routes et alias avec réordonnancement de la chaîne de repli ; journal paginé côté serveur, filtres dans l'URL (partageable), détail d'une requête avec tentatives et latences.
- **Preuves :** tests Playwright : créer un fournisseur pointant vers le simulateur, envoyer une requête, la retrouver dans le journal ; contrôle d'accessibilité `axe` sans violation grave.
- **Pièges :** filtrer 100 000 lignes dans le navigateur ; états de chargement et d'erreur oubliés.

### T1.25 — Release automatisée (4 j)

- **Fichiers :** `.github/workflows/release.yml`, `dist-workspace.toml`, `Dockerfile`, `npx/`.
- **Étapes :** `cargo-dist` pour les binaires et l'installateur ; Docker multi-arch (`buildx`), image distroless non root ; paquet npm lanceur qui télécharge le binaire et vérifie sa somme SHA-256 ; signature `cosign` et SBOM.
- **Preuves :** une release de test `v0.0.1-rc.1` publiée de bout en bout ; `docker run` puis `curl` OK sur amd64 et arm64.
- **Interdit :** jeton de publication dans un fichier du dépôt ; utiliser les secrets GitHub et la publication de confiance (OIDC) de crates.io et npm.

### T1.26 — Campagne de benchmark publiée (4 j)

- **Étapes :** protocole de la section 11.2 du cahier, machines louées à l'heure, résultats bruts dans `benchmarks/results/<date>/`, page de documentation avec graphiques et limites de l'exercice.
- **Piège :** publier un chiffre flatteur obtenu une seule fois ; toujours publier la médiane de 3 exécutions avec son écart.
- **Interdit :** comparer à une autre passerelle mal configurée ; faire relire sa configuration par ses mainteneurs si possible.

### T1.27 — Documentation de démarrage (4 j)

- **Étapes :** guide « 5 minutes », un guide par fournisseur, référence de configuration générée depuis les types Rust (`schemars` → JSON Schema → page), FAQ des erreurs.
- **Preuves :** chaque bloc de commande de la doc est exécuté en CI (tests de documentation) ; une personne extérieure réussit UC1 en moins de 5 minutes, chronométrée.
- **Piège :** doc écrite par un agent qui décrit des options inexistantes ; la référence générée l'empêche.

## 11. Phases 2 à 4 : fiches juste à temps

Les fiches des phases suivantes ne sont pas rédigées maintenant : elles dépendent du code réel de la phase 1, et une fiche écrite un an trop tôt pousserait un agent à suivre un plan périmé. Elles se produisent au début de chaque epic, en une demi-journée.

**Procédure, au début de chaque epic :**

1. Demander à un agent, dans une session neuve, de lire le code actuel, le cahier des charges (exigences FR/NFR de l'epic) et ce guide, puis de rédiger une fiche par tâche au format des sections 6 à 10.
2. Exiger pour chaque fiche : fichiers autorisés réels (existants ou à créer), preuves exécutables, au moins deux pièges et un interdit.
3. Demander à un second agent de critiquer les fiches : critère non vérifiable, fichier oublié, tâche trop grosse, dépendance manquante.
4. Relire, corriger, ajouter les fiches à cet onglet sous un titre « Fiches Phase N — Ex ».

**Points de vigilance déjà connus pour ces phases :**

| Tâche | Ce qu'un agent risque de mal faire | Garde-fou à écrire dans la fiche |
| --- | --- | --- |
| T2.2 Bedrock SigV4 | Implémenter la signature AWS à la main, avec erreurs subtiles | Utiliser `aws-sigv4` officiel ; test contre les vecteurs de test AWS |
| T2.12 Budgets | Compteurs perdus ou doublés sous concurrence | Test concurrent 1 000 tâches, somme exacte ; test de mutation obligatoire |
| T2.14 Coûts | `f64`, arrondis faux | `rust_decimal` imposé ; table de cas calculés à la main |
| T2.18 Cache sémantique | Répondre avec le cache d'un autre utilisateur | Isolation par clé virtuelle testée ; seuil par défaut prudent (0,95) |
| T2.21 MCP | Réimplémenter le protocole | SDK officiel `rmcp` imposé ; tests contre les serveurs MCP de référence |
| T3.1 / T3.2 SSO | Accepter un jeton non vérifié (signature, audience, expiration) | Crates `openidconnect` / `samael` ; tests de jetons forgés, expirés, mauvaise audience |
| T3.4 RBAC | Oublier la vérification sur une route | Matrice rôle × route générée automatiquement, test qui échoue si une route n'est pas classée |
| T3.6 Audit | Chaîne de hash cassée par l'ordre d'insertion concurrent | Écriture sérialisée par une seule tâche ; `buntu audit verify` en test |
| T3.9 Clustering | Consensus maison | Pas de consensus : PostgreSQL + Redis comme sources de vérité ; test de perte de nœud |
| T3.12 Plugins WASM | Plugin qui bloque ou consomme toute la mémoire | Quotas `wasmtime` (fuel, mémoire) ; test d'un plugin en boucle infinie |
