# chaumstar

Anonymous, verifiable reviews from verified purchasers.
Built on BBS+ Anonymous Credentials over BLS12-381.

⚠️ **Early development. Not audited. Not for production use.**

## What is this

chaumstar is a protocol + reference implementation for publishing reviews where:

- A reviewer cryptographically proves "I am a verified purchaser blessed by issuer X"
- Neither the issuer nor the reader can link the review to a specific purchase event
- The same credential cannot be used twice
- The review text cannot be tampered with after publication
- No central platform decides which reviews are real

Built on **BBS+ Anonymous Credentials** over BLS12-381, following the IRTF
CFRG draft `draft-irtf-cfrg-bbs-signatures`.

Wire-format is currently **chaumstar/0.3**. The credential carries five signed
attributes: `hpk`, `merchant_id`, `issued_at` (always revealed), plus
`purchase_tier` and `product_category` which the reviewer can choose to
selectively disclose at publish time.
In this demo `issued_at` is always revealed for simplicity. A production design should coarsen or hide issuance time to reduce timing correlation.
`hpk` MUST be freshly generated per credential. It is not a long-term user identity key.

The Registry maintains an append-only RFC 6962 Merkle log of published
reviews and signs the tree head with a per-process Ed25519 key. Readers
fetch inclusion proofs alongside each review and verify locally — the
Registry cannot silently drop or rewrite a review without the receipt
becoming a proof of misbehaviour.

## Documents

| Doc | Content |
|---|---|
| [`PROTOCOL.md`](./PROTOCOL.md) | Protocol overview, actors, operations, threat model |
| [`CRYPTO.md`](./CRYPTO.md) | Cryptographic details (BBS+ blind issuance, presentation proof, JCS, DSTs) |
| [`DEMO.md`](./DEMO.md) | Demo scenario, screens, attack demos, tech stack |

## Repository layout

```
chaumstar/
├── crates/
│   ├── chaumstar-core/      Crypto + protocol types (BBS+, Ed25519, JCS)
│   ├── chaumstar-server/    Issuer + Registry HTTP server (single binary)
│   └── chaumstar-wasm/      Browser bindings (wallet + verifier)
├── web/                     SvelteKit frontend (TypeScript, JetBrains Mono)
├── Dockerfile               multi-stage build → single runtime image
├── fly.toml                 Fly.io app configuration
├── PROTOCOL.md / CRYPTO.md / DEMO.md
└── LICENSE                  MIT
```

## Architecture (one box, one binary)

```
[browser]
  │  HTTPS
  ▼
[chaumstar-server   axum binary]
  ├── /api/v1/health          GET
  ├── /api/v1/keysets         GET (list / fetch by kid)
  ├── /api/v1/mints           POST (blind sign)
  ├── /api/v1/reviews         GET / POST (publish + verify + nullifier)
  └── /                       SPA (HTML/CSS/JS + WASM)
        └── chaumstar-wasm    BBS+ blind issuance, proof gen, verification
```

Storage is in-memory; restart wipes state (issuer keys, registry, reviews).

## Local development

You need: Rust 1.85+, [bun](https://bun.sh), [wasm-pack](https://rustwasm.github.io/wasm-pack/).

Three things run in parallel during dev:

```bash
# 1. Build the WASM bindings once (or whenever chaumstar-wasm changes)
wasm-pack build crates/chaumstar-wasm --target web --out-dir pkg

# 2. Start the Rust server (API + later the static SPA)
cargo run --release --bin chaumstar-server
# → http://127.0.0.1:8080

# 3. Start the SvelteKit dev server (proxies /api to :8080)
cd web && bun install && bun run dev
# → http://localhost:5173
```

The dev UI lives at <http://localhost:5173/>. The full pipeline is:

- `/cafe/` — issuer view (keysets the server holds)
- `/alice/` — wallet (mint a credential, publish a review)
- `/bob/` — reader (fetch reviews, verify each in-browser via WASM)
- `/eve/` — attacker (run double-spend / tamper / forge against the live server)

## Tests

```bash
cargo test --workspace          # core + server, ~20 tests
cd web && bun run check         # svelte-check type-check
```

## Production build (single binary serves API + SPA)

```bash
# Build the WASM pkg + SvelteKit static SPA
wasm-pack build crates/chaumstar-wasm --target web --out-dir pkg
(cd web && bun install --frozen-lockfile && bun run build)

# Build the server binary
cargo build --release --bin chaumstar-server

# Run with CHAUMSTAR_STATIC_DIR pointing at web/build
CHAUMSTAR_BIND=127.0.0.1:8080 \
CHAUMSTAR_STATIC_DIR=$(pwd)/web/build \
  ./target/release/chaumstar-server
# → http://127.0.0.1:8080  (API + SPA on one origin)
```

## Docker

```bash
docker build -t chaumstar .
docker run --rm -p 8080:8080 chaumstar
```

The multi-stage Dockerfile compiles Rust + WASM, builds the SvelteKit static
SPA, and copies both into a `debian:bookworm-slim` runtime image (~150 MB).

## Deploy to Fly.io

```bash
# Once
flyctl auth login
flyctl apps create chaumstar     # or: flyctl launch --no-deploy

# Subsequent deploys
flyctl deploy
```

`fly.toml` sets `nrt` (Tokyo) as the primary region and lets the machine
auto-stop when idle (saves free-tier hours; cold start is ~3–5 s).

`fly.toml` also adds a `/api/v1/health` check so a wedged process gets noticed.

## Status

Pre-alpha demo. State persists only for the lifetime of a single server
process. Use [`PROTOCOL.md`](./PROTOCOL.md) §3 to understand what the protocol
does and does **not** guarantee.

## License

MIT. See [LICENSE](./LICENSE).
