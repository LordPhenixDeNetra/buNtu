#!/usr/bin/env node
//! Trace le surcoût de la passerelle (P50/P99 par palier) en SVG depuis results.json (T1.26).
//!
//! Usage : `node scripts/bench-chart.mjs benchmarks/results/2026-11-04-e6/results.json docs/assets/bench-overhead.svg`
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const input = process.argv[2] ?? "benchmarks/results/2026-11-04-e6/results.json";
const output = process.argv[3] ?? "docs/assets/bench-overhead.svg";

const report = JSON.parse(readFileSync(path.join(root, input), "utf8"));
const overhead = report.overhead ?? [];
if (overhead.length === 0) {
  console.error("aucune mesure de surcoût dans le rapport");
  process.exit(1);
}

const width = 760;
const height = 380;
const margin = { top: 50, right: 24, bottom: 56, left: 72 };
const plotWidth = width - margin.left - margin.right;
const plotHeight = height - margin.top - margin.bottom;
const maxValue = Math.max(...overhead.flatMap((entry) => [entry.p50_us, entry.p99_us]));
const yMax = Math.ceil((maxValue * 1.2) / 100) * 100;
const xStep = plotWidth / Math.max(overhead.length - 1, 1);

const x = (index) => margin.left + index * xStep;
const y = (value) => margin.top + plotHeight - (value / yMax) * plotHeight;

const grid = [];
for (let tick = 0; tick <= 5; tick += 1) {
  const value = (yMax / 5) * tick;
  grid.push(
    `<line x1="${margin.left}" y1="${y(value)}" x2="${width - margin.right}" y2="${y(value)}" stroke="#d9d9d3" stroke-width="1"/>`,
    `<text x="${margin.left - 8}" y="${y(value) + 4}" text-anchor="end" font-size="11" fill="#5f5f5a">${value.toFixed(0)}</text>`
  );
}

const series = [
  { key: "p50_us", color: "#2f6f4f", label: "P50" },
  { key: "p99_us", color: "#a12622", label: "P99" }
];

const lines = [];
for (const item of series) {
  const points = overhead.map((entry, index) => `${x(index)},${y(entry[item.key])}`).join(" ");
  lines.push(
    `<polyline points="${points}" fill="none" stroke="${item.color}" stroke-width="2.5" stroke-linejoin="round"/>`
  );
  for (const [index, entry] of overhead.entries()) {
    lines.push(
      `<circle cx="${x(index)}" cy="${y(entry[item.key])}" r="4" fill="${item.color}"><title>${item.label} à ${entry.qps} req/s : ${entry[item.key].toFixed(0)} µs</title></circle>`
    );
  }
}

const axisLabels = overhead.map((entry, index) => {
  return `<text x="${x(index)}" y="${height - margin.bottom + 20}" text-anchor="middle" font-size="12" fill="#1c1c1a">${entry.qps}</text>`;
});

const legend = series
  .map(
    (item, index) =>
      `<rect x="${margin.left + index * 110}" y="${margin.top - 28}" width="12" height="12" fill="${item.color}"/>` +
      `<text x="${margin.left + 18 + index * 110}" y="${margin.top - 18}" font-size="12" fill="#1c1c1a">${item.label}</text>`
  )
  .join("");

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" role="img" aria-label="Surcoût de la passerelle par palier de débit">
<rect width="100%" height="100%" fill="#ffffff"/>
<text x="${width / 2}" y="24" text-anchor="middle" font-size="15" font-weight="600" fill="#1c1c1a">Surcoût de la passerelle (µs) — médiane de ${report.protocol?.runs ?? 3} exécutions</text>
${legend}
${grid.join("\n")}
${lines.join("\n")}
${axisLabels.join("\n")}
<text x="${width / 2}" y="${height - 10}" text-anchor="middle" font-size="12" fill="#5f5f5a">débit demandé (req/s)</text>
<text x="18" y="${margin.top + plotHeight / 2}" transform="rotate(-90 18 ${margin.top + plotHeight / 2})" text-anchor="middle" font-size="12" fill="#5f5f5a">latence ajoutée (µs)</text>
</svg>
`;

mkdirSync(path.dirname(path.join(root, output)), { recursive: true });
writeFileSync(path.join(root, output), svg);
console.log(`écrit ${output} (${overhead.length} paliers)`);
