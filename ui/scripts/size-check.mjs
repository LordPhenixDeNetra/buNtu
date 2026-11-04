import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';

const LIMIT_BYTES = 1024 * 1024;
const root = 'build';

function walk(directory) {
  const entries = readdirSync(directory);
  let files = [];
  for (const entry of entries) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) {
      files = files.concat(walk(path));
    } else {
      files.push(path);
    }
  }
  return files;
}

const files = walk(root);
let total = 0;
for (const file of files) {
  total += gzipSync(readFileSync(file)).length;
}
const kib = (total / 1024).toFixed(1);
console.log(`UI gzip total : ${kib} Kio (${files.length} fichiers)`);
if (total > LIMIT_BYTES) {
  console.error(`UI trop lourde : ${kib} Kio > 1024 Kio`);
  process.exit(1);
}
