# Erreurs et dépannage (T1.27)

## Format des erreurs HTTP

Toute erreur de la passerelle a la forme OpenAI :

```json
{ "error": { "message": "…", "type": "invalid_request_error", "code": "invalid_request" } }
```

- `code` est **stable** : c'est ce que les clients doivent tester.
- `x-buntu-error-source` indique l'origine : `client`, `provider`, `budget` ou `gateway`.
- `retry-after` est présent quand la passerelle connaît le délai de reprise.

| Code | HTTP | `error.type` | Source | Cause typique | Action |
| --- | --- | --- | --- | --- | --- |
| `invalid_request` | 400 | `invalid_request_error` | client | JSON invalide, modèle inconnu, alias absent | Vérifier `model`, le corps et la configuration des routes |
| `authentication` | 401 | `authentication_error` | client | Clé virtuelle absente/invalide (admin, T1.20) | Envoyer le bon en-tête d'authentification |
| `budget_exceeded` | 402 | `insufficient_quota` | budget | Budget de clé virtuelle épuisé (T2.12) | Recharger le budget ou changer de clé |
| `provider_rate_limited` | 429 | `rate_limit_error` | provider | Quota fournisseur atteint | Respecter `retry-after` ; ajouter une clé/route de repli |
| `provider_unavailable` | 502 | `api_error` | provider | Fournisseur en panne, connexion refusée | Vérifier `base_url` et l'état du fournisseur |
| `timeout` | 504 | `api_error` | provider | Premier octet ou durée totale dépassés | Augmenter `timeouts.*` ou changer de cible |
| `all_targets_failed` | 502 | `api_error` | provider | Toutes les cibles de l'alias ont échoué | Lire le message (tentatives détaillées) et corriger les cibles |
| `internal_error` | 500 | `server_error` | gateway | Bug interne, journal indisponible | Consulter les journaux du processus et `docs/status.md` |

## Erreurs de configuration (`buntu config validate`)

| Message | Cause | Correction |
| --- | --- | --- |
| `variable d'environnement 'X' absente` | clé déclarée `env:X` sans variable | exporter `X` ou changer la référence |
| `poids doit être supérieur à zéro` | `weight = 0` | utiliser un poids ≥ 1 |
| `aucune cible pour l'alias` | route sans `fallbacks`, ou fallback mal formé | format `fournisseur/modèle`, fournisseur déclaré |
| `fournisseur inconnu` | fallback vers un `id` absent de `[[providers]]` | corriger l'identifiant |
| `base_url manquante` | fournisseur sans `base_url` au moment de servir | ajouter `base_url` (http/https selon le fournisseur) |
| `TOML invalide` | syntaxe du fichier | corriger le TOML signalé avec sa ligne |

## Dépannage rapide

- `curl /health` échoue → le processus n'écoute pas sur l'adresse attendue (`--listen`).
- `curl /ready` renvoie 503 → configuration non chargée ; vérifier les logs `buNtu: …`.
- Une requête est lente → vérifier `x-buntu-provider`/`x-buntu-attempts` puis les timeouts.
- Le journal est vide → `BUNTU_DB_PATH` doit pointer vers un répertoire accessible en écriture.
- `npm run build` échoue dans `ui/` → utiliser Node ≥ 20 et `npm ci` depuis `ui/`.
