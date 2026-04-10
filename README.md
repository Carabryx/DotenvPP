<p align="center">
  <h1 align="center">🔐 DotenvPP</h1>
  <p align="center"><strong>Dotenv, but evolved. Environment configuration for the modern era.</strong></p>
  <p align="center">
    <em>Phase 1 ships interpolation and layered loading in Rust.</em>
  </p>
</p>

<p align="center">
  <a href="https://crates.io/crates/dotenvpp"><img src="https://img.shields.io/crates/v/dotenvpp?color=171717" alt="Crates.io Version" /></a>
  <a href="https://crates.io/crates/dotenvpp"><img src="https://img.shields.io/crates/d/dotenvpp?color=171717" alt="Crates.io Downloads" /></a>
  <a href="https://docs.rs/dotenvpp"><img src="https://img.shields.io/docsrs/dotenvpp?color=171717" alt="docs.rs" /></a>
  <a href="https://github.com/Carabryx/DotenvPP/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/Carabryx/DotenvPP/ci.yml?label=CI&color=171717" alt="CI" /></a>
  <a href="https://github.com/Carabryx/DotenvPP/releases/latest"><img src="https://img.shields.io/github/v/release/Carabryx/DotenvPP?label=release&color=171717" alt="Latest release" /></a>
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

DotenvPP `0.0.3` ships the parser foundation plus Phase 1 interpolation and layered loading.

| Capability | Status | Notes |
|---|---|---|
| Basic `KEY=VALUE` parsing | ✅ Shipped | Core parser behavior |
| Comments, blank lines, `export` | ✅ Shipped | Common dotenv syntax |
| Single-quoted, double-quoted, and unquoted values | ✅ Shipped | Includes multiline quoted values |
| BOM handling and common escape decoding | ✅ Shipped | Phase 0 parser behavior |
| Load parsed values into `std::env` | ✅ Shipped | Includes layered loading and override variants |
| CLI `check` and `run` commands | ✅ Shipped | Supports `--file` and `--env` |
| Variable interpolation (`${VAR}`) | ✅ Shipped | Includes default, required, alternative, and `$$` escaping |
| Environment layering | ✅ Shipped | `.env` < `.env.{ENV}` < `.env.local` < `.env.{ENV}.local` |
| Schema and type system | ⏳ Phase 2 | Roadmap |
| Encryption | ⏳ Phase 3 | Roadmap |
| Expression language | ⏳ Phase 4 | Roadmap |
| Policy engine | ⏳ Phase 5 | Roadmap |
| WASM target | ⏳ Phase 6 | Roadmap |

---

## Quick Start

The commands and APIs below reflect the current shipped surface. Higher-level APIs for schemas, encryption, expressions, policies, and WASM remain roadmap items in [docs/TODO.md](docs/TODO.md) and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

### CLI

```bash
# Install
cargo install dotenvpp-cli

# Check the layered config for a selected environment
dotenvpp check --env production

# Load the layered production stack and run a command with those variables
dotenvpp run --env production -- cargo test

# Or target one explicit file
dotenvpp check --file .env
```

### Rust Crate

```rust
fn main() -> Result<(), dotenvpp::Error> {
    dotenvpp::load_with_env("production")?;

    let app_name = dotenvpp::var("APP_NAME")?;
    println!("APP_NAME={app_name}");

    let preview = dotenvpp::from_read(&b"HOST=localhost\nURL=http://${HOST}"[..])?;
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[1].value, "http://localhost");

    Ok(())
}
```

---

## What Makes It Different

### vs. dotenv / dotenvy
DotenvPP starts with a from-scratch parser instead of wrapping an existing dotenv crate. That leaves interpolation, layering, schemas, and later roadmap features on top of parser behavior the project owns.

### vs. dotenvx
dotenvx is already further ahead on encrypted workflows. DotenvPP is taking a different path: ship a solid Rust parser, interpolation, and layering surface first, then build later phases on that foundation.

### vs. HashiCorp Vault / AWS Secrets Manager
Those are infrastructure products. DotenvPP is a developer-facing library and CLI. Even in Phase 0, the goal is local parsing/loading ergonomics rather than replacing secret-management platforms.

### vs. SOPS
SOPS is focused on encryption. DotenvPP is broader in roadmap scope, but those later capabilities are still planned work rather than current release features.

---

## Architecture

Current workspace layout:

```
dotenvpp/
├── crates/
│   ├── dotenvpp-parser/    # Phase 0 parser engine
│   └── dotenvpp-cli/       # CLI binary with layered loading support
├── src/lib.rs              # Facade crate API
├── tests/                  # Facade integration tests
├── examples/               # In-crate examples
└── usage-examples/         # Separate demo crate (`publish = false`)
```

Planned crates such as `dotenvpp-schema`, `dotenvpp-expr`, `dotenvpp-policy`, `dotenvpp-crypto`, `dotenvpp-layers`, and `dotenvpp-wasm` are part of the design vision, not current workspace members. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for that longer-term target.

---

## Roadmap

| Phase | Description | Status |
|---|---|---|
| 0 | Foundation — Standard `.env` parsing | ✅ Complete |
| 1 | Interpolation & environment layering | ✅ Implemented |
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
- **CLI**: `clap` v4
- **Parser**: custom parser in `dotenvpp-parser`
- **Benchmarking**: `criterion`
- **Quality**: `cargo fmt`, `clippy`, tests, GitHub Actions

Planned later phases introduce additional dependencies such as `miette`, `serde`, `toml`, `crabgraph`, and `wasm-bindgen` as those capabilities land.

---

## Contributing

DotenvPP has shipped Phase 1 and is moving toward Phase 2. Contributions welcome.

1. Read [docs/RESEARCH.md](docs/RESEARCH.md) for context
2. Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the technical vision
3. Check [docs/TODO.md](docs/TODO.md) for the active roadmap, especially interpolation and layering
4. Open an issue or PR

---

<p align="center">
  <strong>The `.env` file hasn't evolved since 2012. It's time.</strong>
</p>
