# syntax=docker/dockerfile:1

# 1) Console web statique (nécessaire à `rust-embed`).
FROM node:22-bookworm AS ui
WORKDIR /src
COPY ui/package.json ui/package-lock.json ./ui/
RUN cd ui && npm ci --no-audit --no-fund
COPY openapi.json ./openapi.json
COPY ui ./ui
RUN cd ui && npm run build

# 2) Binaire statique musl (multi-architecture via TARGETARCH de buildx).
FROM rust:1-bookworm AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends musl-tools pkg-config ca-certificates \
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
RUN case "$TARGETARCH" in \
      amd64) target=x86_64-unknown-linux-musl ;; \
      arm64) target=aarch64-unknown-linux-musl ;; \
      *) echo "architecture non prise en charge : $TARGETARCH" >&2; exit 1 ;; \
    esac \
    && rustup target add "$target" \
    && CC_x86_64_unknown_linux_musl=musl-gcc \
       CC_aarch64_unknown_linux_musl=musl-gcc \
       cargo build --release --locked --target "$target" -p buntu-cli \
    && cp "target/$target/release/buntu" /usr/local/bin/buntu \
    && strip /usr/local/bin/buntu

# 3) Image finale distroless, non root.
FROM gcr.io/distroless/static-debian12:nonroot
COPY --from=build /usr/local/bin/buntu /usr/local/bin/buntu
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/buntu"]
CMD ["serve", "--listen", "0.0.0.0:8080"]
