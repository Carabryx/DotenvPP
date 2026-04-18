# DotenvPP Architecture

> **Status**: Implemented through Phase 6 in this branch.
> **Language**: Rust 2021.
> **Targets**: Native Rust/CLI plus `wasm-bindgen` browser, bundler, Node, and Bun paths.

---

## 1. System Shape

DotenvPP is a workspace of focused crates behind the root `dotenvpp` facade:

```text
                  .env files
                      |
                      v
              dotenvpp-parser
                      |
                      v
       interpolation and layered loading
                      |
      +---------------+----------------+
      |               |                |
      v               v                v
dotenvpp-schema  dotenvpp-expr   dotenvpp-policy
      |               |                |
      +---------------+----------------+
                      |
                      v
             root dotenvpp facade
                      |
      +---------------+----------------+
      |                                |
      v                                v
dotenvpp-cli                    dotenvpp-wasm

dotenvpp-crypto is optional on the facade and enabled by default through
the `crypto-crabgraph` feature.
```

The parser remains the foundation. Schema validation, policy evaluation, and encrypted loading all consume parsed `EnvPair` values rather than reparsing strings independently.

---

## 2. Crate Boundaries

| Crate | Responsibility |
|---|---|
| `dotenvpp-parser` | Common dotenv parser: comments, quotes, multiline values, `export`, BOM, escape handling |
| `dotenvpp` | Facade: loading, interpolation, layering, schema/policy helpers, optional crypto helpers |
| `dotenvpp-schema` | `.env.schema` TOML model, type validation, defaults, docs, examples, JSON Schema, inference |
| `dotenvpp-expr` | Sandboxed recursive-descent expression evaluator |
| `dotenvpp-policy` | `.env.policy` parser and expression-backed rule evaluation |
| `dotenvpp-crypto` | Encrypted env format, key generation, encryption, decryption, rotation, backend selection |
| `dotenvpp-macros` | `#[derive(Schema)]` proc macro |
| `dotenvpp-cli` | User-facing `dotenvpp` binary |
| `dotenvpp-wasm` | `wasm-bindgen` exports and browser playground source |

---

## 3. Parser And Loading

The parser produces flat `EnvPair { key, value, line }` records. The facade adds:

- `${VAR}` interpolation
- `${VAR:-default}`, `${VAR-default}`
- `${VAR:?message}`, `${VAR?message}`
- `${VAR:+alternative}`, `${VAR+alternative}`
- `$$` literal dollar escaping
- cycle detection across interpolated values
- process environment fallback on native targets
- empty process-environment snapshot on `wasm32` so WASM parsing does not trap
- layered file loading with later files overriding earlier files

Layer precedence:

```text
.env
.env.{ENV}
.env.local
.env.{ENV}.local
```

---

## 4. Schema

Schemas are TOML documents:

```toml
[vars.PORT]
type = "port"
required = true
description = "HTTP server port"

[vars.LOG_LEVEL]
type = "enum"
values = ["trace", "debug", "info", "warn", "error"]
default = "info"
```

Implemented schema features:

- primitive types: `string`, `bool`, `i32`, `i64`, `u16`, `u32`, `u64`, `f64`
- rich types: `url`, `email`, `ip`, `port`, `duration`, `datetime`, `regex`, `path`
- `enum`, `string[]`, and `i32[]`
- `required`, `default`, `secret`, `description`
- numeric `range`
- `min_length` and `max_length`
- regex `pattern`
- URL `protocols`
- duration `min` and `max`
- generated `.env.example`
- generated Markdown docs
- generated JSON Schema
- schema inference from existing env files

The schema crate uses `regex-lite` for runtime pattern validation and a conservative built-in URL shape validator to keep the WASM build small.

---

## 5. Expressions

The expression engine is a bounded recursive-descent parser and evaluator. It has no loops and no user-defined functions.

Implemented syntax:

- arithmetic: `+`, `-`, `*`, `/`, `%`
- comparisons: `==`, `!=`, `<`, `>`, `<=`, `>=`
- logic: `&&`, `||`, `!`
- implication: `=>`
- conditionals: `if ... then ... else ...`
- variables: `${VAR}`, `$VAR`, or bare identifiers
- string literals with single or double quotes

