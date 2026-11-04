# Anthropic — correspondance canonique ↔ Messages API (T1.10)

Sources officielles consultées le **2026-11-04** :

- Messages : <https://docs.anthropic.com/en/api/messages>
- Streaming : <https://docs.anthropic.com/en/api/messages-streaming>
- Erreurs : <https://docs.anthropic.com/en/api/errors>
- Outils : <https://docs.anthropic.com/en/docs/build-with-claude/tool-use>

Endpoint : `POST /v1/messages`, en-tête `anthropic-version: 2023-06-01`.

## Requête

| Canonique | Anthropic | Remarque |
| --- | --- | --- |
| `model` | `model` | — |
| `max_tokens` | `max_tokens` | **obligatoire** ; défaut du fournisseur : 4096 |
| messages `system`/`developer` | `system` (chaîne) | extrait hors de `messages` |
| `messages[].role` | `user` / `assistant` | un message `tool` devient un tour `user` |
| `ContentPart::Text` | bloc `{type:"text"}` | — |
| `ContentPart::Image` | bloc `{type:"image",source}` | `base64` ou `url` |
| `ContentPart::ToolCall` | bloc `{type:"tool_use",id,name,input}` | `arguments` deviennent `input` |
| `ContentPart::ToolResult` | bloc `{type:"tool_result",tool_use_id,content}` | dans un tour `user` |
| `tools[]` | `tools[] {name,description,input_schema}` | — |
| `tool_choice` | `auto` / `none` / `any` / `{type:"tool",name}` | — |
| `temperature`, `top_p` | mêmes champs | — |
| `stop` | `stop_sequences` | — |
| `stream` | `stream` | — |
| `response_format` | — | `JsonObject` → consigne système ; `JsonSchema` refusé explicitement |

## Réponse

| Anthropic | Canonique |
| --- | --- |
| `content[] {type:"text"}` | `ContentPart::Text` |
| `content[] {type:"tool_use"}` | `ContentPart::ToolCall` |
| `content[] {type:"thinking"}` / `redacted_thinking` | `ContentPart::Reasoning` (signature conservée) |
| `stop_reason` | `end_turn`→`EndTurn`, `max_tokens`→`MaxTokens`, `stop_sequence`→`StopSequence`, `tool_use`→`ToolCalls`, `pause_turn`→`Pause`, `refusal`→`Refusal` |
| `usage.input_tokens` / `output_tokens` | `usage.input_tokens` / `output_tokens` |
| `usage.cache_read_input_tokens` | `usage.cached_input_tokens` |

## Flux (SSE, événements nommés)

`message_start` → `Start` + `Usage` (entrée) ; `content_block_start` (tool_use) → `ToolCallDelta`
(id, nom) ; `content_block_delta` `text_delta`/`thinking_delta` → `ContentDelta`,
`input_json_delta` → `ToolCallDelta` (fragment d'arguments) ; `message_delta` → `Stop` + `Usage`
(sortie) ; `message_stop` → `Done` ; `error` → `ProviderUnavailable` (le flux est déjà commencé,
pas de réessai).

## Erreurs

| Statut | Canonique |
| --- | --- |
| 400, 404, 422 | `InvalidRequest` |
| 401, 403 | `Authentication` |
| 429 | `ProviderRateLimited` (`retry-after`) |
| 500, 502, 503, 504, 529 | `ProviderUnavailable` |

## Limites actuelles (assumées)

- Fixtures **synthétiques** (aucune clé API) ; contrôle `BUNTU_LIVE=1` à faire.
- `response_format` JSON Schema n'existe pas côté Anthropic : refus explicite, pas d'invention.
- `capabilities()` provisoire (`tools: true, vision: true`) en attendant T2.6.
