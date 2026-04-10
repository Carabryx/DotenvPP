# Introduction

DotenvPP is a from-scratch `.env` toolkit for Rust. It is a special superset of the familiar file format: it understands common `.env` syntax, but it is not a wrapper around `dotenvy` or any other parser.

## Current Release

Phase 1 extends the parser foundation with interpolation and layered loading:

- `KEY=VALUE` parsing
- Comments, blank lines, and `export` prefixes
- Single-quoted, double-quoted, and unquoted values
- Multiline quoted values, BOM handling, and common escape sequences
- `${VAR}` interpolation with default, required, and alternative operators
- Circular reference detection with source-aware errors
- Layered loading for `.env`, `.env.{ENV}`, `.env.local`, and `.env.{ENV}.local`
- `dotenvpp` facade helpers and CLI commands with `--env` / `-e` support

Roadmap items described elsewhere in the repository remain design targets for later phases; the currently shipped API covers parsing, interpolation, and layered loading.

## Why This Direction

DotenvPP is intentionally not "just another env-user crate". The goal is to own the parsing and configuration surface from the ground up so the project can grow into typed config, policies, and other higher-level features without inheriting someone else's design limits.

## What Comes Next

- Phase 2: schema and type system
- Phase 3: encryption
- Phase 4: expressions
- Phase 5: policies
- Phase 6: WASM
- Phase 7: DX and ecosystem tooling
- Phase 8: advanced features
