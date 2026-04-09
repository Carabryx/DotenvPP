//! # DotenvPP
//!
//! From-scratch `.env` parsing and loading for Rust.
//! This crate currently ships the Phase 0 foundation: parse common
//! `.env` syntax, load values into the process environment, and
//! inspect parsed key-value pairs.
//!
//! The broader project roadmap lives in the repository docs.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! // Just load .env - that's it!
//! dotenvpp::load().ok();
//!
//! // Access variables (no `use std::env` needed)
//! let db_url = dotenvpp::var("DATABASE_URL").unwrap();
//! ```
//!
//! ## API Overview
//!
//! | Function | Description |
//! |---|---|
//! | [`load()`] | Load `.env` from cwd (won't override existing) |
//! | [`load_override()`] | Load `.env` overriding existing vars |
//! | [`from_path()`] | Load from a custom path |
//! | [`from_path_override()`] | Load from custom path, overriding |
//! | [`from_read()`] | Parse from any `impl Read` |
//! | [`var()`] | Get a single env var |
//! | [`vars()`] | Iterate all env vars |

use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
use std::path::Path;

// Re-export parser types so users don't need a separate dependency.
pub use dotenvpp_parser::{EnvPair, ParseError};

// ── Error ───────────────────────────────────────────────────

/// Errors that can occur when loading `.env` files.
#[derive(Debug)]
pub enum Error {
    /// An I/O error (file not found, permission denied, etc.).
    Io(std::io::Error),
    /// A parse error in the `.env` content.
    Parse(ParseError),
    /// An environment variable was not found or not valid unicode.
    NotPresent(String),
    /// An environment variable was found but contained invalid unicode.
    NotUnicode(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Parse(err) => write!(f, "parse error: {err}"),
            Self::NotPresent(key) => {
                write!(f, "environment variable `{key}` not found")
            }
            Self::NotUnicode(key) => {
                write!(f, "environment variable `{key}` contains invalid unicode")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Parse(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Self::Parse(err)
    }
}

/// Convenience type alias.
pub type Result<T> = std::result::Result<T, Error>;

// ── Core: load ──────────────────────────────────────────────

/// Load environment variables from `.env` in the current directory.
///
/// Variables are added to the process environment.
/// **Existing** environment variables are NOT overridden.
///
/// # Examples
///
/// ```rust,no_run
/// dotenvpp::load().ok(); // ignore if .env doesn't exist
/// ```
pub fn load() -> Result<Vec<EnvPair>> {
    from_path(".env")
}

/// Load environment variables from `.env`, **overriding** existing ones.
///
/// # Examples
///
/// ```rust,no_run
/// dotenvpp::load_override().ok();
/// ```
pub fn load_override() -> Result<Vec<EnvPair>> {
    from_path_override(".env")
}

// ── Core: from_path ─────────────────────────────────────────

/// Load environment variables from a specific file path.
///
/// **Existing** environment variables are NOT overridden.
///
/// # Safety
///
/// This function mutates the process environment via
/// [`std::env::set_var`]. Call it early in program startup, before
/// spawning threads, to avoid races with concurrent environment access.
///
/// # Examples
///
/// ```rust,no_run
/// dotenvpp::from_path(".env.production")?;
/// # Ok::<(), dotenvpp::Error>(())
/// ```
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Vec<EnvPair>> {
    let content = fs::read_to_string(path)?;
    let pairs = dotenvpp_parser::parse(&content)?;
    let existing_keys: HashSet<OsString> = env::vars_os().map(|(key, _)| key).collect();

    for pair in &pairs {
        if !existing_keys.contains(OsStr::new(&pair.key)) {
            // SAFETY: intended use - loading env config at startup.
            unsafe { env::set_var(&pair.key, &pair.value) };
        }
    }

    Ok(pairs)
}

/// Load from a specific path, **overriding** existing env vars.
///
/// # Safety
///
/// This function mutates the process environment via
/// [`std::env::set_var`]. Call it early in program startup, before
/// spawning threads, to avoid races with concurrent environment access.
pub fn from_path_override<P: AsRef<Path>>(path: P) -> Result<Vec<EnvPair>> {
    let content = fs::read_to_string(path)?;
    let pairs = dotenvpp_parser::parse(&content)?;

    for pair in &pairs {
        // SAFETY: intended use - loading env config at startup.
        unsafe { env::set_var(&pair.key, &pair.value) };
    }

    Ok(pairs)
}

/// Parse a `.env` file and return an iterator over the pairs
/// **without** setting them in the environment.
///
/// # Examples
///
/// ```rust,no_run
/// for pair in dotenvpp::from_path_iter(".env")? {
///     println!("{} = {}", pair.key, pair.value);
/// }
/// # Ok::<(), dotenvpp::Error>(())
/// ```
pub fn from_path_iter<P: AsRef<Path>>(path: P) -> Result<impl Iterator<Item = EnvPair>> {
    let content = fs::read_to_string(path)?;
    let pairs = dotenvpp_parser::parse(&content)?;
    Ok(pairs.into_iter())
}

// ── Core: from_read ─────────────────────────────────────────

/// Parse `.env` content from any reader **without** setting env vars.
///
/// # Examples
///
/// ```
/// let input = b"KEY=value\nNAME=\"hello\"";
/// let pairs = dotenvpp::from_read(&input[..]).unwrap();
/// assert_eq!(pairs[0].key, "KEY");
/// ```
pub fn from_read<R: Read>(mut reader: R) -> Result<Vec<EnvPair>> {
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    let pairs = dotenvpp_parser::parse(&content)?;
    Ok(pairs)
}

// ── Convenience: var / vars ─────────────────────────────────

/// Get a single environment variable's value.
///
/// This is a convenience wrapper around [`std::env::var`] that uses
/// DotenvPP's error type, so you don't need `use std::env`.
///
/// # Examples
///
/// ```rust,no_run
/// dotenvpp::load().ok();
/// let db_url = dotenvpp::var("DATABASE_URL")?;
/// # Ok::<(), dotenvpp::Error>(())
/// ```
pub fn var<K: AsRef<OsStr>>(key: K) -> Result<String> {
    let key_ref = key.as_ref();
    match env::var(key_ref) {
        Ok(val) => Ok(val),
        Err(env::VarError::NotPresent) => {
            Err(Error::NotPresent(key_ref.to_string_lossy().into_owned()))
        }
        Err(env::VarError::NotUnicode(_)) => {
            Err(Error::NotUnicode(key_ref.to_string_lossy().into_owned()))
        }
    }
}

/// Returns an iterator over all environment variables as
/// `(String, String)` pairs.
///
/// Snapshot of the process environment at the time of invocation.
///
/// # Examples
///
/// ```rust,no_run
/// dotenvpp::load().ok();
/// for (key, value) in dotenvpp::vars() {
///     println!("{key} = {value}");
/// }
/// ```
pub fn vars() -> env::Vars {
    env::vars()
}

// ── Meta ────────────────────────────────────────────────────

/// Returns the crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

// ── Tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_from_read() {
        let input = b"KEY=value\nNAME=\"hello\"";
        let pairs = from_read(&input[..]).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].key, "KEY");
        assert_eq!(pairs[0].value, "value");
        assert_eq!(pairs[1].key, "NAME");
        assert_eq!(pairs[1].value, "hello");
    }

    #[test]
    fn test_error_display() {
        let err = Error::Parse(ParseError::EmptyKey {
            line: 1,
        });
        let msg = format!("{err}");
        assert!(msg.contains("parse error"));
    }

    #[test]
    fn test_not_present_error() {
        let err = Error::NotPresent("MISSING_KEY".into());
        let msg = format!("{err}");
        assert!(msg.contains("MISSING_KEY"));
        assert!(msg.contains("not found"));
    }

    #[cfg(not(windows))]
    #[test]
    fn test_from_path_preserves_existing_non_unicode_vars() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use std::time::{SystemTime, UNIX_EPOCH};

        let path = std::env::temp_dir().join(format!(
            "dotenvpp-preserve-{}.env",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::write(&path, "KEY=overridden\n").unwrap();

        let value = OsString::from_vec(vec![0x66, 0x80]);

        // SAFETY: test setup for a temporary env var.
        unsafe { std::env::set_var("KEY", &value) };

        let pairs = from_path(&path).unwrap();
        assert_eq!(pairs[0].key, "KEY");
        assert_eq!(std::env::var_os("KEY").unwrap(), value);

        // SAFETY: test cleanup for a temporary env var.
        unsafe { std::env::remove_var("KEY") };
        std::fs::remove_file(path).unwrap();
    }
}
