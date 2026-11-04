# OpenAI — correspondance canonique ↔ API (T1.9)

Sources officielles consultées le **2026-11-03** :

- OpenAPI officiel, version 2.3.0 : <https://raw.githubusercontent.com/openai/openai-openapi/master/openapi.yaml>
- Référence de l'API Chat Completions : <https://platform.openai.com/docs/api-reference/chat/create>
- Guide du streaming : <https://platform.openai.com/docs/api-reference/chat/streaming>

Endpoint utilisé : `POST /v1/chat/completions`.

## Requête

| Canonique (`buntu-schema`) | OpenAI | Remarque |
| --- | --- | --- |
| `model` | `model` | — |
| `messages[].role` | `messages[].role` | `system`, `developer`, `user`, `assistant`, `tool` (cible : `system` utile) |
| `messages[].content` (un seul `Text`) | `content` (chaîne) | — |
| `messages[].content` (multi-parties) | `content` (tableau `text` / `image_url`) | image `Url` ou `Base64` → `data:<type>;base64,<données>` ; `File` refusé explicitement |
| `ContentPart::ToolCall` | `message.tool_calls[]` | `arguments` re-sérialisés en chaîne JSON |
| `ContentPart::ToolResult` | message séparé `{"role":"tool","tool_call_id":…}` | un message canonique `tool` peut en produire plusieurs |
| `tools[]` | `tools[]` (`type: function`) | `parameters` passés tels quels |
| `tool_choice` | `auto` / `none` / `required` / objet `function` | — |
| `max_tokens` | `max_tokens` | la variante `max_completion_tokens` sera choisie par le catalogue (T2.6) |
| `temperature`, `top_p`, `stop` | mêmes champs | — |
| `stream` | `stream` + `stream_options.include_usage = true` | l'usage arrive dans le dernier chunk |
| `response_format` | `text` / `json_object` / `json_schema` | — |

## Réponse

| OpenAI | Canonique | Remarque |
| --- | --- | --- |
| `choices[].message.content` (chaîne ou `null`) | `ContentPart::Text` | `null` = pas de texte |
| `choices[].message.tool_calls[]` | `ContentPart::ToolCall` | `arguments` (chaîne JSON) décodés en `Value` |
| `choices[].message.refusal` | `ContentPart::Refusal` | — |
| `choices[].finish_reason` | `Response.stop_reason` | `stop`→`EndTurn`, `length`→`MaxTokens`, `tool_calls`→`ToolCalls`, `content_filter`→`ContentFilter`, autre→`Other` |
| `usage.prompt_tokens` | `usage.input_tokens` | — |
| `usage.completion_tokens` | `usage.output_tokens` | — |
| `usage.prompt_tokens_details.cached_tokens` | `usage.cached_input_tokens` | — |
| `usage.completion_tokens_details.reasoning_tokens` | `usage.reasoning_tokens` | — |

## Flux (SSE)

`data: {chunk}` est traduit en événements canoniques :

| OpenAI (`chat.completion.chunk`) | Canonique |
| --- | --- |
| premier chunk (`id`, `model`) | `Start { id, model }` (une seule fois) |
| `choices[].delta.content` | `ContentDelta { index, part: Text }` |
| `choices[].delta.tool_calls[]` | `ToolCallDelta { index, id, name, arguments_delta }` (fragments recollés par `index` en T1.13) |
| `choices[].finish_reason` | `Stop { reason }` |
| `usage` (chunk final avec `include_usage`) | `Usage { usage }` |
| `data: [DONE]` | `Done` (fin de flux) |
| frame `{"error": …}` | erreur `ProviderUnavailable` (le flux est déjà commencé : pas de réessai) |

## Erreurs

| Statut HTTP | Canonique |
| --- | --- |
| 400, 404, 422 | `InvalidRequest { message }` |
| 401, 403 | `Authentication` |
| 429 | `ProviderRateLimited { retry_after }` (en-tête `retry-after`) |
| 500, 502, 503, 504 | `ProviderUnavailable { message }` |

## Limites actuelles (assumées)

- **Fixtures synthétiques** : aucune clé API réelle n'a été utilisée pour ce lot ; les fixtures de
  `tests/fixtures/openai/` sont construites d'après le schéma officiel et seront remplacées par
  des enregistrements réels anonymisés, puis validées par un contrôle `BUNTU_LIVE=1`.
- `capabilities()` renvoie une valeur provisoire (`tools: true, vision: false`) en attendant le
  catalogue de modèles (T2.6).
- Le client amont hyper-util (ADR-0004) n'existe pas encore : seule la traduction est livrée.
