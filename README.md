<p align="center">
  <h1 align="center">DotenvPP</h1>
  <p align="center"><strong>Dotenv, but typed, layered, encrypted, policy-aware, and WASM-ready.</strong></p>
  <p align="center">
    <em>This workspace contains the Phase 0-6 implementation. Publishing is handled separately from this source update.</em>
  </p>
</p>

<p align="center">
  <a href="https://crates.io/crates/dotenvpp"><img src="https://img.shields.io/crates/v/dotenvpp?color=171717" alt="Crates.io Version" /></a>
  <a href="https://crates.io/crates/dotenvpp"><img src="https://img.shields.io/crates/d/dotenvpp?color=171717" alt="Crates.io Downloads" /></a>
  <a href="https://docs.rs/dotenvpp"><img src="https://img.shields.io/docsrs/dotenvpp?color=171717" alt="docs.rs" /></a>
  <a href="https://github.com/Carabryx/DotenvPP/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Carabryx/DotenvPP/ci.yml?label=CI&color=171717" alt="CI" /></a>
  <a href="https://github.com/Carabryx/DotenvPP/releases/latest"><img src="https://img.shields.io/github/v/release/Carabryx/DotenvPP?label=release&color=171717" alt="Latest release" /></a>
</p>

<p align="center">
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#schema">Schema</a> •
  <a href="#encryption">Encryption</a> •
  <a href="#wasm">WASM</a> •
  <a href="#workspace">Workspace</a>
</p>

---

## Features

DotenvPP owns the parser and builds higher-level configuration workflows on top of it.

| Capability | Status |
|---|---|
| Standard `.env` parsing with comments, quotes, multiline values, `export`, BOM handling, and escape decoding | Implemented |
| Variable interpolation with `${VAR}`, defaults, required values, alternatives, `$$`, and cycle detection | Implemented |
| Environment layering: `.env` < `.env.{ENV}` < `.env.local` < `.env.{ENV}.local` | Implemented |
| `.env.schema` TOML type system, validation, defaults, docs, examples, JSON Schema export, and schema inference | Implemented |
| `#[derive(dotenvpp::Schema)]` for Rust structs | Implemented |
| Encrypted `.env` JSON format with X25519 recipients and AES-256-GCM per-value encryption | Implemented |
| Default CrabGraph crypto backend and opt-in RustCrypto backend | Implemented |
| Safe expression evaluator for computed configuration | Implemented |
| `.env.policy` rules with severity levels and a standard security policy library | Implemented |
| WASM package for parse, schema validation, and policy checks | Implemented |
| Browser playground source for the WASM package | Implemented |

Remaining hardening work is tracked in [docs/TODO.md](docs/TODO.md): external audit work, KMS providers, full browser/edge matrix automation, and standalone WASI packaging are intentionally not marked as shipped.

---

## Quick Start

From this checkout:

```bash
cargo install --path crates/dotenvpp-cli
dotenvpp check --file .env
dotenvpp check --env production --strict
dotenvpp run --env production -- cargo test
```

Or run without installing:

```bash
cargo run -p dotenvpp-cli -- check --file .env
cargo run -p dotenvpp-cli -- schema init --file .env --output .env.schema
```

### Rust

```rust
fn main() -> Result<(), dotenvpp::Error> {
    let values = dotenvpp::from_read_evaluated(
        &b"CPU_COUNT=4\nMAX_WORKERS=${CPU_COUNT} * 2\n"[..],
    )?;

    assert_eq!(values[1].value, "8");
    Ok(())
}
```

Typed schemas can be declared in TOML or derived from Rust structs:

```rust
#[derive(dotenvpp::Schema)]
struct AppConfig {
    #[env(required, description = "HTTP port")]
    port: u16,

    #[env(default = "info", values = ["trace", "debug", "info", "warn", "error"])]
    log_level: String,
}
```

---

## Schema

`.env.schema` files use TOML:

```toml
[vars.PORT]
type = "port"
required = true
description = "HTTP server port"

[vars.LOG_LEVEL]
type = "enum"
values = ["trace", "debug", "info", "warn", "error"]
default = "info"

[vars.API_KEY]
type = "string"
required = true
secret = true
min_length = 32
```

Useful commands:

```bash
dotenvpp check --file .env --schema .env.schema
dotenvpp schema init --file .env --output .env.schema
dotenvpp schema example --schema .env.schema --output .env.example
dotenvpp schema docs --schema .env.schema --output CONFIG.md
dotenvpp schema json-schema --schema .env.schema --output env.schema.json
```

