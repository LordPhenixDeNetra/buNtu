#!/usr/bin/env bash
# Preuves SDK officiels (Python + TypeScript) contre buNtu et le simulateur (T1.13).
set -euo pipefail

cargo build --release -p buntu-cli -p buntu-mock-provider

MOCK_LISTEN=127.0.0.1:19001 ./target/release/mock-provider &
MOCK_PID=$!
CONFIG_PATH="${TMPDIR:-/tmp}/buntu-sdk-config.toml"
cat > "${CONFIG_PATH}" <<'EOF'
[server]
listen = "127.0.0.1:19000"

[[providers]]
id = "mock"
type = "mock"
base_url = "http://127.0.0.1:19001"
keys = [{ secret = "sk-test", weight = 1 }]

[[routes]]
alias = "chat"
fallbacks = ["mock/fake-model"]
EOF

./target/release/buntu serve --config "${CONFIG_PATH}" --listen 127.0.0.1:19000 &
GATEWAY_PID=$!
trap 'kill "${MOCK_PID}" "${GATEWAY_PID}" 2>/dev/null || true' EXIT

for _ in $(seq 1 100); do
  if curl -sf http://127.0.0.1:19000/health >/dev/null; then
    break
  fi
  sleep 0.2
done

export BUNTU_BASE_URL=http://127.0.0.1:19000
python -m pytest tests/sdk/python -q
(cd tests/sdk/ts && npm install --silent && node test_gateway.mjs)
