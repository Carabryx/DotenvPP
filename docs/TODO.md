# DotenvPP — Development Roadmap

> This is the phased development plan for DotenvPP.
> Each phase builds on the previous. Ship each phase as a usable release.

---

## Phase 0: Foundation 🏗️

> **Goal**: Working Rust project with basic `.env` parsing that matches existing dotenv behavior.

- [ ] Initialize Rust workspace with `cargo init --lib`
- [ ] Set up workspace with subcrates: `dotenvpp-parser`, `dotenvpp-cli`, `dotenvpp` (facade)
- [ ] Implement standard `.env` parser (KEY=VALUE, comments, blank lines, quotes)
- [ ] Support single-quoted, double-quoted, and unquoted values
- [ ] Handle multiline values (both `\n` escapes and actual multilines in double quotes)
- [ ] Implement `export KEY=VALUE` prefix support
- [ ] Load parsed values into `std::env`
- [ ] Write comprehensive parser test suite (100+ test cases)
- [ ] Benchmark parser against `dotenvy` (Rust) and `dotenv` (Node.js)
- [ ] Set up CI (GitHub Actions) with clippy, fmt, test
- [ ] Publish initial crate structure to crates.io (reserve names)

**Exit Criteria**: `dotenvpp` is a drop-in replacement for `dotenvy`.

---

## Phase 1: Variable Interpolation & Environment Layering 🔗

> **Goal**: Support `${VAR}` interpolation and multi-environment file loading.

- [ ] Implement `${VAR}` basic interpolation
- [ ] Implement `${VAR:-default}` (default if empty/unset)
- [ ] Implement `${VAR:?error}` (required with error message)
- [ ] Implement `${VAR:+alternative}` (alternative if set)
- [ ] Implement `$$` escape for literal dollar sign
- [ ] Implement environment layering (`.env` < `.env.local` < `.env.{ENV}` < `.env.{ENV}.local`)
- [ ] Add `--env` / `-e` flag to CLI for environment selection
- [ ] Detect circular interpolation references and report clear errors
- [ ] Test interpolation edge cases (nested, recursive, missing vars)
- [ ] Write integration tests for layered loading

**Exit Criteria**: Full interpolation parity with dotenvx + layered environments.

---

## Phase 2: Schema & Type System 📐

> **Goal**: `.env.schema` files that define types, defaults, validation rules.

- [ ] Design `.env.schema` TOML format specification
- [ ] Implement schema parser
- [ ] Implement core types: `string`, `bool`, `i32`, `i64`, `u16`, `u32`, `u64`, `f64`
- [ ] Implement rich types: `url`, `email`, `ip`, `port`, `duration`, `datetime`, `path`
- [ ] Implement `enum` type with allowed values
- [ ] Implement array types (`string[]`, `i32[]`) with configurable separators
- [ ] Implement `regex` pattern validation
- [ ] Implement `range` constraints for numeric types
- [ ] Implement `min_length` / `max_length` for strings
- [ ] Implement `required` / `optional` semantics
- [ ] Implement `default` value support
- [ ] Implement `secret` marker (affects logging, example generation, leak detection)
- [ ] Implement `description` field for documentation
- [ ] Implement `dotenvpp check` — validate `.env` against schema
- [ ] Implement `dotenvpp schema init` — generate schema from existing `.env`
- [ ] Implement `dotenvpp schema example` — generate `.env.example` from schema
- [ ] Implement `dotenvpp schema docs` — generate markdown documentation
- [ ] Implement `dotenvpp schema json-schema` — export as JSON Schema
- [ ] Create `#[derive(dotenvpp::Schema)]` proc macro for Rust structs
- [ ] Integrate `miette` for beautiful, actionable error messages
- [ ] Write validation test suite

**Exit Criteria**: Can define a schema, validate against it, and auto-generate `.env.example` + docs.

---

## Phase 3: Encryption 🔒

> **Goal**: Encrypt/decrypt `.env` files with modern cryptography.

- [ ] Implement X25519 keypair generation
- [ ] Implement per-value AES-256-GCM encryption
- [ ] Implement encrypted file format (`.env.enc` or inline encrypted values)
- [ ] Implement `dotenvpp encrypt` command
- [ ] Implement `dotenvpp decrypt` command (to stdout, never to disk by default)
- [ ] Implement `dotenvpp keygen` — generate new keypair
- [ ] Implement `dotenvpp rotate` — re-encrypt with new keys
- [ ] Implement multiple recipients support
- [ ] Implement `DOTENV_PRIVATE_KEY` env var for runtime decryption
- [ ] Implement `dotenvpp run -- <command>` — decrypt + inject + run
- [ ] Implement `zeroize` for all secret values in memory
- [ ] Test against known attack vectors (memory dumps, core dumps)
- [ ] Security audit checklist
- [ ] Optional: pluggable KMS trait (AWS KMS, GCP KMS, Azure Key Vault)

**Exit Criteria**: Encrypted `.env` files safe to commit to git, runtime decryption works.

---

## Phase 4: Expression Language 🧮

> **Goal**: Safe, sandboxed expressions for computed configuration values.

