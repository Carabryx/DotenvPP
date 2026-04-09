# DotenvPP — Technical Architecture Vision

> **Status**: Pre-implementation design document
> **Language**: Rust | **Target**: Native + WASM
> **License**: MIT OR Apache-2.0

---

## 1. What is DotenvPP?

**DotenvPP** (Dotenv++) is a next-generation environment configuration tool that replaces the stagnant `.env` ecosystem with a modern, secure, and typed alternative.

It is:
- A **CLI tool** for managing, encrypting, validating, and linting `.env` files
- A **Rust library** (crate) embeddable in any Rust project
- A **WASM module** that runs in browsers, edge runtimes, and WASI environments
- **Backwards compatible** with existing `.env` files (zero migration cost)

---

## 2. Core Architecture

```
                    ┌──────────────────────────────────┐
                    │            DotenvPP              │
                    │                                  │
  .env files ──────►│  ┌──────────┐  ┌──────────────┐ │──────► Typed Config
  .env.schema ─────►│  │  Parser  │──│  Expression  │ │        Object
  .env.policy ─────►│  │  Engine  │  │  Evaluator   │ │
  .env.*.enc ──────►│  └──────────┘  └──────────────┘ │
                    │        │               │         │
                    │  ┌─────▼───┐   ┌───────▼──────┐ │
                    │  │ Schema  │   │   Policy     │ │
                    │  │Validator│   │   Engine     │ │
                    │  └─────────┘   └──────────────┘ │
                    │        │               │         │
                    │  ┌─────▼───────────────▼──────┐ │
                    │  │     Crypto Engine          │ │
                    │  │  (encrypt/decrypt/zeroize) │ │
                    │  └────────────────────────────┘ │
                    │                                  │
                    │  Targets: Native │ WASM │ WASI   │
                    └──────────────────────────────────┘
```

---

## 3. Module Breakdown

### 3.1 Parser Engine (`dotenvpp-parser`)

The foundation. Parses `.env` files into a structured AST.

**Backwards compatible** with standard `.env` syntax:
```env
# Standard (all existing .env files work)
DATABASE_URL=postgres://localhost:5432/mydb
SECRET_KEY=supersecret123
```

**Extended syntax (DotenvPP++)**:
```env
# Typed annotations
PORT: u16 = 8080
DEBUG: bool = false
TIMEOUT: duration = 30s
ALLOWED_ORIGINS: string[] = http://localhost,https://app.com

# Expressions
MAX_WORKERS = ${CPU_COUNT} * 2
API_URL = "${PROTOCOL}://${HOST}:${PORT}/api/v${API_VERSION}"

# Conditional
LOG_LEVEL = if $ENV == "production" then "warn" else "debug"

# File references
TLS_CERT = file("./certs/server.pem")
CONFIG_JSON = file("./config.json", base64)

# Secure markers (for documentation / leak detection)
API_KEY: secret = sk-xxxxxxxxxxxx
```

**Design Decisions**:
- Parser is a **zero-copy, streaming parser** for maximum performance
- Uses a PEG or custom recursive descent parser (not regex)
- Produces a typed AST, not a flat `HashMap<String, String>`
- Parser module compiles to WASM independently

### 3.2 Schema Validator (`dotenvpp-schema`)

Separate schema definition files that describe expected configuration:

```toml
# .env.schema

[meta]
name = "my-app"
version = "1.0"
description = "Configuration schema for My App"

[vars.DATABASE_URL]
type = "url"
required = true
protocols = ["postgres", "postgresql"]
description = "PostgreSQL connection string"

[vars.PORT]
type = "u16"
default = 8080
range = [1024, 65535]
description = "HTTP server port"

[vars.LOG_LEVEL]
type = "enum"
values = ["trace", "debug", "info", "warn", "error"]
default = "info"

[vars.FEATURE_FLAGS]
type = "string[]"
separator = ","
default = []
description = "Comma-separated list of enabled features"

[vars.API_TIMEOUT]
type = "duration"
default = "30s"
min = "1s"
max = "5m"

[vars.SMTP_PASSWORD]
type = "string"
required = true
secret = true  # Marks as sensitive — affects logging, .env.example generation
min_length = 8
```

**Supported Types**: `string`, `bool`, `i32`, `i64`, `u16`, `u32`, `u64`, `f64`, `url`, `email`, `ip`, `port`, `duration`, `datetime`, `regex`, `path`, `enum`, `string[]`, `i32[]`

**What the schema enables**:
- **Startup fail-fast**: App crashes immediately with clear error if config is invalid
- **Auto-generated `.env.example`**: Schema → example file with descriptions, no secret values
- **IDE autocompletion**: Schema → JSON Schema → VS Code / JetBrains integration
- **Documentation**: Schema is self-documenting configuration reference
- **CI validation**: Validate `.env` files in CI without running the app

