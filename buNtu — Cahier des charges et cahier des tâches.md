# buNtu — Cahier des charges et cahier des tâches

Nov 2, 2026 · @SAD

## 1. Résumé exécutif

buNtu est une passerelle IA open source écrite en Rust, sous licence Apache 2.0, dont toutes les fonctionnalités sont gratuites, y compris celles que les concurrents réservent à une offre entreprise (SSO, RBAC, audit, clustering, guardrails). Elle expose une API unique compatible OpenAI devant plus de 20 fournisseurs de modèles, avec bascule automatique, répartition de charge, cache sémantique, budgets et passerelle MCP.

Le projet est porté par un développeur seul. Le plan livre donc en quatre phases : des fondations puis un cœur minimal utilisable en production (phases 0 et 1, environ 7 mois), la gouvernance et le cache (phase 2), les fonctions entreprise (phase 3), puis l'écosystème (phase 4). Chaque phase produit une version publiable.

**Promesse mesurable.** buNtu ne promet pas « 50x plus rapide » : le surcoût d'une passerelle moderne est déjà de l'ordre de 10 à 60 µs pour des réponses de fournisseurs de 0,5 à 3 s. La promesse porte sur ce qui se mesure et compte en exploitation :

- empreinte mémoire au repos ≤ 30 Mo et ≤ 150 Mo à 5 000 req/s ;
- surcoût P99 ≤ 100 µs à 5 000 req/s, sans pics dus à un ramasse-miettes ;
- ≥ 20 000 req/s soutenues sur 4 vCPU contre un fournisseur simulé ;
- 100 % de succès côté passerelle quand au moins un fournisseur de repli répond ;
- un binaire unique, statique, de moins de 40 Mo, démarré en moins de 200 ms.

**Vision.** Être la passerelle IA de référence pour qui veut tout le périmètre entreprise sans licence payante, avec des performances publiées, reproductibles et vérifiables par n'importe qui.

## 2. Contexte, objectifs et contraintes

Les passerelles IA existantes suivent un modèle « open core » : le routage est gratuit, mais SSO, RBAC, audit, clustering et guardrails sont payants. buNtu supprime cette frontière. C'est une réécriture indépendante : aucun code ni texte de documentation d'un projet tiers n'est copié, et le nom « Bifrost » n'est jamais utilisé.

### Objectifs (mesurables)

| ID | Objectif | Mesure de réussite |
| --- | --- | --- |
| O1 | API unique compatible OpenAI | 100 % des tests de conformité OpenAI Chat, Embeddings, Responses passent |
| O2 | Haute disponibilité applicative | 0 erreur visible si ≥ 1 fournisseur de repli répond |
| O3 | Sobriété | Mémoire et surcoût P99 aux seuils de la section 5 |
| O4 | Tout gratuit | Aucune fonctionnalité derrière une clé de licence, jamais |
| O5 | Adoption | 1 000 étoiles GitHub et 10 contributeurs externes 12 mois après la v1.0 |
| O6 | Remplacement direct | Les SDK OpenAI, Anthropic et Google GenAI fonctionnent en changeant seulement l'URL de base |

### Non-objectifs

- Héberger ou entraîner des modèles : buNtu route, il n'infère pas.
- Proposer un service cloud payant géré par le projet.
- Remplacer un outil d'évaluation de prompts ou un IDE de prompts complet.
- Supporter des fournisseurs sans API documentée publiquement.

### Hypothèses

- Un seul développeur, environ 30 h par semaine, à l'aise en Rust asynchrone ou prêt à le devenir.
- Les fournisseurs gardent des API HTTP + JSON + SSE ; la majorité du trafic est en streaming.
- Les utilisateurs déploient sur Linux x86-64 ou ARM64, en conteneur ou en binaire.

### Contraintes

- **Langage :** Rust stable (MSRV fixée, mise à jour au plus tous les 6 mois), `unsafe` interdit hors modules audités et listés.
- **Licence :** Apache 2.0 ; dépendances compatibles uniquement (MIT, Apache, BSD, ISC, Zlib), vérifiées par `cargo-deny` en CI.
- **Nom :** « buNtu » à valider avant v0.1 (recherche de marques EUIPO/USPTO, disponibilité sur crates.io, GitHub, npm, Docker Hub et du domaine).
- **Budget :** 0 € imposé ; CI sur GitHub Actions gratuit, benchmarks sur machines louées à l'heure (≈ 20 € par campagne).

## 3. Personas et cas d'usage

Cinq profils structurent les priorités : le développeur doit être opérationnel en 5 minutes, l'équipe plateforme doit pouvoir l'exploiter sans surprise.

| Persona | Besoin principal | Ce qui le fait adopter | Phase |
| --- | --- | --- | --- |
| Développeur d'application IA | Une seule API, changer de modèle sans recoder | `docker run` puis un `curl` qui marche en < 5 min | 1 |
| Équipe plateforme / SRE | Haute dispo, métriques, faible coût d'infra | Binaire sobre, Prometheus, Helm, déploiement sans état | 1-2 |
| Responsable FinOps | Maîtriser la dépense par équipe et par client | Budgets hiérarchiques, coûts temps réel, alertes | 2 |
| RSSI / conformité | Contrôler l'accès, tracer, filtrer les données | SSO, RBAC, audit inaltérable, guardrails PII | 3 |
| Agent / outil de code | Passer par une passerelle MCP et des modèles variés | Passerelle MCP, compatibilité SDK Anthropic | 2-3 |

### Cas d'usage de référence

1. **UC1 — Appel unifié.** Une app envoie `model: "anthropic/claude-x"` sur `/v1/chat/completions` ; buNtu traduit vers l'API Anthropic et renvoie le format OpenAI, en streaming.
2. **UC2 — Panne fournisseur.** OpenAI renvoie 503 ; buNtu rejoue sur Azure OpenAI puis sur un modèle de repli, sans erreur pour l'appelant.
3. **UC3 — Quota dépassé.** Une clé reçoit 429 ; buNtu la met en pause selon `retry-after` et route sur les autres clés.
4. **UC4 — Budget.** L'équipe « Marketing » atteint 100 % de son budget mensuel ; ses clés virtuelles reçoivent 402 avec un message clair, une alerte part à 80 %.
5. **UC5 — Cache sémantique.** Une question reformulée (similarité ≥ 0,95) est servie depuis le cache en < 5 ms.
6. **UC6 — Agent MCP.** Un agent appelle les outils de 3 serveurs MCP via un seul point d'entrée, filtré par RBAC.
7. **UC7 — Audit.** Le RSSI exporte qui a appelé quel modèle, quand, avec quel coût, sur 90 jours.
8. **UC8 — Migration.** Une équipe sur une autre passerelle importe sa configuration (fournisseurs, clés, budgets) via un outil `buntu import`.

## 4. Exigences fonctionnelles

Chaque exigence a un identifiant stable (`FR-<module>-<n>`), une priorité MoSCoW (M = indispensable, S = souhaitable, C = si le temps le permet) et une phase. Une exigence est « faite » quand son test d'acceptation automatisé passe en CI.

### 4.1 API et compatibilité (API)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-API-1 | `POST /v1/chat/completions` compatible OpenAI, streaming SSE et non-streaming | M | 1 |
| FR-API-2 | `POST /v1/embeddings`, `GET /v1/models` | M | 1 |
| FR-API-3 | `POST /v1/responses` (API Responses OpenAI) | S | 2 |
| FR-API-4 | Points d'entrée natifs `/anthropic/v1/messages`, `/genai/v1beta/...`, `/bedrock/...` pour le remplacement direct des SDK | M (Anthropic), S (autres) | 1-2 |
| FR-API-5 | Images (génération), audio (transcription, synthèse), fichiers et batch | S | 3 |
| FR-API-6 | Appels d'outils (tool calling), sortie JSON structurée, vision, traduits entre formats | M | 1 |
| FR-API-7 | Nommage `fournisseur/modèle` et alias configurables (`gpt-best` → liste ordonnée) | M | 1 |
| FR-API-8 | Erreurs normalisées au format OpenAI, avec le code et le fournisseur d'origine en en-tête `x-buntu-*` | M | 1 |
| FR-API-9 | API d'administration REST versionnée (`/api/v1/...`) + schéma OpenAPI publié | M | 1 |
| FR-API-10 | Endpoints `/health` (vivant) et `/ready` (prêt, fournisseurs joignables) | M | 1 |

