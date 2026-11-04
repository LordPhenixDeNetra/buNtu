# Ollama (local) — guide fournisseur (T1.27)

Ollama expose une API compatible OpenAI ; buNtu utilise le traducteur **compatible OpenAI** avec
le préréglage `ollama`.

## Configuration

```toml
[[providers]]
id = "ollama"
type = "ollama"
base_url = "http://127.0.0.1:11434"
keys = [{ secret = "ollama", weight = 1 }]

[[routes]]
alias = "local"
fallbacks = ["ollama/qwen3:8b"]
```

- `type = "ollama"` sélectionne le préréglage (pas de `stream_options`, outils parallèles
  désactivés).
- Aucune vraie clé n'est nécessaire (Ollama ignore `Authorization`) ; la clé ci-dessus ne sert
  qu'à satisfaire le format.
- `base_url` est l'adresse du serveur Ollama, sans `/v1` (buNtu ajoute `/v1/chat/completions`).

## Vérifier

```sh
ollama serve &
ollama pull qwen3:8b
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"local","messages":[{"role":"user","content":"bonjour"}]}'
```

## Limites connues

- Les modèles locaux varient en support des outils (`tools`) et des images ; le catalogue de
  modèles (T2.6) affinera les capacités déclarées.
- Le streaming fonctionne sans `stream_options.include_usage` : l'usage peut être absent du flux.
- Les timeouts par défaut (premier octet 60 s, total 10 min) conviennent aux modèles locaux ; ils
  restent surchargeables via `timeouts = { first_byte_ms = …, total_ms = … }` sur le fournisseur
  ou la route.
