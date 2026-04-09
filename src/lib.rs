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
//! | [`vars_os()`] | Iterate env vars without Unicode conversion |

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
/// # Panics
///
/// Panics if any environment variable key or value is not valid
/// Unicode. Use [`vars_os()`] if you need to preserve non-Unicode
/// entries.
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

/// Returns an iterator over all environment variables as
/// `(OsString, OsString)` pairs.
///
/// Snapshot of the process environment at the time of invocation.
pub fn vars_os() -> env::VarsOs {
    env::vars_os()
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
    use std::error::Error as _;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
    }

    struct TempEnvPath {
        path: std::path::PathBuf,
    }

    impl TempEnvPath {
        fn file(name: &str, contents: &str) -> Self {
            let mut path = std::env::temp_dir();
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            path.push(format!("dotenvpp-lib-test-{}-{nanos}-{name}", std::process::id()));
            fs::write(&path, contents).unwrap();
            Self {
                path,
            }
        }

        fn directory() -> Self {
            let mut path = std::env::temp_dir();
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            path.push(format!("dotenvpp-lib-dir-{}-{nanos}", std::process::id()));
            fs::create_dir(&path).unwrap();
            Self {
                path,
            }
        }
    }

    impl Drop for TempEnvPath {
        fn drop(&mut self) {
            if self.path.is_dir() {
                let _ = fs::remove_dir_all(&self.path);
            } else {
                let _ = fs::remove_file(&self.path);
            }
        }
    }

    /// RAII guard that restores (or removes) an env var on drop.
    /// Ensures test env mutations are cleaned up even on panic.
    struct TempEnvVar {
        key: String,
        prev: Option<std::ffi::OsString>,
    }

    impl TempEnvVar {
        fn new(key: &str) -> Self {
            let prev = env::var_os(key);
            Self {
                key: key.to_owned(),
                prev,
            }
        }
    }

    impl Drop for TempEnvVar {
        fn drop(&mut self) {
            match &self.prev {
                // SAFETY: restoring env to its previous state during test cleanup.
                Some(val) => unsafe { env::set_var(&self.key, val) },
                None => unsafe { env::remove_var(&self.key) },
            }
        }
    }

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

        let _guard = test_lock();
        let key = "DOTENVPP_NON_UNICODE_KEY";
        let _env_guard = TempEnvVar::new(key);
        let file = TempEnvPath::file("non-unicode.env", &format!("{key}=overridden\n"));

        let value = OsString::from_vec(vec![0x66, 0x80]);

        // SAFETY: test setup for a temporary env var.
        unsafe { std::env::set_var(key, &value) };

        let pairs = from_path(&file.path).unwrap();
        assert_eq!(pairs[0].key, key);
        assert_eq!(std::env::var_os(key).unwrap(), value);
    }

    #[test]
    fn test_error_display_and_sources() {
        let io = Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "missing file"));
        assert!(format!("{io}").contains("I/O error"));
        assert!(io.source().is_some());

        let parse = Error::Parse(ParseError::InvalidKey {
            line: 2,
            key: "BAD-KEY".into(),
        });
        assert!(format!("{parse}").contains("parse error"));
        assert!(parse.source().is_some());

        let unicode = Error::NotUnicode("BAD".into());
        assert!(format!("{unicode}").contains("invalid unicode"));
        assert!(unicode.source().is_none());
    }

    #[test]
    fn test_from_conversions() {
        let io = Error::from(std::io::Error::other("boom"));
        assert!(matches!(io, Error::Io(_)));

        let parse = Error::from(ParseError::EmptyKey {
            line: 1,
        });
        assert!(matches!(parse, Error::Parse(_)));
    }

    #[test]
    fn test_from_path_sets_missing_vars_only() {
        let _guard = test_lock();
        let key = "DOTENVPP_LIB_FROM_PATH";
        let _env_guard = TempEnvVar::new(key);
        let file = TempEnvPath::file("from-path.env", &format!("{key}=from_file\n"));

        // SAFETY: test setup for a unique env var key.
        unsafe { env::remove_var(key) };
        let pairs = from_path(&file.path).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(var(key).unwrap(), "from_file");

        // SAFETY: test setup for a unique env var key.
        unsafe { env::set_var(key, "existing") };
        let pairs = from_path(&file.path).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(var(key).unwrap(), "existing");
    }

    #[test]
    fn test_from_path_override_replaces_existing_vars() {
        let _guard = test_lock();
        let key = "DOTENVPP_LIB_OVERRIDE";
        let _env_guard = TempEnvVar::new(key);
        let file = TempEnvPath::file("override.env", &format!("{key}=from_file\n"));

        // SAFETY: test setup for a unique env var key.
        unsafe { env::set_var(key, "old_value") };
        let pairs = from_path_override(&file.path).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(var(key).unwrap(), "from_file");
    }

    #[test]
    fn test_from_path_iter_does_not_set_env() {
        let _guard = test_lock();
        let key = "DOTENVPP_LIB_ITER";
        let _env_guard = TempEnvVar::new(key);
        let file = TempEnvPath::file("iter.env", &format!("{key}=preview_only\n"));

        // SAFETY: test setup for a unique env var key.
        unsafe { env::remove_var(key) };
        let pairs: Vec<_> = from_path_iter(&file.path).unwrap().collect();
        assert_eq!(pairs.len(), 1);
        assert!(env::var(key).is_err());
    }

    #[test]
    fn test_var_success_and_vars_snapshot() {
        let _guard = test_lock();
        let key = "DOTENVPP_LIB_VAR";
        let _env_guard = TempEnvVar::new(key);

        // SAFETY: test setup for a unique env var key.
        unsafe { env::set_var(key, "visible") };
        assert_eq!(var(key).unwrap(), "visible");
        assert!(vars().any(|(name, value)| name == key && value == "visible"));
    }

    #[test]
    fn test_load_and_load_override_use_dotenv_in_current_dir() {
        struct CwdRestore {
            orig: std::path::PathBuf,
        }

        impl Drop for CwdRestore {
            fn drop(&mut self) {
                let _ = env::set_current_dir(&self.orig);
            }
        }

        let _guard = test_lock();
        let key = "DOTENVPP_LIB_LOAD";
        let _env_guard = TempEnvVar::new(key);

        // Declare temp_dir BEFORE cwd guard so cwd restores before temp_dir removal.
        let temp_dir = TempEnvPath::directory();
        let original_dir = env::current_dir().unwrap();
        let _cwd_guard = CwdRestore {
            orig: original_dir,
        };

        let env_path = temp_dir.path.join(".env");
        fs::write(&env_path, format!("{key}=from_dotenv\n")).unwrap();
        env::set_current_dir(&temp_dir.path).unwrap();

        // SAFETY: test setup for a unique env var key.
        unsafe { env::remove_var(key) };
        let loaded = load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(var(key).unwrap(), "from_dotenv");

        fs::write(&env_path, format!("{key}=override_value\n")).unwrap();
        let loaded = load_override().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(var(key).unwrap(), "override_value");
    }
}
