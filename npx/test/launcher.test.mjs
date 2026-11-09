import assert from "node:assert/strict";
import { createServer } from "node:http";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import {
  assetName,
  binaryName,
  ensureBinary,
  packageVersion,
  parseChecksum,
  sha256File,
  targetFor,
} from "../cli.js";

test("les triples couvrent les cinq cibles publiées", () => {
  assert.equal(targetFor("linux", "x64"), "x86_64-unknown-linux-musl");
  assert.equal(targetFor("linux", "arm64"), "aarch64-unknown-linux-musl");
  assert.equal(targetFor("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(targetFor("win32", "x64"), "x86_64-pc-windows-msvc");
  assert.throws(() => targetFor("aix", "ppc64"), /non prise en charge/);
});

test("les noms d'assets suivent le schéma de publication", () => {
  assert.equal(assetName("x86_64-unknown-linux-musl"), "buntu-x86_64-unknown-linux-musl");
  assert.equal(assetName("x86_64-pc-windows-msvc"), "buntu-x86_64-pc-windows-msvc.exe");
  assert.equal(binaryName("x86_64-pc-windows-msvc"), "buntu.exe");
});

test("parseChecksum accepte les formats publiés", () => {
  const hash = "a".repeat(64);
  assert.equal(parseChecksum(`${hash}  buntu-x86_64-unknown-linux-musl`, "buntu-x86_64-unknown-linux-musl"), hash);
  assert.equal(parseChecksum(`${hash}\n`, "autre"), hash);
  assert.equal(
    parseChecksum(`SHA256 (buntu-x86_64-apple-darwin) = ${hash}`, "buntu-x86_64-apple-darwin"),
    hash
  );
  assert.throws(() => parseChecksum("pas de hash", "x"), /illisible/);
});

test("téléchargement, vérification SHA-256 et cache", async () => {
  const payload = Buffer.from("binaire factice pour le lanceur");
  const hash = (await import("node:crypto")).createHash("sha256").update(payload).digest("hex");
  const server = createServer((request, response) => {
    if (request.url === "/buntu-x86_64-unknown-linux-musl") {
      response.writeHead(200).end(payload);
      return;
    }
    if (request.url === "/buntu-x86_64-unknown-linux-musl.sha256") {
      response.writeHead(200).end(`${hash}  buntu-x86_64-unknown-linux-musl\n`);
      return;
    }
    response.writeHead(404).end();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  const cacheRoot = mkdtempSync(join(tmpdir(), "buntu-npx-"));
  const options = {
    version: "0.0.0-test",
    triple: "x86_64-unknown-linux-musl",
    releaseBaseUrl: `http://127.0.0.1:${port}`,
    cacheRoot,
  };
  try {
    const binary = await ensureBinary(options);
    assert.deepEqual(readFileSync(binary), payload);
    assert.equal(sha256File(binary), hash);
    // Appel suivant : cache respecté, aucune requête nécessaire.
    server.close();
    const cached = await ensureBinary(options);
    assert.equal(cached, binary);
  } finally {
    rmSync(cacheRoot, { recursive: true, force: true });
  }
});

test("une somme divergente interdit l'exécution", async () => {
  const payload = Buffer.from("binaire corrompu");
  const server = createServer((request, response) => {
    if (request.url.endsWith(".sha256")) {
      response.writeHead(200).end(`${"0".repeat(64)}  buntu-x86_64-unknown-linux-musl\n`);
      return;
    }
    response.writeHead(200).end(payload);
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  const cacheRoot = mkdtempSync(join(tmpdir(), "buntu-npx-bad-"));
  try {
    await assert.rejects(
      ensureBinary({
        version: "0.0.0-test",
        triple: "x86_64-unknown-linux-musl",
        releaseBaseUrl: `http://127.0.0.1:${port}`,
        cacheRoot,
      }),
      /somme SHA-256 invalide/
    );
  } finally {
    server.close();
    rmSync(cacheRoot, { recursive: true, force: true });
  }
});

test("la version par défaut est celle du paquet publié (package.json voisin de cli.js)", () => {
  // `npx` n'a que le contenu du paquet : `cli.js` et `package.json` côte à côte, rien au-dessus.
  const own = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  assert.equal(packageVersion(), own.version);
});

test("package.json déclare le dépôt GitHub (exigé par la provenance npm)", () => {
  // Sans ce champ, npm refuse la publication de confiance : « repository.url is "", expected
  // to match "https://github.com/LordPhenixDeNetra/buNtu" from provenance » (livraison v0.1.2).
  const own = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
  assert.equal(own.repository?.url, "git+https://github.com/LordPhenixDeNetra/buNtu.git");
  assert.equal(own.repository?.directory, "npx");
});