Supported schema types: `string`, `bool`, `i32`, `i64`, `u16`, `u32`, `u64`, `f64`, `url`, `email`, `ip`, `port`, `duration`, `datetime`, `regex`, `path`, `enum`, `string[]`, and `i32[]`.

---

## Policies

`.env.policy` files define violation predicates. When a rule condition evaluates to `true`, DotenvPP reports it.

```toml
[[rules]]
name = "no-debug-in-prod"
description = "Debug logging is forbidden in production"
condition = "ENV == 'production' && LOG_LEVEL == 'debug'"
severity = "error"
```

```bash
dotenvpp lint --file .env --policy .env.policy
dotenvpp check --file .env --schema .env.schema --policy .env.policy --strict
```

The standard policy library checks common production mistakes such as debug logging, missing PostgreSQL SSL mode, localhost URLs outside development, and obvious default credentials.

---

## Expressions

Computed configuration uses a bounded, sandboxed recursive-descent evaluator:

```env
CPU_COUNT=4
MAX_WORKERS=${CPU_COUNT} * 2
LOG_LEVEL=if ENV == "production" then "warn" else "debug"
SECRET_HASH=sha256(RAW_SECRET)
```

Implemented operators include arithmetic, comparison, logical `&&`/`||`/`!`, implication `=>`, string concatenation via `+` or `concat()`, and `if/then/else`.

Built-ins include `len`, `upper`, `lower`, `trim`, `contains`, `starts_with`, `ends_with`, `concat`, `sha256`, `base64_encode`, `base64_decode`, `duration`, `uuid`, `now`, `env`, and `file`. `env()` and `file()` are disabled unless explicitly enabled through `EvalOptions`.

---

## Encryption

The default feature set uses CrabGraph:

```bash
dotenvpp keygen
dotenvpp encrypt --file .env --recipient "$DOTENV_PUBLIC_KEY" --output .env.enc
dotenvpp decrypt --file .env.enc --private-key "$DOTENV_PRIVATE_KEY"
dotenvpp rotate --file .env.enc --private-key "$OLD_PRIVATE_KEY" --recipient "$NEW_PUBLIC_KEY"
dotenvpp run --encrypted .env.enc --private-key "$DOTENV_PRIVATE_KEY" -- node server.js
```

The encrypted format uses:

- X25519 key agreement for recipient wrapping
- HKDF-SHA256 derived wrap keys
- AES-256-GCM authenticated encryption
- per-variable encryption with the variable name as associated data
- zeroizing wrappers for secret byte buffers
- multiple recipients

Use RustCrypto directly instead of CrabGraph:

```bash
cargo test -p dotenvpp-crypto --no-default-features --features crypto-rustcrypto
```

---

## WASM

The WASM package exposes parse, schema validation, and policy checks:

```bash
cd crates/dotenvpp-wasm
npm run build:all
npm run smoke:node
npm run smoke:bun
```

Node usage:

```javascript
import { parse, validate, checkPolicy } from "./pkg/dotenvpp_wasm.js";

const parsed = JSON.parse(parse("PORT=8080\n"));
const report = JSON.parse(validate("PORT=8080\n", `[vars.PORT]\ntype = "port"\n`));
```

Browser playground:

```bash
cd crates/dotenvpp-wasm
npm run build:web
python -m http.server 8080
```

Open `http://localhost:8080/playground/`.

Current optimized WASM size for both bundler and web targets is `494,069` bytes raw and `196,149` bytes gzipped.

---

## Workspace

```text
dotenvpp/
├── crates/
│   ├── dotenvpp-parser/    # .env parser
│   ├── dotenvpp-schema/    # .env.schema parser, validator, docs, JSON Schema
│   ├── dotenvpp-expr/      # sandboxed expression language
│   ├── dotenvpp-policy/    # .env.policy parser and evaluator
│   ├── dotenvpp-crypto/    # encrypted file format and crypto backends
│   ├── dotenvpp-macros/    # #[derive(Schema)]
│   ├── dotenvpp-wasm/      # wasm-bindgen package and playground
│   └── dotenvpp-cli/       # dotenvpp binary
├── src/lib.rs              # facade crate API
├── tests/                  # facade integration tests
├── docs/                   # research, architecture, roadmap
└── complete.md             # Phase 2-6 implementation log for this branch
```

---

## Quality

Primary verification commands:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test -p dotenvpp-crypto --no-default-features --features crypto-rustcrypto
cd crates/dotenvpp-wasm && npm run build:all && npm run smoke:node && npm run smoke:bun
```

---

## Research

The implementation is informed by [docs/RESEARCH.md](docs/RESEARCH.md), local CrabGraph API verification, and upstream documentation for CrabGraph, RustCrypto AES-GCM, wasm-bindgen, wasm-pack, and regex-lite.
