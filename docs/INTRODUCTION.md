# Introduction

DotenvPP is a from-scratch `.env` toolkit for Rust. It remains compatible with common dotenv syntax, but the workspace now includes typed schemas, encrypted env files, computed expressions, policy checks, and WASM bindings.

## Current Workspace

This branch contains the Phase 0-6 implementation:

- `.env` parsing with comments, blank lines, `export`, quotes, multiline values, BOM handling, and escape decoding
- `${VAR}` interpolation with default, required, alternative, and literal-dollar handling
- layered loading for `.env`, `.env.{ENV}`, `.env.local`, and `.env.{ENV}.local`
- `.env.schema` TOML parsing, validation, defaults, generated examples, Markdown docs, JSON Schema export, and schema inference
- `#[derive(dotenvpp::Schema)]` for Rust structs
- X25519/AES-256-GCM encrypted env files with CrabGraph as the default backend and RustCrypto as an opt-in backend
- sandboxed computed expressions with deterministic tracking and opt-in `env()` / `file()` access
- `.env.policy` files with `error`, `warning`, and `info` severities
- CLI commands for check, run, schema generation, linting, keygen, encrypt, decrypt, and rotate
- `dotenvpp-wasm` bindings for JavaScript parse, validate, and policy checks
- a browser playground source wired to the generated web WASM package

Publishing to crates.io/npm, full browser/edge automation, KMS integrations, formal security review, and standalone WASI packaging remain separate release/hardening tasks tracked in [TODO.md](TODO.md).

## Why This Direction

DotenvPP is intentionally not a wrapper around `dotenvy` or another env crate. Owning the parser and configuration model lets the project add validation, policy, encryption, and WASM support without inheriting another crate's syntax or architecture limits.

## Fast Path

```bash
cargo install --path crates/dotenvpp-cli
dotenvpp check --file .env --schema .env.schema
dotenvpp lint --file .env --policy .env.policy
dotenvpp run --env production -- cargo test
```

For WASM:

```bash
cd crates/dotenvpp-wasm
npm run build:all
npm run smoke:node
npm run smoke:bun
```
