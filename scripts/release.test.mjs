// Tests du script de version : `node --test scripts/release.test.mjs`.
import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  bumpCargo,
  bumpPackage,
  cargoVersions,
  changelogSection,
  isSemver,
  maskedPhrases
} from './release.mjs';

const CARGO = `[workspace.package]
version = "0.0.1-rc.1"
edition = "2024"

[workspace.dependencies]
buntu-schema = { version = "0.0.1-rc.1", path = "crates/buntu-schema" }
buntu-cli = { version = "0.0.1-rc.1", path = "crates/buntu-cli" }
buntu-mock-provider = { version = "0.0.1-rc.1", path = "tools/mock-provider" }
serde = { version = "1", features = ["derive"] }
toml = "1.1"
`;

test('semver: official grammar, no leading v, no partial versions', () => {
  for (const ok of ['0.1.0', '1.2.3', '0.1.0-alpha.1', '1.0.0-rc.2+build.5']) {
    assert.equal(isSemver(ok), true, ok);
  }
  for (const ko of ['v0.1.0', '0.1', '01.0.0', '0.1.0-', '1.2.3.4', '']) {
    assert.equal(isSemver(ko), false, ko);
  }
});

test('reads the workspace version and every internal dependency', () => {
  const versions = cargoVersions(CARGO);
  assert.equal(versions.workspace, '0.0.1-rc.1');
  assert.deepEqual(Object.keys(versions.internal), ['buntu-schema', 'buntu-cli', 'buntu-mock-provider']);
});

test('bump rewrites the workspace and internal versions only', () => {
  const bumped = bumpCargo(CARGO, '0.1.0');
  const versions = cargoVersions(bumped);
  assert.equal(versions.workspace, '0.1.0');
  assert.deepEqual(new Set(Object.values(versions.internal)), new Set(['0.1.0']));
  // Les dépendances externes ne bougent pas.
  assert.match(bumped, /serde = \{ version = "1", features/);
  assert.match(bumped, /toml = "1.1"/);
  assert.match(bumped, /edition = "2024"/);
});

test('bump keeps CRLF line endings and the rest of package.json', () => {
  const crlf = CARGO.replace(/\n/g, '\r\n');
  assert.equal(cargoVersions(bumpCargo(crlf, '0.2.0')).workspace, '0.2.0');
  const pkg = '{\n  "name": "buntu",\n  "version": "0.0.1-rc.1",\n  "license": "Apache-2.0"\n}\n';
  const bumped = bumpPackage(pkg, '0.1.0');
  assert.equal(JSON.parse(bumped).version, '0.1.0');
  assert.equal(JSON.parse(bumped).license, 'Apache-2.0');
  assert.ok(bumped.endsWith('}\n'));
});

test('changelog: a version section stops at the next one', () => {
  const text = '# Journal\n\n## [0.2.1] - 2026-11-26\n\n- fix\n\n## [0.2.0] - 2026-11-26\n\n- old\n';
  assert.equal(changelogSection(text, '0.2.1'), '## [0.2.1] - 2026-11-26\n\n- fix\n');
  assert.equal(changelogSection(text, '0.3.0'), null);
});

test('changelog: phrases masked by GitHub Actions are reported', () => {
  // v0.2.0 : « Bearer value » a fait sauter la construction des archives cargo-dist.
  assert.deepEqual(maskedPhrases('Wipe the temporary Bearer value and drop'), ['Bearer value']);
  assert.deepEqual(maskedPhrases('Wipe the temporary authorization header value'), []);
});
