# DotenvPP - Development Roadmap

> This roadmap tracks shipped implementation and remaining hardening work.
> Phase 0-6 source work is implemented in the current branch; publishing and external runtime validation are separate release tasks.

---

## Phase 0: Foundation ✅

> **Goal**: Ship the from-scratch `.env` parser foundation for common dotenv syntax.

- [x] Initialize Rust workspace with `cargo init --lib`
- [x] Set up workspace with subcrates: `dotenvpp-parser`, `dotenvpp-cli`, `dotenvpp` facade
- [x] Implement standard `.env` parser
- [x] Support single-quoted, double-quoted, and unquoted values
- [x] Handle multiline values
- [x] Implement `export KEY=VALUE` prefix support
- [x] Load parsed values into `std::env`
- [x] Write comprehensive parser test suite
- [x] Benchmark parser on representative workloads
- [x] Set up CI with clippy, fmt, and tests
- [x] Add release docs

**Exit Criteria**: Phase 0 was completed and published as `0.0.2`.

---

## Phase 1: Variable Interpolation & Environment Layering ✅

> **Goal**: Support `${VAR}` interpolation and multi-environment file loading.

- [x] Implement `${VAR}` basic interpolation
- [x] Implement `${VAR:-default}` and `${VAR-default}`
- [x] Implement `${VAR:?error}` and `${VAR?error}`
- [x] Implement `${VAR:+alternative}` and `${VAR+alternative}`
- [x] Implement `$$` escape for literal dollar sign
- [x] Implement environment layering (`.env` < `.env.{ENV}` < `.env.local` < `.env.{ENV}.local`)
- [x] Add `--env` / `-e` flag to CLI
- [x] Detect circular interpolation references and report clear errors
- [x] Test interpolation edge cases
- [x] Write integration tests for layered loading

**Exit Criteria**: Phase 1 was completed and published as `0.0.3`.

---

## Phase 2: Schema & Type System ✅

> **Goal**: `.env.schema` files that define types, defaults, and validation rules.

- [x] Design `.env.schema` TOML format
- [x] Implement schema parser
- [x] Implement core types: `string`, `bool`, `i32`, `i64`, `u16`, `u32`, `u64`, `f64`
- [x] Implement rich types: `url`, `email`, `ip`, `port`, `duration`, `datetime`, `path`
- [x] Implement `enum` type with allowed values
- [x] Implement array types (`string[]`, `i32[]`) with configurable separators
- [x] Implement `regex` pattern validation with `regex-lite`
- [x] Implement `range` constraints for numeric types
- [x] Implement `min_length` / `max_length` for strings
- [x] Implement `required` / `optional` semantics
- [x] Implement `default` value support
- [x] Implement `secret` marker for examples, reports, and redaction-aware workflows
- [x] Implement `description` field for documentation
- [x] Implement `dotenvpp check` schema validation
- [x] Implement `dotenvpp schema init`
- [x] Implement `dotenvpp schema example`
- [x] Implement `dotenvpp schema docs`
- [x] Implement `dotenvpp schema json-schema`
- [x] Create `#[derive(dotenvpp::Schema)]` proc macro for Rust structs
- [ ] Integrate `miette` for richer terminal diagnostics
- [x] Write validation test suite

**Exit Criteria**: A schema can validate `.env` content and generate `.env.example`, Markdown docs, and JSON Schema.

---

## Phase 3: Encryption ✅

> **Goal**: Encrypt/decrypt `.env` files with modern cryptography via swappable backends.

- [x] Set up `crypto-crabgraph` feature flag using `crabgraph` as the default backend
- [x] Set up `crypto-rustcrypto` feature flag using `aes-gcm`, `x25519-dalek`, `hkdf`, `rand_core`, and `zeroize`
- [x] Implement shared backend API behind `#[cfg(feature)]`
- [x] Implement X25519 keypair generation
- [x] Implement per-value AES-256-GCM encryption
- [x] Implement encrypted file format
- [x] Implement `dotenvpp encrypt`
- [x] Implement `dotenvpp decrypt` to stdout by default
- [x] Implement `dotenvpp keygen`
- [x] Implement `dotenvpp rotate` as backend-agnostic decrypt and re-encrypt
- [x] Implement multiple recipients support
- [x] Implement `DOTENV_PRIVATE_KEY` support for runtime decryption
- [x] Implement `dotenvpp run --encrypted`
- [x] Implement zeroizing secret byte buffers
- [x] Test tamper detection and encrypted roundtrips
- [x] Verify both backends with independent test runs
- [ ] Add cross-backend compatibility fixture tests in CI
- [ ] Test against memory dumps and core dumps
- [ ] Complete formal security audit checklist
- [ ] Optional: pluggable KMS feature flags
- [ ] Add cargo-fuzz/libFuzzer harness for `dotenvpp_parser::parse()`