- [ ] Design expression language grammar
- [ ] Implement expression parser (recursive descent)
- [ ] Implement arithmetic operations: `+`, `-`, `*`, `/`, `%`
- [ ] Implement comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- [ ] Implement logical operators: `&&`, `||`, `!`
- [ ] Implement string concatenation and interpolation
- [ ] Implement `if/then/else` conditional expressions
- [ ] Implement built-in functions:
  - [ ] `len()`, `upper()`, `lower()`, `trim()`, `contains()`, `starts_with()`, `ends_with()`
  - [ ] `sha256()`, `base64_encode()`, `base64_decode()`
  - [ ] `uuid()`, `now()`, `duration()`
  - [ ] `file()` — read file contents (with strict path constraints)
  - [ ] `env()` — reference OS environment
- [ ] Implement sandbox: no I/O, no loops, bounded recursion
- [ ] Implement deterministic evaluation tracking
- [ ] Test expression edge cases and security boundaries
- [ ] Document expression language with examples

**Exit Criteria**: Can compute `MAX_WORKERS = ${CPU_COUNT} * 2` safely.

---

## Phase 5: Policy Engine 📋

> **Goal**: Declarative policy rules that constrain valid configurations.

- [ ] Design `.env.policy` TOML format
- [ ] Implement policy parser
- [ ] Implement condition evaluator (reuse expression engine)
- [ ] Implement severity levels: `error`, `warning`, `info`
- [ ] Implement `dotenvpp check --strict` (enforce policies)
- [ ] Implement `dotenvpp lint` (report all policy violations)
- [ ] Create standard policy library (common security rules):
  - [ ] No debug in production
  - [ ] SSL required for databases in production
  - [ ] Minimum secret length
  - [ ] No localhost URLs in non-development environments
  - [ ] No default/weak credentials
- [ ] Test policy engine with various rule combinations
- [ ] Document policy writing guide

**Exit Criteria**: Can enforce "no debug logging in production" as a policy rule.

---

## Phase 6: WASM Target 🌐

> **Goal**: Compile core functionality to WASM for browser, edge, and WASI.

- [ ] Configure `wasm-pack` build
- [ ] Create `dotenvpp-wasm` package with `wasm-bindgen` bindings
- [ ] Expose parsing API to JavaScript
- [ ] Expose schema validation API to JavaScript
- [ ] Expose policy checking API to JavaScript
- [ ] Create npm package `@dotenvpp/wasm`
- [ ] Build interactive online playground (HTML + WASM)
- [ ] Test in Node.js, Deno, Bun, and browser environments
- [ ] Test in edge runtimes (Cloudflare Workers, Vercel Edge)
- [ ] Optimize WASM binary size (target < 200KB gzipped)
- [ ] Create WASI target for standalone runtime

**Exit Criteria**: `npm install @dotenvpp/wasm` works and can validate schemas in browser.

---

## Phase 7: DX & Ecosystem 🛠️

> **Goal**: Developer experience, IDE support, and community ecosystem.

- [ ] Create VS Code extension:
  - [ ] Syntax highlighting for `.env`, `.env.schema`, `.env.policy`
  - [ ] Schema-aware autocompletion
  - [ ] Hover documentation from schema descriptions
  - [ ] Inline error diagnostics
  - [ ] Secret value masking
- [ ] Create `dotenvpp diff` command
- [ ] Create `dotenvpp audit` command (scan for leaked secrets, weak values)
- [ ] Create git pre-commit hook for leak prevention
- [ ] Create GitHub Action for CI validation
- [ ] Generate multi-language bindings via C FFI:
  - [ ] Python (`pip install dotenvpp`)
  - [ ] Node.js (`npm install dotenvpp`)
  - [ ] Go
  - [ ] Ruby
- [ ] Write comprehensive documentation site
- [ ] Create migration guides from dotenv, dotenvx, docker-compose env
- [ ] Create starter templates for popular frameworks (Next.js, Rails, Django, etc.)

**Exit Criteria**: Full ecosystem ready for broad adoption.

---

## Phase 8: Advanced Features 🚀

> **Goal**: Features for enterprise and advanced use cases.

- [ ] Remote config sources (fetch from URLs, APIs)
- [ ] Secret rotation automation
- [ ] Config diff and change tracking history
- [ ] Multi-tenant / namespace support
- [ ] Config inheritance across projects (shared base configs)
- [ ] Integration with infrastructure tools (Terraform, Pulumi, Kubernetes)
- [ ] Telemetry-safe config printing (auto-redact secrets in logs)
- [ ] Performance profiling and optimization pass
- [ ] Formal security audit by external party

---

## Priority Matrix

| Feature | Impact | Effort | Priority |
|---|---|---|---|
| Standard .env parsing | High | Low | P0 |
| Variable interpolation | High | Medium | P0 |
| Environment layering | High | Low | P0 |
| Schema & type system | Very High | High | P1 |
| Encryption | Very High | High | P1 |
| Expression language | High | High | P2 |
| Policy engine | High | Medium | P2 |
| WASM target | High | Medium | P2 |
| CLI tool | High | Medium | P1 |
| VS Code extension | Medium | High | P3 |
| Multi-language bindings | Medium | High | P3 |

---

*Last updated: March 2026*
