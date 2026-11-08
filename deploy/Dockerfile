# syntax=docker/dockerfile:1
#
# Image de buNtu : binaire musl statique + console embarquée, sur distroless non root.
#
# Le contexte de build est la RACINE du dépôt :
#   docker build -f deploy/Dockerfile -t buntu .
# Le fichier d'exclusion est `deploy/Dockerfile.dockerignore` (prioritaire sur un
# `.dockerignore` racine) : https://docs.docker.com/build/concepts/context/ (consulté le
# 2026-11-08).

# 1) Console web statique (nécessaire à `rust-embed`). Du JavaScript indépendant de
#    l'architecture : construit une seule fois sur la plateforme de build, jamais sous émulation.
FROM --platform=$BUILDPLATFORM node:22-bookworm-slim AS ui
WORKDIR /src
COPY ui/package.json ui/package-lock.json ./ui/
RUN --mount=type=cache,target=/root/.npm \
    cd ui && npm ci --no-audit --no-fund
COPY openapi.json ./openapi.json
COPY ui ./ui
RUN cd ui && npm run build

# 2) Binaire statique musl, une construction par architecture cible (TARGETARCH de buildx).
FROM rust:1-bookworm AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY tools ./tools
COPY xtask ./xtask
COPY migrations ./migrations
COPY openapi.json ./
COPY --from=ui /src/ui/build ./ui/build
ARG TARGETARCH
# Caches BuildKit : registre Cargo et répertoire `target` réutilisés d'un build à l'autre ;
# le binaire est copié hors du cache dans la même étape.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/src/target,sharing=locked \
    case "$TARGETARCH" in \
      amd64) target=x86_64-unknown-linux-musl ;; \
      arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "architecture non prise en charge : $TARGETARCH" >&2; exit 1 ;; \
    esac \
    && rustup target add "$target" \
    && CC_x86_64_unknown_linux_musl=musl-gcc \
       CC_aarch64_unknown_linux_musl=musl-gcc \
       cargo build --release --locked --target "$target" -p buntu-cli \
    && cp "target/$target/release/buntu" /usr/local/bin/buntu \
    && strip /usr/local/bin/buntu \
    && mkdir -p /out/data

# 3) Image finale distroless, non root (uid 65532), sans shell ni gestionnaire de paquets.
FROM gcr.io/distroless/static-debian12:nonroot
LABEL org.opencontainers.image.title="buntu" \
      org.opencontainers.image.description="Passerelle IA open source : une API compatible OpenAI devant de nombreux fournisseurs." \
      org.opencontainers.image.licenses="Apache-2.0" \
      org.opencontainers.image.source="https://github.com/LordPhenixDeNetra/buNtu"
COPY --from=build /usr/local/bin/buntu /usr/local/bin/buntu
# Données persistantes : base SQLite (journal, jetons, configuration de la console) et clé
# maître créée au premier démarrage (`buntu.sqlite.key`). Sans volume inscriptible, le
# stockage serait indisponible et l'API d'administration resterait fermée.
COPY --from=build --chown=65532:65532 /out/data /data
ENV BUNTU_DB_PATH=/data/buntu.sqlite
WORKDIR /data
VOLUME ["/data"]
EXPOSE 8080
# Sonde intégrée au binaire (`buntu health`) : l'image n'a ni shell ni `curl`. Si l'écoute est
# déplacée hors de 8080, passer l'adresse : `buntu health 127.0.0.1:<port>`.
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD ["/usr/local/bin/buntu", "health"]
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/buntu"]
CMD ["serve", "--listen", "0.0.0.0:8080"]