### 4.2 Fournisseurs (PRV)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-PRV-1 | OpenAI, Anthropic, Google Gemini, Ollama, compatible-OpenAI générique (vLLM, LM Studio, OpenRouter, Groq, Together…) | M | 1 |
| FR-PRV-2 | Azure OpenAI, AWS Bedrock (SigV4), Google Vertex (OAuth service account), Mistral, Cohere | M | 2 |
| FR-PRV-3 | xAI, DeepSeek, Cerebras, Fireworks, Perplexity, Hugging Face, Replicate, Cloudflare, Databricks, SageMaker | S | 3 |
| FR-PRV-4 | Interface `Provider` unique : traduction requête, traduction réponse, traduction flux, calcul des tokens et du coût | M | 1 |
| FR-PRV-5 | Catalogue de modèles (prix par token, contexte max, capacités) mis à jour sans recompiler (fichier distant signé + surcharge locale) | M | 2 |
| FR-PRV-6 | Fournisseur personnalisé déclaré par configuration (URL, en-têtes, mapping) sans code | S | 2 |

### 4.3 Fiabilité et routage (RTE)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-RTE-1 | Plusieurs clés par fournisseur, sélection pondérée | M | 1 |
| FR-RTE-2 | Chaîne de repli (fallback) ordonnée entre fournisseurs et modèles, déclenchée sur 429, 5xx, timeout, erreur réseau | M | 1 |
| FR-RTE-3 | Réessais avec backoff exponentiel + gigue, budget de réessais global pour éviter les tempêtes | M | 1 |
| FR-RTE-4 | Timeouts distincts : connexion, premier token, inter-tokens, total | M | 1 |
| FR-RTE-5 | Disjoncteur (circuit breaker) par clé et par fournisseur, avec demi-ouverture | M | 2 |
| FR-RTE-6 | Répartition adaptative : poids recalculés selon latence et taux d'erreur observés (EWMA) | S | 3 |
| FR-RTE-7 | Règles de routage déclaratives (par en-tête, clé virtuelle, taille du prompt, coût) | S | 3 |
| FR-RTE-8 | Requêtes « hedged » optionnelles (2e requête après le P95 du premier token) | C | 4 |
| FR-RTE-9 | Respect de `retry-after` et des quotas RPM/TPM déclarés par clé | M | 2 |

### 4.4 Gouvernance et coûts (GOV)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-GOV-1 | Clés virtuelles : modèles autorisés, fournisseurs autorisés, expiration, révocation immédiate | M | 2 |
| FR-GOV-2 | Hiérarchie Organisation → Business unit → Équipe → Client → Clé virtuelle | M | 2 |
| FR-GOV-3 | Budgets par niveau, période (jour, semaine, mois) et remise à zéro, blocage dur ou souple | M | 2 |
| FR-GOV-4 | Limites de débit RPM et TPM par niveau (token bucket) | M | 2 |
| FR-GOV-5 | Coût calculé par requête (entrée, sortie, cache, raisonnement) en décimal exact, pas en flottant | M | 2 |
| FR-GOV-6 | Alertes de seuil (50/80/100 %) par webhook, e-mail, Slack | S | 2 |
| FR-GOV-7 | Rapports de consommation exportables (CSV, API) | S | 2 |

### 4.5 Cache (CCH)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-CCH-1 | Cache exact (hash normalisé de la requête), mémoire et Redis/Valkey | M | 2 |
| FR-CCH-2 | Cache sémantique : embedding + recherche vectorielle, seuil configurable par route | M | 2 |
| FR-CCH-3 | Index vectoriel embarqué (HNSW) et externes (Qdrant, pgvector, Redis) | S | 2-3 |
| FR-CCH-4 | Cache isolé par clé virtuelle ou partagé, TTL, invalidation par API | M | 2 |
| FR-CCH-5 | Rejeu d'une réponse en cache en streaming (découpage simulé) | S | 2 |

### 4.6 MCP (MCP)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-MCP-1 | Client MCP : stdio, SSE, Streamable HTTP ; découverte et appel des outils | M | 2 |
| FR-MCP-2 | Passerelle MCP : buNtu expose comme un seul serveur MCP les outils de N serveurs | M | 2 |
| FR-MCP-3 | Exécution automatique des outils dans la boucle de complétion (agent mode), avec liste blanche | S | 3 |
| FR-MCP-4 | Serveurs MCP virtuels : sous-ensembles d'outils par clé virtuelle ou rôle | S | 3 |
| FR-MCP-5 | « Code mode » : l'agent écrit un script qui appelle les outils, exécuté en bac à sable WASM | C | 4 |
| FR-MCP-6 | OAuth 2.1 vers les serveurs MCP distants, secrets chiffrés | S | 3 |

### 4.7 Sécurité et identité (SEC)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-SEC-1 | Authentification admin par mot de passe (Argon2id) et jetons API | M | 1 |
| FR-SEC-2 | SSO OIDC (Okta, Entra ID, Google, Keycloak, Auth0, Zitadel) et SAML 2.0 | M (OIDC), S (SAML) | 3 |
| FR-SEC-3 | SCIM 2.0 pour provisionner utilisateurs et groupes | S | 3 |
| FR-SEC-4 | RBAC : rôles prédéfinis + rôles personnalisés, permissions par ressource | M | 3 |
| FR-SEC-5 | Journal d'audit chaîné par hash (inaltérable), export SIEM | M | 3 |
| FR-SEC-6 | Secrets dans Vault, AWS/GCP/Azure Secret Manager, ou chiffrés localement (AES-256-GCM) | M | 3 |
| FR-SEC-7 | Guardrails : détection PII (regex + modèle), masquage, listes de mots, appels à des modèles de modération | S | 3 |

### 4.8 Observabilité et journaux (OBS)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-OBS-1 | Métriques Prometheus (`/metrics`) | M | 1 |
| FR-OBS-2 | Traces OpenTelemetry (OTLP) avec conventions GenAI | M | 2 |
| FR-OBS-3 | Journal des requêtes (métadonnées par défaut, contenu optionnel), écrit hors chemin critique | M | 1 |
| FR-OBS-4 | Export des journaux (S3, fichiers, Kafka, ClickHouse) | S | 3 |

### 4.9 Interface web (UI)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-UI-1 | Configuration des fournisseurs, clés et routes | M | 1 |
| FR-UI-2 | Journal des requêtes filtrable, détail d'une requête | M | 1 |
| FR-UI-3 | Tableaux de bord coûts, latence, erreurs | M | 2 |
| FR-UI-4 | Gestion gouvernance (équipes, budgets, clés virtuelles), MCP, utilisateurs et rôles | M | 2-3 |
| FR-UI-5 | Bac à sable (playground) pour tester un modèle | S | 2 |
| FR-UI-6 | Interface embarquée dans le binaire, désactivable, i18n FR/EN | M | 1 |

### 4.10 Extensibilité et distribution (EXT)

| ID | Exigence | Prio | Phase |
| --- | --- | --- | --- |
| FR-EXT-1 | Plugins natifs Rust via traits `PreHook` / `PostHook` (compilés) | M | 1 |
| FR-EXT-2 | Plugins dynamiques WebAssembly (WASI, composant), isolés, avec quota CPU et mémoire | S | 3 |
| FR-EXT-3 | Clustering : plusieurs nœuds sans état partageant compteurs et configuration | M | 3 |
| FR-EXT-4 | Bibliothèque Rust (`buntu-core`) utilisable sans la passerelle HTTP | S | 2 |
| FR-EXT-5 | SDK légers : TypeScript et Python pour l'API d'administration | C | 4 |

## 5. Exigences non fonctionnelles

Les seuils ci-dessous sont des critères de livraison : une version qui en dégrade un de plus de 10 % est bloquée en CI. Ils se mesurent selon le protocole de la section 11, contre un fournisseur simulé qui répond en 1 s avec un flux de 200 tokens, afin d'isoler le coût propre à la passerelle.

### 5.1 Performance

