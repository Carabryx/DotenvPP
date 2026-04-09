//! # dotenvpp-parser
//!
//! Core `.env` file parser for DotenvPP.
//!
//! This crate is `no_std`-compatible (with `alloc`) so it can be used
//! in WASM and embedded environments. Enable the `std` feature (on by
//! default) for `std::error::Error` implementations.
//!
//! ## Features
//!
//! - `KEY=VALUE` basic parsing with comment and blank-line handling
//! - Single-quoted values (literal, multiline supported), double-quoted values
//!   (with escapes), and unquoted values
//! - Multiline values in single-quoted and double-quoted strings
//! - `export KEY=VALUE` prefix support
//! - Escape sequences: `\\`, `\"`, `\n`, `\t`, `\r`, `\$`
//! - Common unquoted escapes for spaces, quotes, dollar signs, and newlines
//!
//! ## Example
//!
//! ```
//! use dotenvpp_parser::parse;
//!
//! let input = "# Database config\nDB_HOST=localhost\nDB_PORT=5432\nSECRET='keep-it-safe'\n";
//!
//! let pairs = parse(input).unwrap();
//! assert_eq!(pairs.len(), 3);
//! assert_eq!(pairs[0].key, "DB_HOST");
//! assert_eq!(pairs[0].value, "localhost");
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod error;
mod parser;

pub use error::ParseError;
pub use parser::{parse, EnvPair};