Implemented built-ins:

```text
len, upper, lower, trim, contains, starts_with, ends_with, concat,
sha256, base64_encode, base64_decode, duration, uuid, now, env, file
```

`env()` and `file()` are disabled by default and require explicit `EvalOptions`. `uuid()`, `now()`, `env()`, and `file()` mark an evaluation as non-deterministic.

`uuid()` is a UUID-shaped uniqueness helper for computed config identifiers. It is not used for secrets or cryptographic keys.

---

## 6. Policies

Policies are TOML documents. A rule condition is a violation predicate: if the expression evaluates to true, a violation is reported.

```toml
[[rules]]
name = "no-debug-in-prod"
description = "Debug logging is forbidden in production"
condition = "ENV == 'production' && LOG_LEVEL == 'debug'"
severity = "error"
```

The standard security policy library currently checks:

- debug logging in production
- PostgreSQL production URLs without `sslmode=require`
- localhost URLs outside development
- obvious default credentials

---

## 7. Crypto

`dotenvpp-crypto` exposes one encrypted format with two compile-time backend choices:

```toml
[features]
default = ["crypto-crabgraph"]
crypto-crabgraph = ["crabgraph"]
crypto-rustcrypto = ["aes-gcm", "x25519-dalek", "hkdf", "rand_core", "zeroize"]
```

Exactly one backend must be selected.

Format and flow:

- JSON envelope versioned as `dotenvpp.enc.v1`
- X25519 keypairs encoded as base64
- random data key per encrypted env file
- data key wrapped for one or more recipients using X25519 shared secret plus HKDF-SHA256
- each variable value encrypted independently with AES-256-GCM
- variable name used as associated data
- secret byte buffers zeroized on drop
- rotation implemented as decrypt and re-encrypt for a new recipient set

The default backend uses the local CrabGraph API verified from `C:\all\Carabryx\crabgraph`. The RustCrypto backend is tested separately with `cargo test -p dotenvpp-crypto --no-default-features --features crypto-rustcrypto`.

---

## 8. CLI

Implemented command surface:

```bash
dotenvpp check --file .env --schema .env.schema --policy .env.policy --strict
dotenvpp check --env production
dotenvpp run --env production -- cargo test
dotenvpp run --encrypted .env.enc --private-key "$DOTENV_PRIVATE_KEY" -- node server.js
dotenvpp lint --file .env --policy .env.policy
dotenvpp schema init --file .env --output .env.schema
dotenvpp schema example --schema .env.schema --output .env.example
dotenvpp schema docs --schema .env.schema --output CONFIG.md
dotenvpp schema json-schema --schema .env.schema --output env.schema.json
dotenvpp keygen
dotenvpp encrypt --file .env --recipient "$DOTENV_PUBLIC_KEY" --output .env.enc
dotenvpp decrypt --file .env.enc --private-key "$DOTENV_PRIVATE_KEY"
dotenvpp rotate --file .env.enc --private-key "$OLD_PRIVATE_KEY" --recipient "$NEW_PUBLIC_KEY"
```

---

## 9. WASM

`dotenvpp-wasm` exports:

- `version()`
- `parse(envContent)`
- `validate(envContent, schemaContent)`
- `checkPolicy(envContent, policyContent)`

Builds:

```bash
cd crates/dotenvpp-wasm
npm run build
npm run build:web
npm run smoke:node
npm run smoke:bun
```

The generated bundler and web artifacts are currently `494,069` bytes raw and `196,149` bytes gzipped. The WASM boundary uses explicit JSON serializers for the fixed parse/report payloads to avoid linking generic JSON serialization into the package.

The playground source lives at `crates/dotenvpp-wasm/playground/index.html` and uses the `pkg-web` output.

---

## 10. Quality Gates

Primary verification:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
cargo test -p dotenvpp-crypto --no-default-features --features crypto-rustcrypto
cd crates/dotenvpp-wasm && npm run build:all && npm run smoke:node && npm run smoke:bun
```

Known non-shipped architecture items are tracked in [TODO.md](TODO.md): KMS integrations, formal security audit, cargo-fuzz/libFuzzer harnesses, full Deno/browser/edge automation, and standalone WASI packaging.
