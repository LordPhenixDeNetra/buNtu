# Référence de configuration

> Fichier **généré** : ne pas éditer à la main. Source : `buntu config schema`
> (`schemars` depuis les types Rust), rendu par `node scripts/config-reference.mjs`.
> La CI échoue si ce fichier n'est pas régénéré.

### `Configuration complète`



| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `config` | RawConfigSection | non | `{"mode":"merged"}` | Rôle de la base face au fichier (cahier §7.1). |
| `hooks` | RawHooks | non | `{"post":[],"pre":[]}` | Hooks exécutés avant et après l'appel fournisseur. |
| `providers` | liste de RawProvider | non | `[]` | Fournisseurs de modèles configurés. |
| `routes` | liste de RawRoute | non | `[]` | Alias de modèles et chaînes de repli. |
| `server` | RawServer | non | `{"listen":"127.0.0.1:8080"}` | Paramètres du serveur HTTP. |
| `timeouts` | RawTimeouts | non | `{"connect_ms":null,"first_byte_ms":null,"inter_chunk_ms":null,"total_ms":null}` | Timeouts par défaut, en millisecondes. |

### `ConfigMode`

Mode de configuration (cahier §7.1).

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |

### `RawConfigSection`

Section `[config]`.

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `mode` | ConfigMode | non | `"merged"` | `merged` (défaut) : base + fichier + environnement, l'API d'administration écrit en
base ; `file` : la base est en lecture seule (GitOps). |

### `RawHooks`

Hooks déclarés, dans l'ordre d'exécution.

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `post` | liste de chaîne | non | `[]` | Hooks exécutés après l'appel fournisseur. |
| `pre` | liste de chaîne | non | `[]` | Hooks exécutés avant l'appel fournisseur. |

### `RawKey`

Une clé API de fournisseur.

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `secret` | chaîne | oui | — | Référence `env:VAR` (recommandé en production) ou valeur littérale (tests). |
| `weight` | entier | non | `1` | Poids de sélection ; une clé plus lourde est choisie plus souvent. |

### `RawProvider`

Un fournisseur de modèles.

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `base_url` | string | null | non | `null` | URL de base de l'API du fournisseur (ex. `http://127.0.0.1:9000`). |
| `id` | chaîne | oui | — | Identifiant unique, utilisé dans les cibles (`openai/gpt-4o`). |
| `keys` | liste de RawKey | non | `[]` | Clés API, sélectionnées par poids décroissant. |
| `type` | chaîne | oui | — | Type de fournisseur : `openai`, `anthropic`, `gemini`, `compatible`, `mock`… |

### `RawRoute`

Un alias de modèle et sa chaîne de repli.

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `alias` | chaîne | oui | — | Nom exposé par l'API (`model: "chat"`). |
| `fallbacks` | liste de chaîne | non | `[]` | Cibles essayées dans l'ordre : `fournisseur/modèle`. |
| `timeouts` | RawTimeouts | non | `{"connect_ms":null,"first_byte_ms":null,"inter_chunk_ms":null,"total_ms":null}` | Surcharges de timeouts pour cet alias. |

### `RawServer`

Paramètres du serveur HTTP.

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `listen` | chaîne | non | `"127.0.0.1:8080"` | Adresse d'écoute (`127.0.0.1:8080` par défaut : boucle locale seulement). |

### `RawTimeouts`

Surcharges de timeouts, en millisecondes ; `None` = valeur héritée.

| Champ | Type | Requis | Défaut | Description |
| --- | --- | --- | --- | --- |
| `connect_ms` | integer | null | non | `null` | Délai maximal d'établissement de la connexion amont. |
| `first_byte_ms` | integer | null | non | `null` | Délai maximal avant le premier octet de la réponse amont. |
| `inter_chunk_ms` | integer | null | non | `null` | Délai maximal entre deux morceaux d'un flux amont. |
| `total_ms` | integer | null | non | `null` | Délai maximal total d'une requête. |