| ID | Métrique | Machine | Cible | Plancher bloquant |
| --- | --- | --- | --- | --- |
| NFR-PERF-1 | Surcoût ajouté P50 / P99 à 5 000 req/s | 4 vCPU, 8 Go | ≤ 20 µs / ≤ 100 µs | 50 µs / 250 µs |
| NFR-PERF-2 | Surcoût P99,9 | 4 vCPU | ≤ 500 µs | 1 ms |
| NFR-PERF-3 | Débit soutenu sans erreur | 4 vCPU | ≥ 20 000 req/s | 10 000 req/s |
| NFR-PERF-4 | Connexions de streaming simultanées | 4 vCPU, 8 Go | ≥ 50 000 | 20 000 |
| NFR-PERF-5 | Mémoire résidente au repos | — | ≤ 30 Mo | 60 Mo |
| NFR-PERF-6 | Mémoire à 5 000 req/s | 4 vCPU | ≤ 150 Mo | 300 Mo |
| NFR-PERF-7 | Mémoire par flux actif | — | ≤ 16 Ko | 64 Ko |
| NFR-PERF-8 | Délai ajouté au premier token (TTFT) | — | ≤ 50 µs | 200 µs |
| NFR-PERF-9 | Démarrage à froid jusqu'à `/ready` | — | ≤ 200 ms | 1 s |
| NFR-PERF-10 | Recherche cache sémantique (hors embedding), 1 M d'entrées | 4 vCPU | ≤ 2 ms P99 | 10 ms |
| NFR-PERF-11 | Taille du binaire statique (UI incluse) | — | ≤ 40 Mo | 80 Mo |

**Lecture honnête des gains.** La latence vue par l'utilisateur reste dominée par le fournisseur (0,5 à 3 s). buNtu vise surtout une mémoire 2 à 5 fois plus basse et un P99 plus stable qu'une passerelle à ramasse-miettes ; tout chiffre publié sera accompagné de son protocole et de ses scripts.

### 5.2 Fiabilité et disponibilité

- **NFR-REL-1 :** 0 erreur 5xx émise par buNtu lui-même sous charge nominale ; toute erreur renvoyée est attribuée à un fournisseur.
- **NFR-REL-2 :** bascule sur repli en ≤ 1 tentative supplémentaire après erreur récupérable ; disjoncteur ouvert en ≤ 5 s après 50 % d'erreurs sur 20 requêtes.
- **NFR-REL-3 :** arrêt gracieux : les flux en cours se terminent (délai max 30 s) avant l'arrêt.
- **NFR-REL-4 :** rechargement de la configuration à chaud, sans requête perdue.
- **NFR-REL-5 :** en cluster, la perte d'un nœud n'interrompt que ses flux en cours ; les budgets restent cohérents à ± 1 % près.
- **NFR-REL-6 :** aucune panique (`panic`) atteignable par une entrée réseau, vérifié par fuzzing continu.
- **NFR-REL-7 :** endurance 72 h à 2 000 req/s sans croissance mémoire > 5 %.

### 5.3 Qualité, maintenabilité et portabilité

- Couverture de tests ≥ 80 % sur `buntu-core`, ≥ 70 % global.
- `cargo clippy -D warnings`, `rustfmt`, `cargo-deny`, `cargo-audit` bloquants en CI.
- Cibles : Linux x86-64 et ARM64 (musl statique), macOS ARM64, Windows x86-64 (meilleur effort).
- Ajout d'un fournisseur compatible OpenAI : ≤ 1 jour ; fournisseur natif : ≤ 1 semaine, grâce à un kit de tests de conformité partagé.

### 5.4 Utilisabilité

- Premier appel réussi en ≤ 5 minutes depuis `docker run`, sans fichier de configuration.
- Interface conforme WCAG 2.2 AA, en français et en anglais.
- Toute erreur de configuration indique le champ fautif, la valeur attendue et un lien vers la doc.

## 6. Architecture logicielle

buNtu est un monolithe modulaire : un seul binaire sans état, découpé en crates Rust à frontières strictes. Le chemin critique d'une requête ne prend aucun verrou, ne touche aucune base de données et n'alloue que le strict nécessaire ; tout le reste (journaux, coûts, métriques lourdes) part en asynchrone.

### 6.1 Principes de conception

1. **Configuration en instantané immuable.** La config compilée (routes, clés, clés virtuelles) vit dans un `ArcSwap` : lecture sans verrou, remplacement atomique au rechargement.
2. **Format canonique interne.** Toute requête est traduite une fois vers un schéma interne, puis vers le fournisseur. N formats d'entrée × M fournisseurs = N + M traducteurs, pas N × M.
3. **Streaming sans réassemblage.** Les flux SSE sont transformés morceau par morceau (`Bytes` sans copie), jamais tamponnés en entier.
4. **Hors chemin critique par défaut.** Journaux et coûts sont poussés dans un canal borné ; en cas de saturation, on échantillonne au lieu de ralentir les requêtes.
5. **Pression arrière explicite.** Files bornées, limite de connexions, réponse 503 + `retry-after` plutôt qu'un effondrement.
6. **Fonctionnalités compilées à la demande.** Chaque fournisseur et module lourd est une *feature* Cargo ; le binaire par défaut inclut tout, un binaire minimal reste possible.

### 6.2 Découpage en crates

| Crate | Rôle | Dépend de |
| --- | --- | --- |
| `buntu-schema` | Types canoniques (messages, outils, usage, erreurs), sérialisation | — |
| `buntu-core` | Pipeline, routage, réessais, disjoncteurs, sélection de clés, chaîne de hooks | schema |
| `buntu-providers` | Un module par fournisseur : traduction requête / réponse / flux, signature (SigV4, OAuth) | schema |
| `buntu-gateway` | Serveur HTTP, endpoints compatibles, API d'administration, UI embarquée | core, providers, tous les modules |
| `buntu-store` | Stockage config et données (SQLite par défaut, PostgreSQL), migrations | schema |
| `buntu-governance` | Clés virtuelles, hiérarchie, budgets, limites de débit | core, store |
| `buntu-cache` | Cache exact et sémantique, index vectoriel | core |
| `buntu-mcp` | Client et passerelle MCP | core |
| `buntu-security` | Authentification, OIDC/SAML/SCIM, RBAC, audit, secrets, guardrails | store |
| `buntu-observe` | Métriques, traces, journal des requêtes, exports | core |
| `buntu-plugin-wasm` | Hôte de plugins WebAssembly | core |
| `buntu-cluster` | Appartenance au cluster, compteurs partagés, diffusion de config | core, store |
| `buntu-cli` | Binaire `buntu` : `serve`, `import`, `bench`, `doctor`, `config validate` | gateway |

### 6.3 Choix techniques

| Besoin | Choix | Raison | Alternative écartée |
| --- | --- | --- | --- |
| Runtime async | `tokio` multi-thread | Standard de fait, écosystème complet | `monoio`/`glommio` (io\_uring) : à réévaluer en phase 4 |
| Serveur HTTP | `hyper` 1.x + `axum` | HTTP/1.1 et HTTP/2, performant, mature | `actix-web` : modèle d'acteurs inutile ici |
| Client HTTP amont | `hyper-util` avec pool HTTP/2, `rustls` | Contrôle fin du pool et des timeouts | `reqwest` : pratique mais moins de contrôle |
| JSON | `serde_json` ; `sonic-rs` sur le chemin chaud après mesure | Sûr d'abord, SIMD si le profil le justifie | — |
| Allocateur | `mimalloc` | Moins de fragmentation sous charge | `jemalloc` : comparer en bench |
| Base de données | `sqlx` (SQLite, PostgreSQL) | Requêtes vérifiées à la compilation | ORM lourd |
| Cache distribué | `fred` (Redis/Valkey) | Pipelines, cluster, TLS | — |
| Vecteurs embarqués | `usearch` ou HNSW maison | Rapide, sans service externe | — |
| Plugins | `wasmtime` (Component Model) | Isolation, quotas, multi-langages | Chargement de `.so` : dangereux |
| MCP | SDK Rust officiel `rmcp` | Suit la spec | Réimplémenter le protocole |
| Observabilité | `tracing`, `metrics` + export Prometheus, `opentelemetry` | Standard | — |
| Montants | `rust_decimal` | Pas d'erreur d'arrondi sur les coûts | `f64` |
| Limites de débit | GCRA (`governor`) en local, script Lua Redis en cluster | Précis et O(1) | Fenêtre glissante en base |
| UI | SvelteKit en mode statique, embarquée via `rust-embed` | Légère (< 1 Mo gzip), aucun Node au runtime | Next.js : plus lourd |

