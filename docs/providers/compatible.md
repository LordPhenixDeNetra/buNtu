# Fournisseurs compatibles OpenAI (vLLM, Groq, OpenRouter, Ollama) (T1.12)

Sources consultées le **2026-11-04** :

- Compatibilité OpenAI d'Ollama : <https://docs.ollama.com/api/openai-compatibility>

Ces serveurs exposent `POST /v1/chat/completions` au format OpenAI Chat Completions. La
traduction utilisée est donc **exactement** celle de `buntu-providers::openai` ; le fournisseur
compatible ne fait que fixer le chemin et les divergences connues.

## Paramétrage (fiche T1.12)

Le fournisseur est paramétré, pas dupliqué :

- `path` : chemin de l'endpoint (`/v1/chat/completions` par défaut) ;
- `base_url` : URL absolue optionnelle (ex. `https://api.groq.com/openai`) ; sans elle, le client
  amont joint l'URL de base configurée ;
- `headers` : en-têtes supplémentaires (ex. OpenRouter `HTTP-Referer`, `X-Title`) ;
- `quirks` **déclarés** : `stream_usage` (omet `stream_options` si le serveur le refuse) et
  `parallel_tools` (envoie `parallel_tool_calls: false` si le serveur ne gère pas les appels
  d'outils parallèles).

## Préréglages

| Préréglage | Chemin | Quirks |
| --- | --- | --- |
| `Compatible::Vllm` | `/v1/chat/completions` | défaut |
| `Compatible::Groq` | `/v1/chat/completions` | défaut |
| `Compatible::OpenRouter` | `/v1/chat/completions` | défaut |
| `Compatible::Ollama` | `/v1/chat/completions` | `stream_usage = false` |

`OpenAiCompatibleProvider::new(path)` accepte un chemin arbitraire ; le client amont
(hyper-util, ADR-0004) portera l'URL de base et la clé.

## Correspondance

Celle d'OpenAI : voir [`openai.md`](openai.md). Les cas de conformité rejouent les mêmes
fixtures (`tests/fixtures/openai/`) avec le fournisseur compatible.

## Limites actuelles (assumées)

- L'API **native** d'Ollama (`/api/chat`, NDJSON) n'est pas traduite : on passe par sa couche de
  compatibilité OpenAI. À ajouter seulement si un besoin réel apparaît.
- Les divergences de champs optionnels (`max_completion_tokens`, `stream_options`,
  `response_format` selon les serveurs) ne sont pas détectées dynamiquement ; seul
  `stream_options` est paramétrable pour l'instant.
- Acceptation de la fiche (vLLM, Groq, OpenRouter, Ollama passent le kit) : **réserve**, faute
  d'endpoints réels dans ce lot ; le kit est vert contre les fixtures synthétiques.
