#!/usr/bin/env bash
# Exécute les commandes du guide « Démarrage en 5 minutes » (T1.27) contre une instance réelle.
# Utilisé en CI (job docs) : toute régression du parcours UC1 casse la CI.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT="$PWD"
WORK="${TMPDIR:-/tmp}/buntu-quickstart-$$"
BIN_DIR="${CARGO_TARGET_DIR:-$ROOT/target}/release"
MOCK_PID=""
GATEWAY_PID=""
cleanup() {
  if [ -n "${MOCK_PID}" ]; then kill "${MOCK_PID}" 2>/dev/null || true; fi
  if [ -n "${GATEWAY_PID}" ]; then kill "${GATEWAY_PID}" 2>/dev/null || true; fi
  wait 2>/dev/null || true
  sleep 0.5
  rm -rf "${WORK}" 2>/dev/null || true
}
trap cleanup EXIT

mkdir -p "${WORK}"
echo "== construction de la console et des binaires"
(cd ui && npm ci --no-audit --no-fund && npm run build)
cargo build --release --locked -p buntu-cli -p buntu-mock-provider

echo "== configuration minimale (simulateur, sans compte fournisseur)"
cat > "${WORK}/buntu.toml" <<'EOF'
[server]
listen = "127.0.0.1:8080"

[[providers]]
id = "mock"
type = "mock"
base_url = "http://127.0.0.1:19001"
keys = [{ secret = "test-key", weight = 1 }]

[[routes]]
alias = "chat"
fallbacks = ["mock/fake-model"]
EOF

MOCK_LISTEN=127.0.0.1:19001 "${BIN_DIR}/mock-provider" &
MOCK_PID=$!
BUNTU_DB_PATH="${WORK}/buntu.sqlite" "${BIN_DIR}/buntu" serve --config "${WORK}/buntu.toml" --listen 127.0.0.1:8080 &
GATEWAY_PID=$!

for _ in $(seq 1 150); do
  if curl -sf http://127.0.0.1:8080/ready >/dev/null; then
    break
  fi
  sleep 0.2
done

echo "== /health et /ready"
curl -sf http://127.0.0.1:8080/health >/dev/null
curl -sf http://127.0.0.1:8080/ready >/dev/null

echo "== premier appel chat (format OpenAI)"
CHAT="${WORK}/chat.json"
curl -sf http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"chat","messages":[{"role":"user","content":"bonjour"}]}' > "${CHAT}"
grep -q '"content"' "${CHAT}"
grep -q 'chat.completion' "${CHAT}"

echo "== liste des modèles"
MODELS="${WORK}/models.json"
curl -sf http://127.0.0.1:8080/v1/models > "${MODELS}"
grep -q '"chat"' "${MODELS}"

echo "== console web embarquée"
curl -sf http://127.0.0.1:8080/ | grep -qi '<!doctype html'

echo "quickstart OK — UC1 exécuté de bout en bout (simulateur)"
