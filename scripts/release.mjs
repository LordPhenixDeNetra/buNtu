#!/usr/bin/env node
// Gestion de version de buNtu (docs/release.md, ADR-0009).
//
//   node scripts/release.mjs check [vX.Y.Z]   versions cohérentes (et égales au tag s'il est donné)
//   node scripts/release.mjs bump X.Y.Z       monte la version partout, Cargo.lock et CHANGELOG
//
// Une seule version pour tout le dépôt : workspace Cargo, dépendances internes déclarées dans
// `[workspace.dependencies]` (leur `version` est exigée par cargo-deny) et lanceur npm.
// Le script ne commite ni ne tague rien : il affiche les commandes à lancer.
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const CARGO = path.join(root, 'Cargo.toml');
const NPX = path.join(root, 'npx', 'package.json');
const CHANGELOG = path.join(root, 'CHANGELOG.md');

// Grammaire officielle : https://semver.org/#is-there-a-suggested-regular-expression-regex-to-check-a-semver-string
const SEMVER =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+([0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?$/;

const WORKSPACE_VERSION = /(\[workspace\.package\]\r?\n(?:[^\[]*?\r?\n)?version = ")([^"]+)(")/;
const INTERNAL = /^(buntu-[\w-]+) = \{ version = "([^"]+)", path = /gm;

export function isSemver(version) {
  return SEMVER.test(version);
}

export function cargoVersions(text) {
  const workspace = text.match(WORKSPACE_VERSION)?.[2];
  const internal = Object.fromEntries([...text.matchAll(INTERNAL)].map((m) => [m[1], m[2]]));
  return { workspace, internal };
}

export function bumpCargo(text, version) {
  return text
    .replace(WORKSPACE_VERSION, `$1${version}$3`)
    .replace(INTERNAL, (_line, name) => `${name} = { version = "${version}", path = `);
}

export function bumpPackage(text, version) {
  return text.replace(/("version":\s*")[^"]+(")/, `$1${version}$2`);
}

/** Toutes les versions du dépôt, par emplacement. */
function versions() {
  const cargo = cargoVersions(readFileSync(CARGO, 'utf8'));
  return {
    'Cargo.toml [workspace.package]': cargo.workspace,
    ...Object.fromEntries(
      Object.entries(cargo.internal).map(([name, v]) => [`Cargo.toml ${name}`, v])
    ),
    'npx/package.json': JSON.parse(readFileSync(NPX, 'utf8')).version
  };
}

function check(tag) {
  const found = versions();
  const expected = tag ? tag.replace(/^v/, '') : found['Cargo.toml [workspace.package]'];
  const wrong = Object.entries(found).filter(([, v]) => v !== expected);
  if (!isSemver(expected ?? '')) {
    console.error(`version invalide (SemVer attendu) : ${expected}`);
    process.exit(1);
  }
  if (wrong.length > 0) {
    console.error(`versions incohérentes (attendu ${expected}) :`);
    for (const [where, v] of wrong) console.error(`  ${where} = ${v}`);
    process.exit(1);
  }
  console.log(`OK — ${Object.keys(found).length} emplacements en ${expected}${tag ? ` (tag ${tag})` : ''}.`);
}

function hasGitCliff() {
  try {
    execFileSync('git-cliff', ['--version'], { stdio: 'ignore' });
    return true;
  } catch {
    return false;
  }
}

function bump(version) {
  if (!isSemver(version)) {
    console.error(`version invalide (SemVer, sans « v ») : ${version}`);
    process.exit(1);
  }
  writeFileSync(CARGO, bumpCargo(readFileSync(CARGO, 'utf8'), version));
  writeFileSync(NPX, bumpPackage(readFileSync(NPX, 'utf8'), version));
  // Cargo.lock suit les versions du workspace, sans toucher aux dépendances externes.
  execFileSync('cargo', ['update', '--workspace', '--offline'], { cwd: root, stdio: 'inherit' });
  check(`v${version}`);

  if (hasGitCliff()) {
    const args = existsSync(CHANGELOG)
      ? ['--config', 'cliff.toml', '--unreleased', '--tag', `v${version}`, '--prepend', 'CHANGELOG.md']
      : ['--config', 'cliff.toml', '--tag', `v${version}`, '--output', 'CHANGELOG.md'];
    execFileSync('git-cliff', args, { cwd: root, stdio: 'inherit' });
    console.log('CHANGELOG.md mis à jour : relisez-le avant de commiter.');
  } else {
    console.warn('git-cliff absent : CHANGELOG.md non généré (cargo install git-cliff --locked).');
  }

  console.log(`\nÉtapes suivantes (docs/release.md) :
  git add Cargo.toml Cargo.lock npx/package.json CHANGELOG.md
  git commit -s -m "chore(release): v${version}"
  # après fusion sur main :
  git tag -a v${version} -m "v${version}" && git push origin v${version}`);
}

const [command, argument] = process.argv.slice(2);
if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  if (command === 'check') {
    check(argument);
  } else if (command === 'bump' && argument) {
    bump(argument);
  } else {
    console.error('usage : node scripts/release.mjs check [vX.Y.Z] | bump X.Y.Z');
    process.exit(2);
  }
}
