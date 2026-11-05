# Démarrage en 5 minutes (T1.27)

Objectif : obtenir une réponse de la passerelle via une **API unique compatible OpenAI**, sans
compte fournisseur, puis brancher un vrai fournisseur.

> Les commandes de ce guide sont **exécutées en CI** (`scripts/check-quickstart.sh`) : elles
> correspondent exactement à ce que fait la passerelle. Durée mesurée sur une machine de
> développement : moins de 5 minutes après compilation.

## Prérequis

- Rust stable (`rust-toolchain.toml`), `git`, `curl`.
- Node.js ≥ 20 (pour construire la console web embarquée dans le binaire).
- Ports libres : `8080` (passerelle), `19001` (simulateur).

## 1. Construire (≈ 2 min la première fois)

```sh
cd ui && npm ci && npm run build && cd ..
cargo build --release -p buntu-cli -p buntu-mock-provider
```

## 2. Démarrer le simulateur et la passerelle

```sh
MOCK_LISTEN=127.0.0.1:19001 ./target/release/mock-provider &
cat > buntu.toml <<'EOF'
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
BUNTU_DB_PATH=./buntu.sqlite ./target/release/buntu serve --config buntu.toml &
```

## 3. Premier appel (format OpenAI, comme n'importe quel client)

```sh
curl -s http://127.0.0.1:8080/v1/chat/completions \
  -H 'content-type: application/json' \
  -d '{"model":"chat","messages":[{"role":"user","content":"bonjour"}]}'
curl -s http://127.0.0.1:8080/v1/models
```

Réponses attendues : un JSON `chat.completion` avec `choices[0].message.content`, puis la liste
des modèles (alias `chat` et cible `mock/fake-model`).

## 4. Brancher un vrai fournisseur

Remplacez le bloc `providers` par votre fournisseur et référencez la clé par variable
d'environnement (`env:VAR`, jamais la clé en clair dans le fichier) :

```toml
[[providers]]
id = "openai"
type = "openai"
base_url = "https://api.openai.com"
keys = [{ secret = "env:OPENAI_API_KEY", weight = 1 }]

[[routes]]
alias = "chat"
fallbacks = ["openai/gpt-4o-mini"]
```

```sh
export OPENAI_API_KEY=...   # votre clé
./target/release/buntu serve --config buntu.toml
```

Appelez toujours `http://127.0.0.1:8080/v1/chat/completions` : si OpenAI tombe, la chaîne de
repli peut basculer vers une autre cible sans changer le code client.

## Image conteneur (après la première release)

```sh
docker run --rm -p 8080:8080 ghcr.io/lordphenixdenetra/buntu:latest
curl -s http://127.0.0.1:8080/health   # {"status":"ok"}
```

Sans fichier de configuration, la passerelle démarre avec les valeurs par défaut ; montez un
fichier (`-v "$PWD/buntu.toml:/etc/buntu.toml"`) et passez `--config /etc/buntu.toml`.

## Étapes suivantes

- [Référence de configuration générée](config-reference.md)
- [Guides fournisseurs](providers/openai.md) : OpenAI, Anthropic, Gemini, Ollama, compatible.
- [Erreurs et dépannage](faq-errors.md)
- Console web : `http://127.0.0.1:8080/` (fournisseurs, routes, journal des requêtes).

## Accès à l'administration

L'API d'administration (`/admin/v1/*`) et la console exigent un jeton
(`Authorization: Bearer <jeton>`). Au premier démarrage, buNtu génère un jeton, l'affiche
**une seule fois** dans la sortie standard et n'en garde que l'empreinte en base. Vous pouvez aussi
fournir le vôtre (32 caractères minimum) :

```sh
BUNTU_ADMIN_TOKEN="$(openssl rand -hex 32)" buntu serve --config buntu.toml
```

Par défaut, buNtu n'écoute que sur `127.0.0.1:8080`. Pour l'exposer au réseau, passez
`--listen 0.0.0.0:8080` (c'est ce que fait l'image Docker).
