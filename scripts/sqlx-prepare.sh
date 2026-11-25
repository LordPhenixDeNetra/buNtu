#!/usr/bin/env bash
# Régénère le cache hors ligne des requêtes sqlx vérifiées (T1.17) : crates/buntu-store/.sqlx/.
#
# Construit une base temporaire avec les migrations du dépôt, puis recompile buntu-store contre
# elle : chaque macro `sqlx::query!` y décrit sa requête. À relancer après toute modification
# d'une requête ou d'une migration ; la CI échoue si le cache versionné n'est plus à jour.
#
# Usage : scripts/sqlx-prepare.sh   (PYTHON=python sous Windows si `python3` est absent)
set -euo pipefail

cd "$(dirname "$0")/.."

# Chemin natif pour les programmes Windows (Git Bash) ; inchangé ailleurs.
native() { cygpath -m "$1" 2>/dev/null || printf '%s' "$1"; }

workdir="$(mktemp -d)"
trap 'rm -rf "${workdir}"' EXIT
database="$(native "${workdir}")/schema.sqlite"

"${PYTHON:-python3}" - "${database}" <<'EOF'
import glob
import sqlite3
import sys

connection = sqlite3.connect(sys.argv[1])
for migration in sorted(glob.glob("migrations/*.sql")):
    with open(migration, encoding="utf-8") as script:
        connection.executescript(script.read())
connection.close()
EOF

cache="$(native "${PWD}")/crates/buntu-store/.sqlx"
rm -rf "${cache}"
mkdir -p "${cache}"
# Force la recompilation de la crate pour que chaque macro réécrive sa description.
touch crates/buntu-store/src/store.rs
SQLX_OFFLINE=false SQLX_OFFLINE_DIR="${cache}" DATABASE_URL="sqlite://${database}" \
  cargo check --quiet -p buntu-store --all-targets
echo "cache sqlx : $(find "${cache}" -name 'query-*.json' | wc -l) requête(s) dans ${cache}"
