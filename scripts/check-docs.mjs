// Vérifie l'état documentaire et de livraison du dépôt buNtu.
//
// - fichiers requis (documents de référence + documentation de démarrage T1.27) ;
// - liens relatifs des documents sous docs/ résolus ;
// - ancien nom NFrost interdit partout ;
// - référence de configuration marquée comme générée ;
// - les blocs de commande du démarrage sont couverts par scripts/check-quickstart.sh.
import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const required = [
  "buNtu — Cahier des charges et cahier des tâches.md",
  "Guide d'exécution (agents IA).md",
  "AGENTS.md",
  "CLAUDE.md",
  "docs/quickstart.md",
  "docs/config-reference.md",
  "docs/faq-errors.md",
  "docs/providers/openai.md",
  "docs/providers/anthropic.md",
  "docs/providers/gemini.md",
  "docs/providers/ollama.md",
  "docs/providers/compatible.md",
  "scripts/check-quickstart.sh",
  "Dockerfile",
  "dist-workspace.toml",
  "npx/cli.js",
  "openapi.json",
  "config.schema.json",
];

// Ancien nom du projet : plus aucune occurrence tolérée (le nom « Bifrost » reste légitime).
const forbidden = /\b(?:NFrost|nfrost|NFROST)\b/;

const errors = [];

async function markdownFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const full = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      if (["node_modules", "target", ".git"].includes(entry.name)) continue;
      files.push(...(await markdownFiles(full)));
    } else if (entry.name.toLowerCase().endsWith(".md")) {
      files.push(full);
    }
  }
  return files;
}

for (const name of required) {
  try {
    await readFile(path.join(root, name), "utf8");
  } catch {
    errors.push(`fichier manquant : ${name}`);
  }
}

const files = [path.join(root, "README.md"), ...(await markdownFiles(path.join(root, "docs")))];
for (const file of files) {
  const relative = path.relative(root, file);
  const content = await readFile(file, "utf8");
  const lines = content.split(/\r?\n/);
  for (const [index, line] of lines.entries()) {
    if (forbidden.test(line)) {
      errors.push(`ancien nom détecté : ${relative}:${index + 1}`);
    }
    for (const match of line.matchAll(/\]\(([^)]+)\)/g)) {
      const target = match[1];
      if (/^(?:https?:|mailto:|#)/.test(target) || target.startsWith("<")) continue;
      const cleaned = target.split("#")[0];
      if (cleaned === "") continue;
      const absolute = path.resolve(path.dirname(file), cleaned);
      if (!absolute.startsWith(root)) continue;
      try {
        await stat(absolute);
      } catch {
        errors.push(`lien cassé : ${relative}:${index + 1} → ${target}`);
      }
    }
  }
}

const configReference = await readFile(path.join(root, "docs/config-reference.md"), "utf8");
if (!configReference.includes("Fichier **généré**")) {
  errors.push("docs/config-reference.md ne porte pas l'avertissement de génération");
}

const quickstart = await readFile(path.join(root, "docs/quickstart.md"), "utf8");
if (!quickstart.includes("check-quickstart.sh")) {
  errors.push("docs/quickstart.md ne référence pas les commandes vérifiées en CI");
}

if (errors.length > 0) {
  console.error("Échec des vérifications documentaires :");
  for (const error of errors) {
    console.error(`  - ${error}`);
  }
  process.exit(1);
}

console.log(
  `OK — ${required.length} fichiers requis, ${files.length} documents contrôlés (liens, ancien nom, doc générée).`,
);
