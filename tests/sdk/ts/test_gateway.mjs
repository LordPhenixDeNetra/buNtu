// Preuves SDK OpenAI (TypeScript/Node) contre buNtu + simulateur (T1.13).
// Le SDK officiel `openai` est utilisé sans modification : seule l'URL de base change.
import assert from "node:assert/strict";
import OpenAI from "openai";

const base = process.env.BUNTU_BASE_URL;
if (!base) {
  throw new Error("BUNTU_BASE_URL est requis");
}
const client = new OpenAI({ baseURL: `${base}/v1`, apiKey: "sk-test" });
const messages = [{ role: "user", content: "bonjour" }];

const completion = await client.chat.completions.create({
  model: "chat",
  messages,
});
const content = completion.choices?.[0]?.message?.content;
assert.ok(content, "réponse texte attendue");
assert.ok(completion.usage?.prompt_tokens > 0, "usage attendu");

const stream = await client.chat.completions.create({
  model: "chat",
  messages,
  stream: true,
});
let streamed = "";
for await (const chunk of stream) {
  streamed += chunk.choices?.[0]?.delta?.content ?? "";
}
assert.ok(streamed.trim().length > 0, "contenu de flux attendu");

const embeddings = await client.embeddings.create({
  model: "chat",
  input: "bonjour",
});
assert.ok(embeddings.data[0].embedding.length > 0, "vecteur attendu");

const models = await client.models.list();
assert.ok(
  models.data.some((model) => model.id === "chat"),
  "alias `chat` attendu dans /v1/models",
);

console.log("SDK TypeScript : OK");
