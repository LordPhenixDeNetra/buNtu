# ADR-0002 — Schéma canonique v0

- **Statut :** accepté (T0.5)
- **Date :** 3 novembre 2026
- **Contexte :** cahier des charges §6.1 (format canonique interne), §6.2 (`buntu-schema`), T0.5.

## Décision

Toute requête est traduite une fois vers des types Rust propres à buNtu (`crates/buntu-schema`),
puis traduite vers le fournisseur visé. N formats d'entrée × M fournisseurs deviennent N + M
traducteurs.

Règles retenues :

1. **Messages unifiés.** Un message a un rôle (`system`, `developer`, `user`, `assistant`,
   `tool`, `function`) et un contenu en parties typées. Le `system` top-level d'Anthropic et le
   `systemInstruction` de Gemini seront extraits vers des messages `system` par leurs
   traducteurs respectifs.
2. **Appels d'outils dans le contenu.** Un appel d'outil est une partie `tool_call` (comme les
   blocs `tool_use` d'Anthropic) ; les appels au niveau message d'OpenAI y sont convertis. Le
   résultat d'outil est une partie `tool_result`, portée par un message `tool` au format
   OpenAI (`tool_call_id` intégré à la partie).
3. **Enums, pas de chaînes libres.** Rôles, raisons d'arrêt, choix d'outil et niveaux de détail
   d'image sont des enums. Une valeur inconnue tombe dans `Other(String)` (raison d'arrêt,
   choix d'outil) au lieu d'être perdue.
4. **Passthrough.** Chaque structure (`Request`, `Message`, `Tool`) possède un champ `extra`
   aplati (`#[serde(flatten)] Map<String, Value>`) : un champ fournisseur non modélisé est
   conservé et réémis, jamais perdu.
5. **Types de contenu inconnus conservés bruts.** `ContentPart` a une sérialisation manuelle :
   un `type` non reconnu devient `ContentPart::Unknown(Value)` et ressort tel quel.
6. **Compteurs entiers.** `Usage` ne contient que des `u32` (entrée, sortie, cache, réflexion).
   Les montants seront calculés en `rust_decimal` dans `buntu-governance` (cahier §6.3) :
   aucun `f64` pour l'argent.
7. **Aucun code fournisseur dans le crate.** Les traducteurs OpenAI/Anthropic/Gemini vivront
   dans `buntu-providers` (T1.9–T1.11) ; les convertisseurs de test sont dans
   `crates/buntu-schema/tests/`.

## Correspondance des concepts

| Concept | OpenAI Chat Completions | Anthropic Messages | Gemini `generateContent` | Canonique |
| --- | --- | --- | --- | --- |
| Rôles | `system`, `developer`, `user`, `assistant`, `tool`, `function` | `user`, `assistant` (+ `system` top-level) | `user`, `model` (+ `systemInstruction`) | `Role` (les 6 rôles OpenAI) |
| Texte | partie `{"type":"text"}` ou chaîne | bloc `{"type":"text"}` | `parts[].text` | `ContentPart::Text` |
| Image | `{"type":"image_url","image_url":{"url","detail"}}` | `{"type":"image","source":{"type":"base64"/"url"/"file"}}` | `parts[].inlineData`/`fileData` | `Image { source, detail }` |
| Audio entrant | `{"type":"input_audio","input_audio":{"data","format"}}` | — | `parts[].inlineData` (mimeType audio) | `Audio { data, format }` |
| Fichier | `{"type":"file","file":{"file_id","file_data","filename"}}` | `{"type":"document"}` | `parts[].fileData` | `File { file_id }` |
| Appel d'outil | `message.tool_calls[]` (`function.arguments` en chaîne JSON) | bloc `{"type":"tool_use","id","name","input"}` | `parts[].functionCall {name,args}` | `ContentPart::ToolCall` / `ToolCall` |
| Résultat d'outil | message `role:"tool"` + `tool_call_id` | bloc `{"type":"tool_result","tool_use_id"}` | `parts[].functionResponse` | `ContentPart::ToolResult` / `ToolResult` |
| Définition d'outil | `{"type":"function","function":{name,description,parameters}}` | `{name,description,input_schema}` | `tools[].functionDeclarations[]` | `Tool` |
| Choix d'outil | `"auto"`, `"none"`, `"required"`, `{"type":"function","function":{"name"}}` | `{type: auto/any/tool/none}` | `toolConfig.functionCallingConfig` | `ToolChoice` |
| Raison d'arrêt | `finish_reason`: `stop`, `length`, `tool_calls`, `content_filter` | `stop_reason`: `end_turn`, `max_tokens`, `stop_sequence`, `tool_use`, `pause_turn`, `refusal` | `finishReason`: `STOP`, `MAX_TOKENS`, `SAFETY`, `RECITATION`… | `StopReason` |
| Usage | `prompt_tokens`, `completion_tokens`, `prompt_tokens_details.cached_tokens`, `completion_tokens_details.reasoning_tokens` | `input_tokens`, `output_tokens`, `cache_read_input_tokens`, `cache_creation_input_tokens` | `usageMetadata.promptTokenCount`, `candidatesTokenCount`, `cachedContentTokenCount`, `thoughtsTokenCount` | `Usage` |
| Réflexion | non exposée dans Chat Completions (seulement `reasoning_tokens`) | bloc `{"type":"thinking","thinking","signature"}` | `parts[].thought` | `ContentPart::Reasoning` |
| Cache de prompt | `prompt_cache_breakpoint` sur les parties, `prompt_tokens_details.cached_tokens` | `cache_control` sur les blocs, `cache_read_input_tokens` | `cachedContents` + `cachedContentTokenCount` | `Usage::cached_input_tokens` (+ passthrough `extra` pour l'instant) |
| Refus | `{"type":"refusal","refusal"}` ou `message.refusal` | `stop_reason: "refusal"` | `finishReason` de sécurité | `ContentPart::Refusal` |
| Streaming | `chat.completion.chunk` SSE + `data: [DONE]` | événements nommés `message_start`, `content_block_delta`, `message_delta`, `message_stop` | flux `streamGenerateContent` | `StreamEvent` |

## Sources consultées (3 novembre 2026)

- OpenAI — *Create chat completion* :
  <https://developers.openai.com/api/reference/resources/chat/subresources/completions/methods/create.md>
- OpenAI — *Chat Completions overview* :
  <https://developers.openai.com/api/reference/chat-completions/overview.md>
- Anthropic — *Create a Message* :
  <https://platform.claude.com/docs/en/api/messages>
- Google — *API Discovery, Generative Language v1beta* (schémas `GenerateContentRequest`,
  `GenerationConfig`, `UsageMetadata`) :
  <https://generativelanguage.googleapis.com/$discovery/rest?version=v1beta>
- Google — *generateContent* : <https://ai.google.dev/api/generate-content>

## Limites connues de la v0

- Aucune capture réelle de fournisseur : le dépôt n'a pas de clés API. Les jeux de test fixes
  sont tirés des **exemples de la documentation officielle**, cités ci-dessus, et le test
  proptest génère 200 requêtes. T1.8/T1.9 remplaceront ces exemples par des fixtures réelles
  anonymisées.
- Les événements de flux sont modélisés (`StreamEvent`) mais leur correspondance fine par
  fournisseur (fragments d'arguments, usage en fin de flux) sera faite en T1.9–T1.11.
- Les paramètres de génération non modélisés (par exemple `seed`, `logprobs`) restent dans
  `extra` jusqu'à ce qu'un besoin réel justifie un champ typé (règle : pas d'abstraction sans
  deux usages).
