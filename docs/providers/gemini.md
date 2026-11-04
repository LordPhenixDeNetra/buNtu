# Gemini (AI Studio) — correspondance canonique ↔ `generateContent` (T1.11)

Sources officielles consultées le **2026-11-04** :

- `generateContent` : <https://ai.google.dev/api/generate-content>
- Appels de fonctions : <https://ai.google.dev/gemini-api/docs/function-calling>
- Streaming : <https://ai.google.dev/gemini-api/docs/streaming>
- Dépannage / codes : <https://ai.google.dev/gemini-api/docs/troubleshooting>

Endpoints : `POST /v1beta/models/{model}:generateContent` et
`POST /v1beta/models/{model}:streamGenerateContent?alt=sse`.

## Requête

| Canonique | Gemini | Remarque |
| --- | --- | --- |
| `model` | dans l'URL | `{model}:generateContent` |
| messages `system`/`developer` | `systemInstruction.parts[].text` | — |
| messages user/assistant | `contents[]` avec rôles `user`/`model` | — |
| `ContentPart::Text` | `parts[].text` | — |
| `ContentPart::Image` | `parts[].inlineData {mimeType,data}` | base64 uniquement (URL refusée) |
| `ContentPart::ToolCall` | `parts[].functionCall {name,args}` | pas d'identifiant côté Gemini |
| `ContentPart::ToolResult` | `parts[].functionResponse {name,response}` | `tool_call_id` sert de `name` (limite) |
| `tools[]` | `tools[].functionDeclarations[]` | — |
| `tool_choice` | `toolConfig.functionCallingConfig` | `AUTO` / `ANY` (+ `allowedFunctionNames`) / `NONE` |
| `max_tokens` | `generationConfig.maxOutputTokens` | — |
| `temperature`, `top_p`, `stop` | `temperature`, `topP`, `stopSequences` | — |
| `response_format` | `responseMimeType` / `responseSchema` | `JsonObject` et `JsonSchema` supportés |

## Réponse

| Gemini | Canonique |
| --- | --- |
| `candidates[0].content.parts[].text` | `ContentPart::Text` |
| `candidates[0].content.parts[].functionCall` | `ContentPart::ToolCall` (id généré `call-N`) |
| `candidates[0].content.parts[].inlineData` | `ContentPart::Image` (base64) |
| `finishReason` | `STOP`→`EndTurn`, `MAX_TOKENS`→`MaxTokens`, `SAFETY`/`RECITATION`/`BLOCKLIST`/`PROHIBITED_CONTENT`→`ContentFilter`, autre→`Other` |
| `usageMetadata.promptTokenCount` | `usage.input_tokens` |
| `usageMetadata.candidatesTokenCount` | `usage.output_tokens` |
| `usageMetadata.cachedContentTokenCount` | `usage.cached_input_tokens` |
| `usageMetadata.thoughtsTokenCount` | `usage.reasoning_tokens` |

Un appel de fonction force `stop_reason = ToolCalls`, même si Gemini renvoie `STOP`.

## Flux (SSE)

Chaque `data:` est un `GenerateContentResponse` partiel : `Start` au premier chunk
(`responseId`, `modelVersion`), `ContentDelta` pour les textes, `ToolCallDelta` pour les appels de
fonction (arguments en une fois), `Usage` si `usageMetadata` est présent, puis `Stop` + `Done` sur
`finishReason`. Un frame `error` devient `ProviderUnavailable`.

## Erreurs

| Statut | Canonique |
| --- | --- |
| 400, 404, 422 | `InvalidRequest` |
| 401, 403 | `Authentication` |
| 429 | `ProviderRateLimited` (`retry-after`) |
| 500, 502, 503, 504 | `ProviderUnavailable` |

## Limites actuelles (assumées)

- Fixtures **synthétiques** ; contrôle `BUNTU_LIVE=1` à faire (clé AI Studio requise).
- Images par URL refusées (seul `inlineData` base64 est produit).
- `functionResponse.name` utilise `tool_call_id` : le schéma canonique ne transporte pas le nom
  de l'outil dans le résultat (à corriger si le besoin apparaît en T1.13).
- Les `parameters` d'outils sont transmis tels quels : Gemini n'accepte qu'un **sous-ensemble
  d'OpenAPI** ; le filtrage des mots-clés non supportés est laissé au client amont (réserve).
- La clé API devra être envoyée en **en-tête** (`x-goog-api-key`), jamais en paramètre d'URL :
  c'est le rôle du client amont (ADR-0004 / T1.13), pas du traducteur.
- `capabilities()` provisoire en attendant T2.6.