### 6.4 Parcours d'une requête

Le schéma ci-dessous suit un appel `/v1/chat/completions` de bout en bout ; les étapes en pointillé sont asynchrones et ne retardent jamais la réponse.

&#91;embedded content: parcours d'une requête · 8 étapes synchrones, 3 asynchrones\]

Seul le routage décide ; la traduction et le fournisseur sont interchangeables. Les trois étapes en pointillé ne peuvent jamais ralentir le client : si le canal sature, buNtu échantillonne les journaux au lieu de bloquer.

## 7. Données, stockage et configuration

buNtu démarre sans rien : SQLite embarqué par défaut, PostgreSQL dès qu'il y a plusieurs nœuds. La base n'est lue qu'au démarrage et au rechargement ; en fonctionnement, tout est servi depuis la mémoire.

### 7.1 Trois sources de configuration, un seul modèle

1. **Fichier** `buntu.toml` (ou YAML/JSON), versionnable dans Git, avec références `env:VAR` et `secret:vault/chemin`.
2. **API d'administration** et **interface web**, qui écrivent en base.
3. **Variables d'environnement** `BUNTU_*` pour les surcharges de déploiement.

Priorité : environnement > fichier > base. Un mode `config.mode = "file"` rend la base en lecture seule pour le GitOps. `buntu config validate` vérifie un fichier sans démarrer, `buntu config diff` montre ce qui changerait.

### 7.2 Modèle de données principal

| Entité | Champs clés | Volume attendu | Stockage |
| --- | --- | --- | --- |
| Provider | id, type, base\_url, options, actif | dizaines | base + mémoire |
| ProviderKey | id, provider\_id, secret\_ref, poids, modèles, quotas RPM/TPM | centaines | base + mémoire, secret chiffré |
| Route / Alias | nom, chaîne de repli ordonnée, règles | dizaines | base + mémoire |
| Organisation, BusinessUnit, Team, Customer | id, parent\_id, nom, budget\_id | milliers | base + mémoire |
| VirtualKey | hash SHA-256 de la clé, propriétaire, autorisations, expiration | dizaines de milliers | base + mémoire |
| Budget | portée, montant (décimal), période, politique | milliers | base ; compteurs en mémoire / Redis |
| User, Role, Permission | identité OIDC, rôles | milliers | base |
| McpServer | transport, URL/commande, outils autorisés, secrets | dizaines | base + mémoire |
| RequestLog | id, horodatage, clé virtuelle, modèle, fournisseur, tokens, coût, latences, statut, contenu (option) | millions par jour | SQLite/PostgreSQL, ClickHouse en option |
| AuditEvent | acteur, action, cible, avant/après, hash précédent | milliers par jour | table en ajout seul |

### 7.3 Règles de gestion des données

- **Clés virtuelles :** seule l'empreinte est stockée ; la clé n'est affichée qu'une fois à la création (format `nf-<préfixe>-<32 octets base62>`).
- **Compteurs de budget :** incrémentés en mémoire, vidés en base toutes les secondes ; en cluster, compteurs atomiques Redis. Une perte de nœud coûte au plus 1 s de comptage, ce qui fixe la tolérance de ± 1 %.
- **Journaux :** rétention configurable (30 jours par défaut), purge par partitions journalières, contenu des messages désactivé par défaut.
- **Migrations :** versionnées, en avant uniquement, exécutées au démarrage sous verrou consultatif ; test de montée de version depuis chaque version publiée en CI.
- **Sauvegarde :** `buntu backup` exporte la configuration complète (sans secrets en clair) en un fichier signé.

## 8. Sécurité et conformité

Une passerelle IA détient toutes les clés fournisseurs et voit tous les prompts : c'est une cible de premier rang. La sécurité est traitée dès la phase 1, pas ajoutée à la fin.

### 8.1 Menaces principales (STRIDE simplifié)

| Menace | Exemple | Contre-mesure | Phase |
| --- | --- | --- | --- |
| Vol de clés fournisseurs | Dump de la base, logs verbeux | Chiffrement AES-256-GCM au repos, clé maître hors base (KMS/Vault/fichier 0600), secrets jamais journalisés (type `Secret<T>` sans `Debug`) | 1 |
| Usurpation | Clé virtuelle devinée ou fuitée | 256 bits d'entropie, comparaison en temps constant, révocation immédiate, préfixe détectable par les scanners de secrets GitHub | 1-2 |
| Élévation de privilèges | Un développeur modifie un budget | RBAC vérifié côté serveur sur chaque route d'admin, tests de matrice de permissions | 3 |
| SSRF | Fournisseur personnalisé pointant vers `169.254.169.254` | Liste de blocage des IP privées et métadonnées cloud, configurable | 2 |
| Déni de service | Prompts énormes, connexions lentes | Taille max du corps, timeouts d'en-têtes, limites par clé, files bornées | 1 |
| Injection via outils MCP | Un outil renvoie des instructions malveillantes | Liste blanche d'outils, bac à sable WASM, confirmation pour les outils à effet de bord | 2-3 |
| Répudiation | « Ce n'est pas moi qui ai changé la route » | Audit chaîné par hash, horodaté, export SIEM | 3 |
| Fuite de données personnelles | PII envoyées à un fournisseur hors UE | Guardrails PII, routage par résidence des données | 3 |

### 8.2 Exigences transverses

- TLS 1.2 minimum (1.3 par défaut) via `rustls`, mTLS optionnel entre nœuds et vers l'amont.
- En-têtes de sécurité sur l'UI (CSP stricte, HSTS, `frame-ancestors 'none'`), cookies `HttpOnly` + `SameSite=Strict`, protection CSRF.
- Image conteneur *distroless*, utilisateur non root, système de fichiers en lecture seule.
- SBOM (CycloneDX) et signature des binaires et images (Sigstore/cosign), provenance SLSA niveau 3 via GitHub Actions.
- Politique de divulgation `SECURITY.md`, réponse sous 72 h, correctif critique sous 7 jours.
- Fuzzing continu (`cargo-fuzz`) des parseurs JSON, SSE et des traducteurs de fournisseurs.

### 8.3 Conformité

buNtu est un logiciel, pas un service : la conformité reste de la responsabilité de l'exploitant. Le projet fournit les briques nécessaires :

- **RGPD :** minimisation (contenu non journalisé par défaut), droit à l'effacement par identifiant utilisateur, rétention configurable, routage par région.
- **AI Act européen :** journaux d'usage et traçabilité modèle / version utiles aux obligations des déployeurs.
- **SOC 2 / ISO 27001 :** audit, RBAC, SSO et chiffrement documentés dans un guide « contrôles couverts ».

Ces points sont des aides à la conformité, pas un avis juridique.

## 9. Observabilité

L'observabilité de buNtu doit répondre en moins d'une minute à trois questions : qu'est-ce qui est lent, qu'est-ce qui échoue, combien ça coûte. Son propre coût est plafonné : ≤ 5 % du CPU et ≤ 10 µs par requête avec tout activé.

### 9.1 Métriques Prometheus (extrait)

| Métrique | Type | Labels |
| --- | --- | --- |
| `buntu_requests_total` | compteur | provider, model, route, status, cache |
| `buntu_request_duration_seconds` | histogramme natif | provider, model, stream |
| `buntu_gateway_overhead_seconds` | histogramme | route |
| `buntu_time_to_first_token_seconds` | histogramme | provider, model |
| `buntu_tokens_total` | compteur | provider, model, direction (in/out/cached/reasoning) |
| `buntu_cost_total` | compteur (micro-unités) | provider, model, team |
| `buntu_fallbacks_total` / `buntu_retries_total` | compteur | from, to, reason |
| `buntu_circuit_state` | jauge | provider, key |
| `buntu_budget_usage_ratio` | jauge | scope |
| `buntu_log_queue_dropped_total` | compteur | — |

