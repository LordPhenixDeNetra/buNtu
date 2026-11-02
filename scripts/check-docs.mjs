// Vérifie l'état documentaire du dépôt buNtu.
//
// Le code Rust n'existe pas encore (voir AGENTS.md, T0.2) : ce test protège les deux
// documents de référence et le renommage NFrost -> buNtu. Il ne remplace pas la future
// CI Cargo (fmt, clippy, nextest, deny) décrite dans AGENTS.md.
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const required = [
  "buNtu — Cahier des charges et cahier des tâches.md",
  "Guide d'exécution (agents IA).md",
  "AGENTS.md",
  "CLAUDE.md",
];

// Ancien nom du projet : plus aucune occurrence tolérée (le nom « Bifrost » reste légitime).
const forbidden = /\b(?:NFrost|nfrost|NFROST)\b/;

const errors = [];

for (const name of required) {
  try {
    await readFile(path.join(root, name), "utf8");
  } catch {
    errors.push(`fichier manquant : ${name}`);
  }
}

const entries = await readdir(root, { withFileTypes: true });
const markdown = entries
  .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".md"))
  .map((entry) => entry.name);

for (const name of markdown) {
  const content = await readFile(path.join(root, name), "utf8");
  const lines = content.split(/\r?\n/);
  lines.forEach((line, index) => {
    if (forbidden.test(line)) {
      errors.push(`ancien nom détecté : ${name}:${index + 1}`);
    }
  });
}

if (errors.length > 0) {
  console.error("Échec des vérifications documentaires :");
  for (const error of errors) {
    console.error(`  - ${error}`);
  }
  process.exit(1);
}

console.log(
  `OK — ${required.length} fichiers requis présents, ${markdown.length} documents sans ancien nom.`,
);
