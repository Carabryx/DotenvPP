<p align="center">
  <h1 align="center">🔐 DotenvPP</h1>
  <p align="center"><strong>Dotenv, but evolved. Environment configuration for the modern era.</strong></p>
  <p align="center">
    <em>Written in Rust. Compiled to WASM. Zero compromises.</em>
  </p>
</p>

<p align="center">
  <a href="https://crates.io/crates/dotenvpp"><img src="https://img.shields.io/crates/v/dotenvpp?color=171717" alt="Crates.io Version" /></a>
  <a href="https://crates.io/crates/dotenvpp"><img src="https://img.shields.io/crates/d/dotenvpp?color=171717" alt="Crates.io Downloads" /></a>
  <a href="https://docs.rs/dotenvpp"><img src="https://img.shields.io/docsrs/dotenvpp?color=171717" alt="docs.rs" /></a>
  <a href="https://github.com/Carabryx/DotenvPP/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Carabryx/DotenvPP/ci.yml?label=CI&color=171717" alt="CI" /></a>
  <a href="https://github.com/Carabryx/DotenvPP/releases"><img src="https://img.shields.io/badge/release-v0.0.2-171717" alt="Release 0.0.2" /></a>
  <a href="https://coderabbit.ai"><img src="https://img.shields.io/coderabbit/prs/github/Carabryx/DotenvPP?utm_source=oss&utm_medium=github&utm_campaign=Carabryx%2FDotenvPP&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews" alt="CodeRabbit Pull Request Reviews" /></a>
</p>

<p align="center">
  <a href="#why">Why?</a> •
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#what-makes-it-different">What's Different</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#roadmap">Roadmap</a> •
  <a href="#contributing">Contributing</a>
</p>

---

## Why?

The `.env` file format was created in 2012. Since then:
- Cloud-native computing was born
- Supply chain attacks became the #1 threat vector
- Microservices replaced monoliths
- Edge computing and WASM emerged
- AI-assisted development changed how we write code

**Yet `.env` files haven't changed at all.** They're still plaintext, untyped, unvalidated, and insecure.

DotenvPP reimagines environment configuration from first principles — taking everything we've learned in 14 years and building something that **actually helps** instead of being a silent source of bugs and security vulnerabilities.