La cardinalité est bornée : pas de label par utilisateur ni par clé virtuelle (ces vues passent par les journaux).

### 9.2 Traces et journaux

- **Traces OTLP** suivant les conventions sémantiques OpenTelemetry GenAI : un span par requête, un span enfant par tentative amont, propagation `traceparent` entrante et sortante.
- **Journaux applicatifs** `tracing` en JSON structuré, niveau réglable à chaud par module.
- **Journal des requêtes** consultable dans l'UI, filtrable par modèle, statut, clé, coût, latence ; export S3/Kafka/ClickHouse.

### 9.3 Livrables prêts à l'emploi

- Tableaux de bord Grafana (vue d'ensemble, fournisseurs, coûts, cluster) versionnés dans le dépôt.
- Règles d'alerte Prometheus : taux d'erreur par fournisseur > 5 %, disjoncteur ouvert > 5 min, budget > 90 %, file de journaux qui perd des événements.
- `buntu doctor` : diagnostic local (connectivité fournisseurs, horloge, limites système, configuration).

## 10. Matériel, dimensionnement et optimisation bas niveau

Une passerelle IA est limitée par le réseau, le nombre de connexions ouvertes et le coût du TLS et du JSON, rarement par le calcul brut. Le dimensionnement se fait donc en connexions simultanées et en requêtes par seconde, pas en cœurs « au cas où ».

### 10.1 Profils de déploiement de référence

Chiffres cibles à valider par la campagne de benchmark de la phase 1 ; ils seront publiés avec le protocole.

| Profil | Machine | Charge visée | Flux simultanés | Notes |
| --- | --- | --- | --- | --- |
| Edge / homelab | Raspberry Pi 5 (4 cœurs ARM, 4 Go) | 200 req/s | 2 000 | Binaire musl ARM64, SQLite |
| Petite équipe | 2 vCPU, 2 Go | 2 000 req/s | 10 000 | Un nœud, SQLite |
| Production standard | 4 vCPU, 8 Go (x86-64 ou Graviton) | 20 000 req/s | 50 000 | PostgreSQL, Redis optionnel |
| Grande entreprise | 3 à N nœuds de 8 vCPU, 16 Go | 50 000+ req/s par nœud | 100 000+ par nœud | PostgreSQL HA, Redis/Valkey, répartiteur L4 |

### 10.2 Budget mémoire par flux

La mémoire d'un flux actif se décompose ainsi (cible ≤ 16 Ko, NFR-PERF-7) : état de la tâche async ≈ 2 Ko, tampons TLS côté client et amont ≈ 2 × 4 Ko en HTTP/2 partagé, tampon de traduction SSE ≈ 2 Ko, métadonnées de requête et de comptage ≈ 1 Ko. 50 000 flux tiennent donc en environ 800 Mo, hors cache.

### 10.3 Réglages système fournis (Linux)

- **Descripteurs de fichiers :** `LimitNOFILE=1048576` dans l'unité systemd et le chart Helm.
- **Réseau :** `net.core.somaxconn=65535`, `net.ipv4.ip_local_port_range="1024 65535"`, `tcp_tw_reuse=1`, `SO_REUSEPORT` pour répartir l'acceptation entre threads, `TCP_NODELAY` sur toutes les connexions.
- **Connexions amont :** HTTP/2 multiplexé vers chaque fournisseur, pool préchauffé au démarrage, keep-alive et PING HTTP/2 pour détecter les connexions mortes.
- **CPU :** un thread worker tokio par cœur, épinglage optionnel (`core_affinity`) ; sur machines NUMA, une instance par nœud NUMA derrière `SO_REUSEPORT`.
- **Allocateur :** `mimalloc` par défaut, retour mémoire au système après pic, comparaison avec `jemalloc` dans chaque campagne.
- **Conteneurs :** lecture des limites cgroup v2 pour dimensionner les threads et les caches automatiquement.

### 10.4 Optimisations de compilation et de code

- Profil release : `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, puis PGO (profile-guided optimization) sur la charge de benchmark, et BOLT en option ; gain attendu 10 à 20 %, à confirmer par mesure.
- Binaires `x86-64-v3` (AVX2) en plus du binaire générique, sélection automatique au téléchargement.
- Chemin chaud sans allocation : `Bytes` partagés, chaînes interned pour les noms de modèles, `SmallVec` pour les petites listes, aucune `String` formatée par requête hors erreurs.
- Accélération TLS matérielle (AES-NI, ARMv8 Crypto) via `aws-lc-rs` comme backend `rustls`.
- Phase 4, expérimental : runtime io\_uring et kTLS pour réduire les copies noyau, gardé seulement si le gain mesuré dépasse 15 %.

### 10.5 Règle d'or

Aucune optimisation n'entre sans mesure avant/après dans le dépôt (`benches/` + flamegraph). Une optimisation qui complique le code pour moins de 5 % de gain est refusée.

## 11. Tests, benchmarks et méthodologie de comparaison

En solo, les tests remplacent le relecteur : toute fusion sur `main` passe la CI complète, et toute affirmation de performance est reproductible par un tiers avec une seule commande (`buntu bench run <scénario>`).

### 11.1 Pyramide de tests

| Niveau | Outil | Contenu | Quand |
| --- | --- | --- | --- |
| Unitaires | `cargo nextest` | Traducteurs, sélection de clés, budgets, calcul de coût | Chaque commit |
| Propriétés | `proptest` | Aller-retour de traduction sans perte, invariants de budget | Chaque commit |
| Instantanés | `insta` | Requêtes et réponses traduites par fournisseur | Chaque commit |
| Conformité fournisseurs | Kit `buntu-conformance` + serveurs simulés (`wiremock`) | Chaque fournisseur passe la même batterie : texte, flux, outils, vision, erreurs | Chaque commit |
| Intégration | `testcontainers` (PostgreSQL, Redis, Qdrant) | Persistance, cluster, cache | Chaque PR |
| Compatibilité SDK | SDK officiels OpenAI, Anthropic, GenAI en Python et TypeScript | Remplacement direct réel | Chaque PR |
| Contrôle réel | Vrais fournisseurs, petits modèles, budget plafonné (≤ 5 €/mois) | Détection des changements d'API | Chaque nuit |
| Fuzzing | `cargo-fuzz` | Parseurs JSON/SSE, traducteurs | Continu (nuit) |
| Chaos | Injection de pannes (latence, 5xx, coupures TCP) | Repli, disjoncteur, zéro panique | Chaque release |
| Endurance | 72 h à 2 000 req/s | Fuites mémoire, dérive | Chaque release mineure |
| UI | Playwright | Parcours critiques, accessibilité (axe) | Chaque PR touchant l'UI |

### 11.2 Protocole de benchmark

1. **Banc :** deux machines identiques dans la même zone (générateur et passerelle), plus un fournisseur simulé en Rust sur une troisième, pour que la passerelle soit le seul goulot.
2. **Fournisseur simulé :** latence fixe 1 s, flux de 200 tokens à intervalle régulier, payloads réalistes (2 Ko en entrée, 1 Ko en sortie).
3. **Générateur :** `oha` ou `k6` en boucle ouverte (taux constant) pour éviter l'omission coordonnée ; HDR Histogram pour les percentiles.
4. **Mesure du surcoût :** latence via passerelle − latence directe vers le simulateur, sur les mêmes requêtes.
5. **Paliers :** 500, 1 000, 2 000, 5 000, 10 000, 20 000 req/s, 5 min chacun après 1 min de chauffe, 3 répétitions, médiane publiée avec écart.
6. **Relevés :** P50/P90/P99/P99,9, débit, taux de succès, RSS mémoire max, CPU, connexions.
7. **Comparaison :** les autres passerelles open source sont testées sur le même banc, dans leur dernière version, avec leur configuration recommandée ; leurs mainteneurs sont invités à relire la configuration avant publication.
8. **Publication :** scripts, configurations, versions et résultats bruts dans `benchmarks/` ; aucune comparaison publiée sans les données brutes.

### 11.3 Régression continue

Benchmarks micro (`criterion`, suivi par CodSpeed ou Bencher) à chaque PR ; un recul > 10 % sur un chemin chaud bloque la fusion. Campagne macro complète avant chaque release mineure.

## 12. Packaging, déploiement et cycle de release

Tout est automatisé : un tag Git `vX.Y.Z` produit, signe et publie tous les artefacts sans intervention manuelle, ce qui rend le rythme tenable pour une personne seule.

### 12.1 Artefacts publiés

| Canal | Commande utilisateur | Cibles |
| --- | --- | --- |
| Binaires GitHub Releases | `curl -fsSL https://<domaine>/install.sh \| sh` | Linux x86-64/ARM64 (musl), macOS ARM64, Windows x86-64 |
| Image conteneur (GHCR + Docker Hub) | `docker run -p 8080:8080 ghcr.io/<org>/buntu` | linux/amd64, linux/arm64, distroless |
| npm (lanceur) | `npx buntu` | Télécharge le binaire natif vérifié |
| crates.io | `cargo install buntu` / `buntu-core` en bibliothèque | Toutes plateformes Rust |
| Homebrew, Scoop, paquets .deb/.rpm | `brew install buntu` | macOS, Windows, Debian, RHEL |
| Helm chart | `helm install buntu oci://ghcr.io/<org>/charts/buntu` | Kubernetes ≥ 1.28 |
| Terraform / exemples | Modules AWS ECS, GCP Cloud Run, Azure Container Apps | Cloud |

### 12.2 Pipeline de release

1. `cargo-release` monte la version et génère le changelog depuis les commits conventionnels (`git-cliff`).
2. `cargo-dist` compile les binaires (avec PGO), l'image multi-architecture est construite en parallèle.
3. SBOM, signatures cosign et attestation de provenance SLSA sont attachés.
4. Tests de fumée sur chaque artefact publié (démarrage, `/ready`, un appel au simulateur).
5. Publication de la doc versionnée et annonce (GitHub, Discord, réseaux).

### 12.3 Versionnement et support

- SemVer strict ; avant 1.0, une mineure peut casser, avec guide de migration.
- À partir de 1.0 : API HTTP publique et format de configuration stables sur toute la série 1.x ; dépréciation annoncée au moins 2 mineures à l'avance.
- Rythme visé : un patch quand nécessaire, une mineure toutes les 6 à 8 semaines.
- Support : dernière mineure + correctifs de sécurité sur la précédente pendant 3 mois.

### 12.4 Documentation

Site statique (mdBook ou Astro Starlight) en français et en anglais : démarrage en 5 minutes, guides par fournisseur, référence de configuration générée depuis le code, référence OpenAPI, guides de migration depuis d'autres passerelles, recettes de déploiement, page benchmarks.

## 13. Cahier des tâches

Le projet compte environ 395 jours de travail estimés, soit environ 2 ans et 3 mois à 4 jours effectifs par semaine, marge de 20 % incluse. Chaque tâche fait 1 à 12 jours, renvoie aux exigences qu'elle couvre et se termine par un critère d'acceptation vérifiable. Cochez au fil de l'eau.

**Règles de travail en solo.** Une seule tâche en cours à la fois. Une tâche dépassant 150 % de son estimation est redécoupée. Chaque vendredi : 30 min de revue du tableau et mise à jour des estimations. Toute décision structurante fait l'objet d'un ADR (Architecture Decision Record) dans `docs/adr/`.

**Exécution détaillée.** Chaque tâche a une fiche pas à pas, avec les règles à imposer aux agents IA : Guide d'exécution (agents IA)

### Phase 0 — Fondations (15 j, v0.0)

- [ ] **T0.1** Valider le nom : marques EUIPO/USPTO, crates.io, GitHub, npm, Docker Hub, domaine — 1 j — *Acceptation :* nom réservé partout ou nom de rechange choisi.
- [x] **T0.2** Workspace Cargo avec les 13 crates vides, CI (fmt, clippy, nextest, deny, audit, couverture) — 2 j — *Acceptation :* PR d'exemple bloquée par un warning clippy.
- [x] **T0.3** Simulateur de fournisseur en Rust : latence, flux, erreurs 429/5xx, coupures injectables — 3 j — *Acceptation :* tient 50 000 req/s seul sur 4 vCPU.
- [x] **T0.4** Banc de benchmark scripté (section 11.2) + mesure de référence d'une passerelle existante — 3 j — *Acceptation :* rapport reproductible en une commande.
- [x] **T0.5** Schéma canonique v0 : messages, contenus multimodaux, outils, usage, erreurs — 4 j — *Acceptation :* aller-retour OpenAI → canonique → OpenAI sans perte sur 200 cas (`proptest`).
- [x] **T0.6** ADR initiales : runtime, HTTP, stockage, UI, licence — 1 j.
- [x] **T0.7** README, CONTRIBUTING, SECURITY, CODE\_OF\_CONDUCT, LICENSE Apache 2.0, modèles d'issues — 1 j.

### Phase 1 — MVP utilisable en production (95 j, v0.1 → v0.5)

**E1 — Moteur (21 j)**

- [x] **T1.1** Serveur hyper/axum, `/health`, `/ready`, arrêt gracieux, limites de taille et de connexions — 3 j — FR-API-10, NFR-REL-3.
- [x] **T1.2** Configuration fichier + environnement, instantané `ArcSwap`, rechargement à chaud, `config validate` — 4 j — NFR-REL-4 — *Acceptation :* rechargement sous 5 000 req/s sans erreur.
- [x] **T1.3** Pipeline de requête et chaîne de hooks `PreHook`/`PostHook` — 4 j — FR-EXT-1.
- [x] **T1.4** Sélection pondérée des clés (alias O(1)) — 2 j — FR-RTE-1 — *Acceptation :* < 50 ns par sélection (criterion).
- [x] **T1.5** Réessais avec backoff + gigue, budget global de réessais — 3 j — FR-RTE-3.
- [x] **T1.6** Chaîne de repli et alias de modèles — 3 j — FR-RTE-2, FR-API-7 — *Acceptation :* scénario chaos UC2 sans erreur client.
- [x] **T1.7** Quatre timeouts (connexion, premier token, inter-tokens, total) — 2 j — FR-RTE-4.

**E2 — Fournisseurs (22 j)**

- [x] **T1.8** Kit de conformité partagé (texte, flux, outils, vision, JSON structuré, erreurs) — 4 j — FR-PRV-4.
- [x] **T1.9** OpenAI : chat, embeddings, outils, vision, flux — 5 j — *Acceptation :* kit 100 % vert + contrôle réel nocturne.
- [x] **T1.10** Anthropic : messages, outils, vision, cache de prompt, réflexion étendue — 5 j.
- [x] **T1.11** Google Gemini (AI Studio) — 5 j.
- [x] **T1.12** Compatible OpenAI générique + Ollama — 3 j — *Acceptation :* vLLM, Groq, OpenRouter et Ollama passent le kit.

**E3 — API publique et d'administration (14 j)**

- [x] **T1.13** `/v1/chat/completions`, `/v1/embeddings`, `/v1/models` — 4 j — FR-API-1, 2, 6.
- [x] **T1.14** Point d'entrée natif `/anthropic/v1/messages` — 3 j — FR-API-4 — *Acceptation :* SDK Anthropic officiel fonctionne en changeant l'URL.
- [x] **T1.15** Erreurs normalisées + en-têtes `x-buntu-*` — 2 j — FR-API-8.
- [x] **T1.16** API d'administration REST v1 + OpenAPI généré (`utoipa`) — 5 j — FR-API-9.

**E4 — Stockage, journaux, sécurité de base (14 j)**

- [x] **T1.17** `buntu-store` SQLite + migrations versionnées — 4 j.
- [x] **T1.18** Journal des requêtes asynchrone (canal borné, échantillonnage) — 3 j — FR-OBS-3 — *Acceptation :* surcoût ≤ 5 µs, aucune requête ralentie si la base est lente.
- [x] **T1.19** Métriques Prometheus — 2 j — FR-OBS-1.
- [x] **T1.20** Authentification admin Argon2id + jetons API — 3 j — FR-SEC-1.
- [x] **T1.21** Chiffrement des secrets au repos, type `Secret<T>` — 2 j.

**E5 — Interface web (12 j)**

- [ ] **T1.22** Squelette SvelteKit statique embarqué, i18n FR/EN, thème clair/sombre — 3 j — FR-UI-6.
- [ ] **T1.23** Pages fournisseurs, clés, routes et alias — 5 j — FR-UI-1.
- [ ] **T1.24** Journal des requêtes filtrable + détail — 4 j — FR-UI-2.

**E6 — Livraison (12 j)**

- [ ] **T1.25** Release automatisée : `cargo-dist`, image Docker multi-arch, lanceur `npx`, signatures — 4 j.
- [ ] **T1.26** Première campagne de benchmark publiée — 4 j — NFR-PERF-1 à 9.
- [ ] **T1.27** Documentation : démarrage en 5 min, guides des 5 fournisseurs, référence config — 4 j — *Acceptation :* une personne extérieure réussit UC1 en < 5 min.

### Phase 2 — Gouvernance, cache, MCP et v1.0 (124 j, v0.6 → v1.0)

**E7 — Fournisseurs étendus et API Responses (30 j)**

- [ ] **T2.1** Azure OpenAI (déploiements, versions d'API, Entra ID) — 3 j.
- [ ] **T2.2** AWS Bedrock : signature SigV4, Converse API, flux binaire *event-stream* — 6 j.
- [ ] **T2.3** Google Vertex AI : OAuth compte de service, régions — 4 j.
- [ ] **T2.4** Mistral — 2 j ; **T2.5** Cohere — 3 j.
- [ ] **T2.6** Catalogue de modèles (prix, contexte, capacités) : fichier distant signé + surcharge locale — 4 j — FR-PRV-5.
- [ ] **T2.7** Fournisseur personnalisé déclaratif sans code — 3 j — FR-PRV-6.
- [ ] **T2.8** `/v1/responses` (API Responses) — 5 j — FR-API-3.

**E8 — Fiabilité avancée (6 j)**

- [ ] **T2.9** Disjoncteur par clé et par fournisseur, demi-ouverture — 3 j — FR-RTE-5, NFR-REL-2.
- [ ] **T2.10** Respect de `retry-after`, quotas RPM/TPM par clé — 3 j — FR-RTE-9 — *Acceptation :* scénario UC3 vert.

**E9 — Gouvernance et coûts (22 j)**

- [ ] **T2.11** Hiérarchie organisation → clé virtuelle, clés hachées, révocation immédiate — 6 j — FR-GOV-1, 2.
- [ ] **T2.12** Budgets par niveau et période, blocage dur/souple — 5 j — FR-GOV-3 — *Acceptation :* UC4 vert, écart de comptage ≤ 1 %.
- [ ] **T2.13** Limites RPM/TPM GCRA par niveau — 3 j — FR-GOV-4.
- [ ] **T2.14** Calcul du coût en décimal (entrée, sortie, cache, raisonnement) — 3 j — FR-GOV-5.
- [ ] **T2.15** Alertes 50/80/100 % (webhook, e-mail, Slack) — 3 j — FR-GOV-6.
- [ ] **T2.16** Rapports et export CSV — 2 j — FR-GOV-7.

**E10 — Cache (15 j)**

- [ ] **T2.17** Cache exact mémoire + Redis/Valkey — 3 j — FR-CCH-1.
- [ ] **T2.18** Cache sémantique + index HNSW embarqué — 6 j — FR-CCH-2 — *Acceptation :* NFR-PERF-10 tenu sur 1 M d'entrées.
- [ ] **T2.19** Index externes Qdrant et pgvector — 4 j — FR-CCH-3.
- [ ] **T2.20** Rejeu en flux d'une réponse en cache, isolation par clé, TTL, invalidation — 2 j — FR-CCH-4, 5.

**E11 — MCP (12 j)**

- [ ] **T2.21** Client MCP (stdio, SSE, Streamable HTTP) sur `rmcp` — 6 j — FR-MCP-1.
- [ ] **T2.22** Passerelle MCP : agrégation de N serveurs en un — 6 j — FR-MCP-2 — *Acceptation :* UC6 vert avec 3 serveurs de référence.

**E12 — Plateforme (12 j)**

- [ ] **T2.23** Traces OTLP (conventions GenAI) — 3 j — FR-OBS-2.
- [ ] **T2.24** Support PostgreSQL — 4 j.
- [ ] **T2.25** Protection SSRF — 2 j.
- [ ] **T2.26** `buntu-core` publié comme bibliothèque avec exemples — 3 j — FR-EXT-4.

**E13 — Interface web (17 j)**

- [ ] **T2.27** Tableaux de bord coûts, latence, erreurs — 5 j — FR-UI-3.
- [ ] **T2.28** Écrans gouvernance (équipes, budgets, clés virtuelles) — 6 j — FR-UI-4.
- [ ] **T2.29** Playground — 3 j — FR-UI-5 ; **T2.30** Écrans MCP — 3 j.

**E14 — Durcissement et v1.0 (10 j)**

- [ ] **T2.31** Revue de sécurité complète + fuzzing longue durée + correction — 4 j — NFR-REL-6.
- [ ] **T2.32** Test d'endurance 72 h — 2 j — NFR-REL-7.
- [ ] **T2.33** Gel de l'API et de la config, guides de migration depuis d'autres passerelles — 4 j — *Acceptation :* v1.0 publiée avec benchmarks à jour.

### Phase 3 — Entreprise gratuite (125 j, v1.1 → v1.6)

**E15 — Identité et accès (29 j)**

- [ ] **T3.1** SSO OIDC + recettes Okta, Entra ID, Google, Keycloak, Auth0, Zitadel — 6 j — FR-SEC-2.
- [ ] **T3.2** SAML 2.0 — 6 j.
- [ ] **T3.3** SCIM 2.0 — 5 j — FR-SEC-3.
- [ ] **T3.4** RBAC (rôles prédéfinis + personnalisés) + matrice de tests — 8 j — FR-SEC-4.
- [ ] **T3.5** Écrans utilisateurs, rôles, audit — 4 j — FR-UI-4.

**E16 — Audit, secrets, guardrails (18 j)**

- [ ] **T3.6** Journal d'audit chaîné par hash + export SIEM — 4 j — FR-SEC-5 — *Acceptation :* toute altération d'une ligne est détectée par `buntu audit verify`.
- [ ] **T3.7** Vault, AWS/GCP/Azure Secret Manager — 6 j — FR-SEC-6.
- [ ] **T3.8** Guardrails : PII (regex + modèle), masquage, listes, modération externe — 8 j — FR-SEC-7.

**E17 — Échelle et routage (32 j)**

- [ ] **T3.9** Clustering : appartenance, compteurs partagés Redis, diffusion de configuration — 12 j — FR-EXT-3, NFR-REL-5.
- [ ] **T3.10** Répartition adaptative EWMA — 5 j — FR-RTE-6.
- [ ] **T3.11** Règles de routage déclaratives — 5 j — FR-RTE-7.
- [ ] **T3.12** Plugins WebAssembly (Component Model, quotas) — 10 j — FR-EXT-2.

**E18 — MCP avancé et exports (19 j)**

- [ ] **T3.13** Exécution automatique des outils (agent mode) — 6 j — FR-MCP-3.
- [ ] **T3.14** Serveurs MCP virtuels — 4 j — FR-MCP-4 ; **T3.15** OAuth 2.1 MCP — 4 j — FR-MCP-6.
- [ ] **T3.16** Export des journaux S3, Kafka, ClickHouse — 5 j — FR-OBS-4.

**E19 — Couverture fonctionnelle (27 j)**

- [ ] **T3.17** Images, audio, fichiers, batch — 10 j — FR-API-5.
- [ ] **T3.18** Dix fournisseurs supplémentaires (xAI, DeepSeek, Cerebras, Fireworks, Perplexity, Hugging Face, Replicate, Cloudflare, Databricks, SageMaker) — 15 j — FR-PRV-3.
- [ ] **T3.19** Points d'entrée natifs GenAI et Bedrock — 2 j — FR-API-4.

### Phase 4 — Écosystème (35 j, v2.x, ordre libre)

- [ ] **T4.1** Requêtes hedged — 3 j — FR-RTE-8.
- [ ] **T4.2** MCP « code mode » en bac à sable WASM — 10 j — FR-MCP-5.
- [ ] **T4.3** Expérimentation io\_uring + kTLS, gardée si gain ≥ 15 % — 8 j.
- [ ] **T4.4** SDK d'administration TypeScript et Python — 8 j — FR-EXT-5.
- [ ] **T4.5** Outil `buntu import` depuis d'autres passerelles — 6 j — UC8.

## 14. Planning et jalons

En démarrant le Nov 9, 2026, la v0.5 (MVP) sort fin juin 2027, la v1.0 en mars 2028 et les fonctions entreprise gratuites fin 2028. Ces dates supposent 4 jours effectifs par semaine et incluent 20 % de marge ; elles se recalent chaque mois sur la vélocité réelle.

&#91;embedded content: feuille de route · 5 phases, 5 versions\]

La v0.1 est une alpha publique (OpenAI, Anthropic, compatibles OpenAI) destinée à recueillir des retours tôt ; la v1.0 est le premier engagement de stabilité.

### Rythme de travail

- **Cycles de 2 semaines :** un objectif par cycle, une démo enregistrée de 3 minutes à la fin (utile aussi pour la communauté).
- **Une semaine sur 8 sans fonctionnalité :** dette technique, documentation, dépendances, issues.
- **Revue mensuelle :** jours réels / jours estimés par epic ; si le ratio dépasse 1,3 sur deux mois, la phase en cours est re-scopée (on retire du S/C, jamais du M).
- **Garde-fous de santé :** pas plus de 35 h par semaine, 4 semaines de congés par an déjà couvertes par la marge.

### Accélérateurs possibles

- Ouvrir le dépôt dès la v0.1 avec des issues « good first issue » sur les fournisseurs compatibles OpenAI : chaque contributeur régulier peut faire gagner 10 à 20 % sur les phases 3 et 4.
- Utiliser des assistants de code pour les traducteurs de fournisseurs, qui sont répétitifs et couverts par le kit de conformité.

## 15. Risques et mitigations

Le risque dominant n'est pas technique : c'est l'épuisement d'un développeur seul face à un périmètre de plus de deux ans et à 30 API de fournisseurs qui changent sans prévenir. Les mitigations visent d'abord à livrer tôt et à attirer des contributeurs.

| ID | Risque | Probabilité | Impact | Mitigation | Signal d'alerte |
| --- | --- | --- | --- | --- | --- |
| R1 | Épuisement, abandon du projet | Haute | Critique | Phases livrables seules, cycles de 2 semaines, plafond de 35 h, communauté dès v0.1 | 3 cycles d'affilée sans démo |
| R2 | Les API des fournisseurs changent | Certaine | Élevé | Kit de conformité, contrôles réels nocturnes, traducteurs isolés, fournisseurs « maintenus par la communauté » | Contrôle nocturne rouge > 48 h |
| R3 | Dérive du périmètre (vouloir tout faire avant la v1.0) | Haute | Élevé | MoSCoW strict, seules les exigences M bloquent une phase | Phase en retard > 30 % |
| R4 | Objectifs de performance non tenus | Moyenne | Moyen | Banc dès la phase 0, mesure à chaque PR, objectifs réalistes (pas de « 50x ») | Régression > 10 % non expliquée |
| R5 | Conflit de nom ou de marque | Faible | Élevé | T0.1 avant toute publication, nom de rechange prêt | Courrier ou réclamation |
| R6 | Accusation de copie d'un projet tiers | Faible | Élevé | Réécriture indépendante, aucune copie de code ou de doc, ADR traçant les choix | Issue publique sur le sujet |
| R7 | Faille de sécurité (fuite de clés) | Moyenne | Critique | Section 8, fuzzing, `SECURITY.md`, chiffrement dès la phase 1 | Rapport de vulnérabilité |
| R8 | Complexité du Rust async (durées de vie, `Send`, annulation) | Moyenne | Moyen | Modèles simples (état via `Arc`, canaux), revue des points d'annulation, prototypes courts en phase 0 | Tâche > 150 % de l'estimation |
| R9 | Faible adoption face à des projets installés | Moyenne | Élevé | Différenciation claire (tout gratuit + sobriété), migration en une commande, benchmarks ouverts | < 100 étoiles 3 mois après v0.5 |
| R10 | Coûts des tests réels et des benchmarks | Faible | Faible | Petits modèles, plafonds de dépense, simulateur pour 99 % des tests | Dépense > 25 €/mois |
| R11 | Financement nul sur la durée | Moyenne | Moyen | GitHub Sponsors, Open Collective, subventions (NLnet, Sovereign Tech Fund), sans jamais réserver de fonctionnalité | — |

## 16. Gouvernance open source, glossaire et annexes

### 16.1 Engagement « tout gratuit »

La promesse est écrite dans `GOVERNANCE.md` : aucune fonctionnalité ne sera jamais réservée à une version payante, ni conditionnée à une clé de licence. Le financement passe par les dons, les subventions et le support ou le conseil proposés à côté du logiciel, jamais par le code.

### 16.2 Contribution et décision

- **Modèle :** dictateur bienveillant (le fondateur) jusqu'à 3 mainteneurs réguliers, puis comité de mainteneurs avec vote à la majorité.
- **DCO** (`Signed-off-by`) plutôt qu'un CLA : plus simple, et garantit qu'aucune relicence propriétaire n'est possible sans l'accord de tous.
- **RFC** dans `rfcs/` pour toute modification d'API publique ou de format de configuration.
- **Commits conventionnels**, PR petites, CI verte obligatoire, revue par un mainteneur dès qu'il y en a deux.
- **Feuille de route publique** dans GitHub Projects, alignée sur ce cahier des tâches.

### 16.3 Glossaire

| Terme | Définition |
| --- | --- |
| Passerelle IA | Service placé entre les applications et les fournisseurs de modèles, qui unifie l'accès et ajoute fiabilité et contrôle |
| Fournisseur | Service qui héberge des modèles (OpenAI, Anthropic, Bedrock…) |
| Repli (fallback) | Bascule automatique vers un autre fournisseur ou modèle en cas d'échec |
| Disjoncteur | Mécanisme qui cesse temporairement d'appeler une cible défaillante |
| Clé virtuelle | Clé émise par buNtu, liée à des droits et un budget, qui masque les vraies clés fournisseurs |
| Cache sémantique | Cache qui répond à une question proche par le sens, via des embeddings |
| MCP | Model Context Protocol, protocole standard pour exposer des outils aux modèles |
| Surcoût (overhead) | Latence ajoutée par la passerelle elle-même, hors temps du fournisseur |
| P99 | Latence sous laquelle se trouvent 99 % des requêtes |
| TTFT | Time To First Token, délai avant le premier morceau de réponse |
| RPM / TPM | Requêtes / tokens par minute autorisés |
| GCRA | Algorithme de limitation de débit précis en O(1) |
| ADR | Architecture Decision Record, fiche courte qui trace une décision et ses raisons |
| MoSCoW | Priorisation Must / Should / Could / Won't |
| PGO | Optimisation guidée par profil d'exécution réel |

### 16.4 Annexes

- **Annexe A — Exemple de configuration minimale.**

```toml
[server]
listen = "0.0.0.0:8080"

[[providers]]
id = "openai"
type = "openai"
keys = [{ secret = "env:OPENAI_API_KEY", weight = 1 }]

[[providers]]
id = "anthropic"
type = "anthropic"
keys = [{ secret = "env:ANTHROPIC_API_KEY", weight = 1 }]

[[routes]]
alias = "chat-default"
fallbacks = ["openai/gpt-4o-mini", "anthropic/claude-haiku"]
```

- **Annexe B — Modèle d'ADR :** contexte, décision, alternatives écartées, conséquences, date, statut.
- **Annexe C — Questions ouvertes :**
  - Nom définitif après la recherche de marques (T0.1).
  - SvelteKit ou Leptos (UI en Rust) : à trancher par un prototype d'une journée en phase 0.
  - Faut-il un mode « bibliothèque C/FFI » pour Python et Node, ou les SDK HTTP suffisent-ils ?