### 3.3 Expression Evaluator (`dotenvpp-expr`)

A safe, sandboxed expression language for computed configuration values.

**Core Expressions**:
```
# Arithmetic
MAX_POOL = ${CPU_COUNT} * 2 + 1

# String operations
FULL_URL = concat(${PROTOCOL}, "://", ${HOST}, ":", ${PORT})

# Conditionals
LOG_LEVEL = if ${ENV} == "production" then "warn" else "debug"

# Built-in functions
SECRET_HASH = sha256(${RAW_SECRET})
CACHE_TTL = duration("30m").as_secs()
RANDOM_ID = uuid()
TIMESTAMP = now().iso8601()
```

**Safety Guarantees**:
- No I/O operations (no file reads, no network, no shell exec)
- No infinite loops (no loop constructs)
- Bounded recursion depth
- Deterministic evaluation (except for `uuid()`, `now()`)
- Pure functions only (except explicitly marked ones)

### 3.4 Policy Engine (`dotenvpp-policy`)

Declarative rules that constrain valid configurations:

```toml
# .env.policy

[meta]
name = "production-security"
description = "Security policies for production deployments"

[[rules]]
name = "no-debug-in-prod"
description = "Debug logging is forbidden in production"
condition = "ENV == 'production' && LOG_LEVEL == 'debug'"
severity = "error"

[[rules]]
name = "ssl-required"
description = "Database must use SSL in production"
condition = "ENV == 'production' && !contains(DATABASE_URL, 'sslmode=require')"
severity = "error"

[[rules]]
name = "strong-secrets"
description = "All secrets must be at least 32 characters"
condition = "any(secrets, |s| len(s) < 32)"
severity = "warning"

[[rules]]
name = "no-localhost"
description = "No localhost URLs in staging/production"
condition = "ENV != 'development' && any_contains(urls, 'localhost')"
severity = "error"
```

### 3.5 Crypto Engine (`dotenvpp-crypto`)

**Encryption**: AES-256-GCM with X25519 key exchange.

**Swappable Backend via Feature Flags**:

DotenvPP uses [CrabGraph](https://crates.io/crates/crabgraph) as its **default** crypto backend — an ergonomic Rust crypto toolkit built on audited RustCrypto primitives. For users who prefer using the raw audited crates directly, DotenvPP offers a compile-time alternative via feature flags. **No runtime overhead, no trait generics, no bloat.**

```toml
# Cargo.toml (dotenvpp-crypto)
[features]
default = ["crypto-crabgraph"]
crypto-crabgraph  = ["crabgraph"]           # Default: ergonomic, batteries-included
crypto-rustcrypto = ["aes-gcm", "x25519-dalek", "argon2", "hkdf", "zeroize"]
```

```rust
// Default path — clean, zero bloat
#[cfg(feature = "crypto-crabgraph")]
mod backend {
    use crabgraph::aead::{AesGcm256, CrabAead};
    use crabgraph::asym::X25519KeyPair;

    pub fn encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        let cipher = AesGcm256::new(key)?;
        cipher.encrypt(data, None)
    }
}

// Opt-in alternative — only compiles if explicitly chosen
#[cfg(feature = "crypto-rustcrypto")]
mod backend {
    use aes_gcm::{Aes256Gcm, KeyInit, /* ... */};

    pub fn encrypt(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
        // Raw RustCrypto implementation
    }
}
```

**Why CrabGraph as default?**

| CrabGraph Feature | DotenvPP Usage |
|---|---|
| `aead::AesGcm256` | Per-value encryption |
| `asym::X25519KeyPair` | Key exchange for encryption |
| `kdf::hkdf_extract_expand` | Key derivation from shared secrets |
| `kdf::argon2_derive` | Passphrase-based encryption |
| `hash::sha256` | Expression language `sha256()` function |
| `rand::secure_bytes` | Nonce and key generation |
| `secrets` module | Automatic memory zeroization |
| `key_rotation` module | Built-in key rotation support |
| `wasm` feature flag | Shared WASM compilation target |

For **99% of users**: `cargo add dotenvpp` → uses CrabGraph → done.
For the **paranoid 1%**: `cargo add dotenvpp --no-default-features --features crypto-rustcrypto`

```
Encryption Flow:
                                           
  Plaintext .env ──► Per-value encryption ──► .env.enc
                      │                           │
                      ▼                           ▼
                 Ephemeral key              DOTENV_PUBLIC_KEY
                 (per value)                embedded in file
                      │
                      ▼
                 X25519 key exchange
                      │
                      ▼
                 AES-256-GCM seal

Decryption Flow:

  .env.enc + DOTENV_PRIVATE_KEY ──► In-memory plaintext ──► Zeroized after use
```

**Key Features**:
- **Per-value encryption**: Each value gets its own ephemeral key
- **Memory zeroization**: Secrets wiped from RAM after use (via CrabGraph's `secrets` module)
- **Key rotation**: Built-in `dotenvpp rotate` command (via CrabGraph's `key_rotation`)
- **Multiple recipients**: Encrypt for multiple team members
- **Optional KMS integration**: AWS KMS, GCP KMS, Vault (future feature flags)

### 3.6 Environment Layering (`dotenvpp-layers`)

```
Priority (highest wins):
  1. Process environment variables (OS-level)
  2. .env.{environment}.local    (gitignored, env-specific)
  3. .env.local                  (gitignored, all environments)
  4. .env.{environment}          (committed, env-specific)
  5. .env                        (committed, defaults)
  6. .env.schema defaults        (schema-defined defaults)
```

Each layer **merges** into the previous, with clear override semantics.

---

## 4. Target Outputs

### 4.1 CLI Tool (`dotenvpp`)

```bash
# Parse and validate
dotenvpp check                    # Validate .env against schema
dotenvpp check --strict           # Also enforce policies
dotenvpp lint                     # Lint for best practices

# Encryption
dotenvpp encrypt                  # Encrypt .env → .env.enc
dotenvpp decrypt                  # Decrypt .env.enc → .env (in-memory)
dotenvpp encrypt --rotate         # Re-encrypt with new keys

# Run commands with injected env
dotenvpp run -- node server.js    # Like dotenvx run
dotenvpp run -e production -- ./app

# Schema tools
dotenvpp schema init              # Generate .env.schema from existing .env
dotenvpp schema example           # Generate .env.example from schema
dotenvpp schema docs              # Generate markdown docs from schema
dotenvpp schema json-schema       # Export as JSON Schema (for IDE integration)

# Diff and audit
dotenvpp diff .env .env.production  # Show differences between env files
dotenvpp audit                      # Scan for leaked secrets, weak values
```

### 4.2 Rust Crate (`dotenvpp`)

```rust
use dotenvpp::Config;

#[derive(dotenvpp::Schema)]
struct AppConfig {
    #[env(required, description = "Database connection URL")]
    database_url: url::Url,

    #[env(default = 8080, range(1024..=65535))]
    port: u16,

    #[env(default = "info", values = ["trace", "debug", "info", "warn", "error"])]
    log_level: String,

    #[env(secret, min_length = 32)]
    api_key: dotenvpp::Secret<String>,  // Auto-zeroized on drop
}

fn main() -> Result<(), dotenvpp::Error> {
    let config = Config::from_env()?;  // Validates, types, and loads
    println!("Server starting on port {}", config.port);
    // config.api_key is zeroized when dropped
    Ok(())
}
```

### 4.3 WASM Module (`dotenvpp-wasm`)

```javascript
import { DotenvPP } from '@dotenvpp/wasm';

// Validate in browser (e.g., CI dashboard, web IDE)
const result = DotenvPP.validate(envFileContent, schemaContent);
if (result.errors.length > 0) {
  console.error('Config errors:', result.errors);
}

// Parse with type coercion
const config = DotenvPP.parse(envFileContent, { typed: true });
console.log(config.PORT);  // number, not string
```

---

## 5. File Format Summary

| File | Purpose | Git? |
|---|---|---|
| `.env` | Default configuration values | ✅ Committed |
| `.env.local` | Local overrides | ❌ Gitignored |
| `.env.production` | Production-specific values | ✅ Committed (if encrypted) |
| `.env.production.local` | Local production overrides | ❌ Gitignored |
| `.env.enc` | Encrypted `.env` | ✅ Committed |
| `.env.schema` | Type definitions and validation rules | ✅ Committed |
| `.env.policy` | Policy constraints | ✅ Committed |
| `.env.example` | Auto-generated example (from schema) | ✅ Committed |

---

## 6. Technology Stack

| Component | Technology |
|---|---|
| Core Language | Rust (2021 edition) |
| Parser | Custom PEG / recursive descent |
| Crypto (default) | `crabgraph` — ergonomic wrapper over RustCrypto primitives |
| Crypto (alt) | `aes-gcm`, `x25519-dalek`, `argon2`, `hkdf`, `zeroize` (opt-in) |
| Schema Format | TOML |
| Policy Format | TOML with embedded expressions |
| Serialization | `serde`, `serde_json`, `toml` |
| WASM Target | `wasm-bindgen`, `wasm-pack` |
| CLI Framework | `clap` v4 |
| Error Handling | `thiserror`, `miette` (pretty errors) |
| Testing | `proptest` (property-based), `insta` (snapshot) |

---

*This is a living architecture document. It will evolve as implementation progresses.*
