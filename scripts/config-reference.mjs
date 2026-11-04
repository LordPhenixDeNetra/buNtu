#!/usr/bin/env node
//! Génère `docs/config-reference.md` depuis `config.schema.json` (T1.27).
//!
//! Usage : `node scripts/config-reference.mjs [--check]`
//! `--check` échoue si le fichier commité ne correspond plus au schéma.
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const schemaPath = path.join(root, "config.schema.json");
const outputPath = path.join(root, "docs", "config-reference.md");

const schema = JSON.parse(readFileSync(schemaPath, "utf8"));

function typeOf(node) {
  if (!node) return "?";
  if (node.$ref) return node.$ref.split("/").pop();
  if (node.anyOf) {
    const types = node.anyOf.filter((entry) => entry.type !== "null").map(typeOf);
    return [...new Set(types)].join(" | ");
  }
  if (node.type === "array") return `liste de ${typeOf(node.items)}`;
  if (Array.isArray(node.type)) return node.type.join(" | ");
  if (node.type === "integer") return "entier";
  if (node.type === "number") return "nombre";
  if (node.type === "string") return "chaîne";
  if (node.type === "boolean") return "booléen";
  if (node.type === "object") return "objet";
  return node.type ?? "?";
}

function defaultOf(node) {
  if (!node || !("default" in node)) return "—";
  const rendered = JSON.stringify(node.default);
  return rendered === undefined ? "—" : `\`${rendered}\``;
}

function requiredList(node) {
  return new Set(node?.required ?? []);
}

function tableFor(name, definition) {
  const required = requiredList(definition);
  const rows = Object.entries(definition.properties ?? {}).map(([field, property]) => {
    const description = (property.description ?? "").replace(/\|/g, "\\|");
    const isRequired = required.has(field) ? "oui" : "non";
    return `| \`${field}\` | ${typeOf(property)} | ${isRequired} | ${defaultOf(property)} | ${description} |`;
  });
  return [
    `### \`${name}\``,
    "",
    definition.description ?? "",
    "",
    "| Champ | Type | Requis | Défaut | Description |",
    "| --- | --- | --- | --- | --- |",
    ...rows,
    "",
  ].join("\n");
}

const sections = [
  ["Configuration complète", schema],
  ...Object.entries(schema.$defs ?? {}),
];

const parts = [
  "# Référence de configuration",
  "",
  "> Fichier **généré** : ne pas éditer à la main. Source : `buntu config schema`",
  "> (`schemars` depuis les types Rust), rendu par `node scripts/config-reference.mjs`.",
  "> La CI échoue si ce fichier n'est pas régénéré.",
  "",
  ...sections.map(([name, definition]) => tableFor(name, definition)),
];

const content = `${parts.join("\n").trimEnd()}\n`;

if (process.argv.includes("--check")) {
  const current = readFileSync(outputPath, "utf8");
  if (current !== content) {
    console.error(
      "docs/config-reference.md est obsolète : exécutez `node scripts/config-reference.mjs` après `buntu config schema > config.schema.json`.",
    );
    process.exit(1);
  }
  console.log("docs/config-reference.md est à jour.");
  process.exit(0);
}

writeFileSync(outputPath, content);
console.log(`écrit ${path.relative(root, outputPath)} (${sections.length} sections)`);
