#!/usr/bin/env bash
# Instance réelle pour les tests E2E : simulateur (19001) + buntu serve (19000, SQLite temporaire).
set -euo pipefail

cd "$(dirname "$0")/../.."

# Git Bash / CI : cargo vit dans ~/.cargo/bin, pas toujours dans le PATH du shell.
export PATH="${HOME}/.cargo/bin:${PATH}"

cargo build -p buntu-cli -p buntu-mock-provider

DB_PATH="${TMPDIR:-/tmp}/buntu-e2e-$$.sqlite"
CONFIG_PATH="${TMPDIR:-/tmp}/buntu-e2e-$$.toml"
cat > "${CONFIG_PATH}" <<'EOF'
[server]
listen = "127.0.0.1:19000"

[[providers]]
id = "e2e"
type = "mock"
base_url = "http://127.0.0.1:19001"
keys = [{ secret = "test-key-0001", weight = 1 }]

[[routes]]
alias = "chat-e2e"
fallbacks = ["e2e/modele-a", "e2e/modele-b"]
EOF

MOCK_LISTEN=127.0.0.1:19001 ./target/debug/mock-provider &
MOCK_PID=$!
BUNTU_DB_PATH="${DB_PATH}" ./target/debug/buntu serve --config "${CONFIG_PATH}" --listen 127.0.0.1:19000 &
GATEWAY_PID=$!
trap 'kill "${MOCK_PID}" "${GATEWAY_PID}" 2>/dev/null || true; rm -f "${DB_PATH}" "${CONFIG_PATH}"' EXIT

for _ in $(seq 1 100); do
  if curl -sf http://127.0.0.1:19000/health >/dev/null && curl -sf http://127.0.0.1:19001/health >/dev/null; then
    break
  fi
  sleep 0.2
done

# Le script reste le propriétaire des serveurs pendant toute la durée des tests.
wait "${GATEWAY_PID}"