**Exit Criteria**: Users can generate keys, encrypt for multiple recipients, decrypt at runtime, and rotate encrypted env files.

---

## Phase 4: Expression Language ✅

> **Goal**: Safe, sandboxed expressions for computed configuration values.

- [x] Design expression language grammar
- [x] Implement expression parser with recursive descent
- [x] Implement arithmetic operations: `+`, `-`, `*`, `/`, `%`
- [x] Implement comparison operators: `==`, `!=`, `<`, `>`, `<=`, `>=`
- [x] Implement logical operators: `&&`, `||`, `!`
- [x] Implement implication operator: `=>`
- [x] Implement string concatenation
- [x] Implement `if/then/else` conditional expressions
- [x] Implement string built-ins: `len`, `upper`, `lower`, `trim`, `contains`, `starts_with`, `ends_with`, `concat`
- [x] Implement crypto/encoding/time built-ins: `sha256`, `base64_encode`, `base64_decode`, `uuid`, `now`, `duration`
- [x] Implement gated `file()` with strict root constraints
- [x] Implement gated `env()` access
- [x] Implement sandbox defaults: no I/O, no loops, bounded recursion and expression length
- [x] Implement deterministic evaluation tracking
- [x] Test expression edge cases and sandbox boundaries
- [x] Document expression language examples in README and architecture docs

**Exit Criteria**: DotenvPP can compute values such as `MAX_WORKERS = ${CPU_COUNT} * 2` safely.

---

## Phase 5: Policy Engine ✅

> **Goal**: Declarative policy rules that constrain valid configurations.

- [x] Design `.env.policy` TOML format
- [x] Implement policy parser
- [x] Implement condition evaluator using the expression engine
- [x] Implement severity levels: `error`, `warning`, `info`
- [x] Implement `dotenvpp check --strict`
- [x] Implement `dotenvpp lint`
- [x] Create standard policy library
- [x] Standard policy: no debug in production
- [x] Standard policy: SSL required for PostgreSQL in production
- [x] Standard policy: no localhost URLs outside development
- [x] Standard policy: no obvious default credentials
- [ ] Standard policy: schema-aware minimum length for all secret-marked variables
- [x] Test policy engine with rule combinations
- [x] Document policy examples in README and architecture docs

**Exit Criteria**: DotenvPP can enforce "no debug logging in production" as a policy rule.

---

## Phase 6: WASM Target ✅

> **Goal**: Compile core functionality to WASM for browser and JavaScript runtimes.

- [x] Configure `wasm-pack` build
- [x] Create `dotenvpp-wasm` package with `wasm-bindgen` bindings
- [x] Expose parsing API to JavaScript
- [x] Expose schema validation API to JavaScript
- [x] Expose policy checking API to JavaScript
- [x] Create generated package metadata for `@dotenvpp/wasm`
- [x] Build browser playground source
- [x] Test in Node.js
- [x] Test in Bun using the web-target initializer
- [ ] Test in Deno after Deno is available in the local/CI environment
- [ ] Add automated browser test matrix
- [ ] Test in edge runtimes such as Cloudflare Workers and Vercel Edge
- [x] Optimize WASM binary size below 200KB gzipped (`196,149` bytes)
- [ ] Create standalone WASI package/runtime
- [ ] Add cargo-fuzz/libFuzzer harness for `dotenvpp_parser::parse()`

**Exit Criteria**: The local `@dotenvpp/wasm` package can parse, validate schemas, and check policies from JavaScript. Public npm installation remains a release task.

---

## Phase 7: DX & Ecosystem

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

## Phase 8: Advanced Features

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

| Feature | Impact | Effort | Priority | Status |
|---|---|---:|---|---|
| Standard `.env` parsing | High | Low | P0 | Done |
| Variable interpolation | High | Medium | P0 | Done |
| Environment layering | High | Low | P0 | Done |
| Schema & type system | Very High | High | P1 | Done |
| Encryption | Very High | High | P1 | Done |
| Expression language | High | High | P2 | Done |
| Policy engine | High | Medium | P2 | Done |
| WASM target | High | Medium | P2 | Done locally |
| VS Code extension | Medium | High | P3 | Planned |
| Multi-language bindings | Medium | High | P3 | Planned |

---

*Last updated: April 18, 2026*
