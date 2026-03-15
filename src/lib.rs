//! # DotenvPP — Dotenv++
//!
//! Next-generation environment configuration with typed schemas,
//! encryption, expressions, policies, and WASM support.
//!
//! **⚠️ This crate is in early development. The API is not yet stable.**
//!
//! ## What is DotenvPP?
//!
//! DotenvPP is a modern replacement for `.env` file management that adds:
//!
//! - **Typed schemas** — Define types, ranges, and validation rules
//! - **Encryption** — AES-256-GCM with X25519 key exchange
//! - **Expressions** — Computed configuration values
//! - **Policies** — Declarative rules for configuration governance
//! - **WASM support** — Runs in browsers and edge runtimes
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! // Coming soon — DotenvPP is in active development
//! ```

/// DotenvPP is in early development. This is a name reservation.
/// Full implementation coming soon.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_returns_version() {
        assert_eq!(version(), "0.0.1");
    }
}
