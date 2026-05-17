# syntax=docker/dockerfile:1.6
#
# Multi-stage build for chaumstar:
#   1. rust-builder  → server binary + chaumstar-wasm pkg
#   2. web-builder   → static SvelteKit SPA (consumes the wasm pkg)
#   3. runtime       → debian-slim with binary + static assets

# ────────────────────────────────────────────────────────────
# Stage 1: Rust workspace + wasm-pack
# ────────────────────────────────────────────────────────────
FROM rust:1.85-slim AS rust-builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Install wasm-pack from the prebuilt binary (avoids needing a newer rustc to
# bootstrap wasm-pack's own dependency tree).
RUN curl -fsSL https://rustwasm.github.io/wasm-pack/installer/init.sh | sh

WORKDIR /app

# Cache cargo deps by copying only manifests first.
COPY Cargo.toml Cargo.lock ./
COPY crates/chaumstar-core/Cargo.toml ./crates/chaumstar-core/
COPY crates/chaumstar-server/Cargo.toml ./crates/chaumstar-server/
COPY crates/chaumstar-wasm/Cargo.toml ./crates/chaumstar-wasm/

# Stub sources so `cargo fetch` and a dummy build can prime the dependency cache.
RUN mkdir -p crates/chaumstar-core/src \
             crates/chaumstar-server/src \
             crates/chaumstar-wasm/src \
    && echo 'fn main() {}' > crates/chaumstar-server/src/main.rs \
    && echo '' > crates/chaumstar-core/src/lib.rs \
    && echo '' > crates/chaumstar-wasm/src/lib.rs \
    && cargo fetch

# Real sources.
COPY . .

RUN cargo build --release --bin chaumstar-server

RUN wasm-pack build crates/chaumstar-wasm --target web --out-dir pkg


# ────────────────────────────────────────────────────────────
# Stage 2: SvelteKit static build
# ────────────────────────────────────────────────────────────
FROM oven/bun:1.3 AS web-builder

WORKDIR /app

# Bring in the wasm pkg first so the `file:` dep resolves.
COPY --from=rust-builder /app/crates/chaumstar-wasm/pkg /app/crates/chaumstar-wasm/pkg

# Cache bun deps.
COPY web/package.json web/bun.lock /app/web/
RUN cd web && bun install --frozen-lockfile

COPY web /app/web
RUN cd web && bun run build


# ────────────────────────────────────────────────────────────
# Stage 3: Runtime
# ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system chaumstar \
    && useradd --system --gid chaumstar --no-create-home chaumstar

COPY --from=rust-builder /app/target/release/chaumstar-server /usr/local/bin/chaumstar-server
COPY --from=web-builder /app/web/build /var/lib/chaumstar/web

USER chaumstar

ENV CHAUMSTAR_BIND=0.0.0.0:8080
ENV CHAUMSTAR_STATIC_DIR=/var/lib/chaumstar/web
ENV RUST_LOG=chaumstar_server=info,tower_http=info

EXPOSE 8080

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/usr/local/bin/chaumstar-server"]
