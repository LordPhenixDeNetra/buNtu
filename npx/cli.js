#!/usr/bin/env node
"use strict";

/**
 * Lanceur npm `@buntu/cli` (T1.25) : `npx @buntu/cli`.
 *
 * Télécharge le binaire natif publié dans la release GitHub du même numéro de version,
 * vérifie sa somme SHA-256 avant toute exécution, puis lui passe les arguments reçus.
 * Aucun exécutable n'est lancé avant la vérification.
 */

const { createHash } = require("node:crypto");
const { spawn } = require("node:child_process");
const {
  chmodSync,
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} = require("node:fs");
const { homedir, platform, arch } = require("node:os");
const { join } = require("node:path");

const REPOSITORY = "LordPhenixDeNetra/buNtu";
const USER_AGENT = "buntu-npx-launcher";

const TARGETS = {
  win32: {
    x64: "x86_64-pc-windows-msvc",
    arm64: "aarch64-pc-windows-msvc",
  },
  darwin: {
    x64: "x86_64-apple-darwin",
    arm64: "aarch64-apple-darwin",
  },
  linux: {
    x64: "x86_64-unknown-linux-musl",
    arm64: "aarch64-unknown-linux-musl",
  },
};

function targetFor(platformName, archName) {
  const triple = TARGETS[platformName]?.[archName];
  if (!triple) {
    throw new Error(`plateforme non prise en charge : ${platformName}/${archName}`);
  }
  return triple;
}

function assetName(triple) {
  return `buntu-${triple}${triple.includes("windows") ? ".exe" : ""}`;
}

function binaryName(triple) {
  return triple.includes("windows") ? "buntu.exe" : "buntu";
}

function releaseBase(version) {
  return `https://github.com/${REPOSITORY}/releases/download/v${version}`;
}

function sha256File(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

/** Accepte « <hash>  <fichier> », « SHA256 (<fichier>) = <hash> » ou un hash seul. */
function parseChecksum(text, expectedFile) {
  const lines = text
    .trim()
    .split(/\r?\n/)
    .filter((line) => line.trim() !== "");
  const line = lines.find((candidate) => candidate.includes(expectedFile)) ?? lines[0];
  if (!line) {
    throw new Error("fichier de somme SHA-256 vide");
  }
  const match = line.match(/[0-9a-fA-F]{64}/);
  if (!match) {
    throw new Error(`somme SHA-256 illisible : ${line}`);
  }
  return match[0].toLowerCase();
}

async function fetchBytes(url) {
  const response = await fetch(url, { headers: { "user-agent": USER_AGENT } });
  if (!response.ok) {
    throw new Error(`téléchargement impossible (${response.status}) : ${url}`);
  }
  return Buffer.from(await response.arrayBuffer());
}

async function fetchText(url) {
  const response = await fetch(url, { headers: { "user-agent": USER_AGENT } });
  if (!response.ok) {
    throw new Error(`téléchargement impossible (${response.status}) : ${url}`);
  }
  return response.text();
}

async function download(url, destination) {
  writeFileSync(destination, await fetchBytes(url));
}

async function ensureBinary(options = {}) {
  const version = options.version ?? require("../package.json").version;
  const triple = options.triple ?? targetFor(platform(), arch());
  const base = options.releaseBaseUrl ?? releaseBase(version);
  const asset = assetName(triple);
  const cacheRoot = options.cacheRoot ?? join(homedir(), ".cache", "buntu");
  const directory = join(cacheRoot, version);
  const binary = join(directory, binaryName(triple));
  const checksumPath = `${binary}.sha256`;

  mkdirSync(directory, { recursive: true });

  if (existsSync(binary) && existsSync(checksumPath)) {
    const expected = readFileSync(checksumPath, "utf8").trim();
    if (sha256File(binary) === expected) {
      return binary;
    }
    rmSync(binary, { force: true });
  }

  const temporary = `${binary}.download`;
  await download(`${base}/${asset}`, temporary);
  const expected = parseChecksum(await fetchText(`${base}/${asset}.sha256`), asset);
  const actual = sha256File(temporary);
  if (actual !== expected) {
    rmSync(temporary, { force: true });
    throw new Error(`somme SHA-256 invalide pour ${asset} (attendu ${expected}, obtenu ${actual})`);
  }
  renameSync(temporary, binary);
  writeFileSync(checksumPath, `${expected}\n`);
  if (!triple.includes("windows")) {
    chmodSync(binary, 0o755);
  }
  return binary;
}

async function main() {
  const binary = await ensureBinary();
  const child = spawn(binary, process.argv.slice(2), { stdio: "inherit" });
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? 1);
  });
}

if (require.main === module) {
  main().catch((error) => {
    console.error(`buNtu: ${error.message}`);
    process.exit(1);
  });
}

module.exports = {
  assetName,
  binaryName,
  ensureBinary,
  fetchBytes,
  fetchText,
  parseChecksum,
  releaseBase,
  sha256File,
  targetFor,
};