> 💡 **A million secrets** have been leaked from exposed `.env` files ([Trend Micro, 2022](https://www.trendmicro.com/)). It's time for something better.

---

## Features

### 🎯 What dotenv should have been

| Feature | dotenv | dotenvx | DotenvPP |
|---|:---:|:---:|:---:|
| Basic `KEY=VALUE` parsing | ✅ | ✅ | ✅ |
| Variable interpolation (`${VAR}`) | ⚠️ | ✅ | ✅ |
| Multi-environment layering | ❌ | ✅ | ✅ |
| Encryption at rest | ❌ | ✅ | ✅ |
| **Type system & validation** | ❌ | ❌ | ✅ |
| **Schema definitions** | ❌ | ❌ | ✅ |
| **Expression language** | ❌ | ❌ | ✅ |
| **Policy-as-code rules** | ❌ | ❌ | ✅ |
| **Memory zeroization** | ❌ | ❌ | ✅ |
| **WASM support** | ❌ | ❌ | ✅ |
| Written in Rust | ❌ | ❌ | ✅ |

### 🔒 Security-First

- **Encryption at rest** — AES-256-GCM with X25519 key exchange. Encrypted files are safe to commit to git.
- **Memory zeroization** — Secrets are wiped from RAM after use via Rust's `zeroize` crate.
- **Leak prevention** — Built-in git hooks, CI scanners, and audit commands to catch exposed secrets.
- **Per-value encryption** — Each value encrypted with a unique ephemeral key.

### 📐 Typed Configuration

```toml
# .env.schema
[vars.PORT]
type = "u16"
default = 8080
range = [1024, 65535]

[vars.DATABASE_URL]
type = "url"
required = true
protocols = ["postgres", "postgresql"]

[vars.LOG_LEVEL]
type = "enum"
values = ["trace", "debug", "info", "warn", "error"]
default = "info"
```

Your app crashes **at startup** with a clear error — not at 3 AM in production when it tries to parse `PORT=banana` as a number.

### 🧮 Computed Configuration

```env
MAX_WORKERS = ${CPU_COUNT} * 2
API_URL = "${PROTOCOL}://${HOST}:${PORT}/api/v${API_VERSION}"
LOG_LEVEL = if $ENV == "production" then "warn" else "debug"
```

A safe, sandboxed expression language. No I/O, no loops, no side effects.

### 📋 Policy Engine

```toml
# .env.policy
[[rules]]
name = "no-debug-in-prod"
condition = "ENV == 'production' && LOG_LEVEL == 'debug'"
severity = "error"
```

Like OPA, but for your `.env` files. Enforce security rules across all environments.

---

## Quick Start

> ⚠️ **DotenvPP is in active development.** The API shown here represents the design target.

### CLI

```bash
# Install
cargo install dotenvpp

# Parse and validate
dotenvpp check

# Encrypt your .env file
dotenvpp encrypt

# Run a command with decrypted env vars
dotenvpp run -- node server.js

# Generate .env.example from schema
dotenvpp schema example
```

### Rust Crate

```rust
use dotenvpp::Config;

#[derive(dotenvpp::Schema)]
struct AppConfig {
    #[env(required)]
    database_url: url::Url,

    #[env(default = 8080, range(1024..=65535))]
    port: u16,

    #[env(secret, min_length = 32)]
    api_key: dotenvpp::Secret<String>,
}

fn main() -> Result<(), dotenvpp::Error> {
    let config: AppConfig = Config::from_env()?;
    println!("Listening on port {}", config.port);
    Ok(())
}
```

### WASM (Browser/Edge)

```javascript
import { DotenvPP } from '@dotenvpp/wasm';

const result = DotenvPP.validate(envFile, schema);
if (!result.valid) {
  console.error(result.errors);
}
```

---

## What Makes It Different

### vs. dotenv / dotenvy
DotenvPP is a **superset**. Every existing `.env` file works unchanged. But DotenvPP adds types, schemas, encryption, expressions, and policies.

### vs. dotenvx
dotenvx adds encryption. DotenvPP adds encryption **and** types, schemas, expressions, policies, WASM support, and memory safety. Built in Rust, not JavaScript.

### vs. HashiCorp Vault / AWS Secrets Manager
Those are infrastructure. DotenvPP is a **tool**. No servers, no SaaS, no ops overhead. Use DotenvPP for local dev and CI. Use Vault for production secrets if you need to — DotenvPP can bridge both.

### vs. SOPS
SOPS is encryption-only. DotenvPP is encryption + types + schemas + expressions + policies + WASM.

---

## Architecture

DotenvPP is built as a modular Rust workspace:

```
dotenvpp/
├── crates/
│   ├── dotenvpp-parser/    # Zero-copy .env parser
│   ├── dotenvpp-schema/    # Schema validation engine
│   ├── dotenvpp-expr/      # Expression language evaluator
│   ├── dotenvpp-policy/    # Policy-as-code engine
│   ├── dotenvpp-crypto/    # Encryption (AES-256-GCM + X25519)
│   ├── dotenvpp-layers/    # Environment layering
│   └── dotenvpp-wasm/      # WASM bindings
├── dotenvpp/               # Facade crate (re-exports)
└── dotenvpp-cli/           # CLI tool
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full technical vision.

---

## Roadmap

| Phase | Description | Status |
|---|---|---|
| 0 | Foundation — Standard `.env` parsing | ✅ Complete |
| 1 | Interpolation & environment layering | 🔜 Next |
| 2 | Schema & type system | 📋 Planned |
| 3 | Encryption | 📋 Planned |
| 4 | Expression language | 📋 Planned |
| 5 | Policy engine | 📋 Planned |
| 6 | WASM target | 📋 Planned |
| 7 | DX & ecosystem (VS Code, bindings) | 📋 Planned |
| 8 | Advanced (remote config, rotation, audit) | 📋 Planned |

See [docs/TODO.md](docs/TODO.md) for the detailed roadmap.

---

## Research

This project is informed by extensive research into:

- **Academic papers**: Trend Micro (2022), Basak et al. (2022), OWASP guidelines
- **Competitor analysis**: dotenvx, SOPS, Infisical, Doppler, Configu, HashiCorp Vault
- **Industry standards**: 12-Factor App, Policy-as-Code (OPA), Zero Trust Architecture

See [docs/RESEARCH.md](docs/RESEARCH.md) for the full research synthesis.

---

## Tech Stack

- **Language**: Rust (2021 edition)
- **Crypto**: `crabgraph` (default) — ergonomic wrapper over audited RustCrypto primitives, with WASM support and auto-zeroization. Raw `aes-gcm` + `x25519-dalek` available as opt-in alternative via feature flags.
- **CLI**: `clap` v4 with `miette` for beautiful errors
- **WASM**: `wasm-bindgen`, `wasm-pack`
- **Serialization**: `serde`, `toml`
- **Testing**: `proptest` (property-based), `insta` (snapshot)

---

## Contributing

DotenvPP is in the design phase. Contributions welcome!

1. Read [docs/RESEARCH.md](docs/RESEARCH.md) for context
2. Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the technical vision
3. Check [docs/TODO.md](docs/TODO.md) for what needs doing
4. Open an issue or PR

---

<p align="center">
  <strong>The `.env` file hasn't evolved since 2012. It's time.</strong>
</p>
