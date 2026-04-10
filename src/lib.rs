//! # DotenvPP
//!
//! From-scratch `.env` parsing, interpolation, and layered loading for Rust.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! dotenvpp::load().ok();
//! let db_url = dotenvpp::var("DATABASE_URL").unwrap();
//! ```
//!
//! ## API Overview
//!
//! | Function | Description |
//! |---|---|
//! | [`load()`] | Load layered `.env` files from cwd without overriding existing vars |
//! | [`load_override()`] | Load layered `.env` files overriding existing vars |
//! | [`load_with_env()`] | Load layered files for a named environment |
//! | [`from_layered_env()`] | Preview layered config without mutating the process env |
//! | [`from_path()`] | Load and resolve a specific file |
//! | [`from_read()`] | Parse and resolve from any `impl Read` |
//! | [`var()`] | Get a single env var |
//! | [`vars()`] | Iterate all env vars |
//! | [`vars_os()`] | Iterate env vars without Unicode conversion |

use std::collections::{HashMap, HashSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub use dotenvpp_parser::{EnvPair, ParseError};

/// Errors that can occur when loading `.env` files.
#[derive(Debug)]
pub enum Error {
    /// An I/O error (file not found, permission denied, etc.).
    Io(std::io::Error),
    /// A parse error in the `.env` content.
    Parse(ParseError),
    /// Interpolation failed after parsing.
    Interpolation(InterpolationError),
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
            Self::Interpolation(err) => write!(f, "interpolation error: {err}"),
            Self::NotPresent(key) => write!(f, "environment variable `{key}` not found"),
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
            Self::Interpolation(err) => Some(err),
            Self::NotPresent(_) | Self::NotUnicode(_) => None,
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

impl From<InterpolationError> for Error {
    fn from(err: InterpolationError) -> Self {
        Self::Interpolation(err)
    }
}

/// Interpolation failures for `${VAR}` style expansions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterpolationError {
    /// The key whose value was being expanded.
    pub key: String,
    /// The 1-based line where the key was defined.
    pub line: usize,
    /// The source file when known.
    pub source: Option<PathBuf>,
    /// The specific interpolation failure.
    pub kind: InterpolationErrorKind,
}

impl std::fmt::Display for InterpolationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(source) = &self.source {
            write!(f, "{}:{} for key `{}`: {}", source.display(), self.line, self.key, self.kind)
        } else {
            write!(f, "line {} for key `{}`: {}", self.line, self.key, self.kind)
        }
    }
}

impl std::error::Error for InterpolationError {}

/// Specific interpolation failure kinds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpolationErrorKind {
    /// A `${VAR:?message}` or `${VAR?message}` expansion failed.
    MissingRequiredVariable {
        /// The missing variable name.
        variable: String,
        /// The final error message after nested interpolation, when any.
        message: String,
    },
    /// A cycle was found while recursively expanding values.
    CircularReference {
        /// The cycle path, including the repeated key at the end.
        cycle: Vec<String>,
    },
    /// `${...}` contained invalid syntax.
    InvalidSyntax {
        /// The offending inner expression.
        expression: String,
        /// A short explanation of what was invalid.
        reason: &'static str,
    },
}

impl std::fmt::Display for InterpolationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredVariable {
                variable,
                message,
            } => {
                write!(f, "variable `{variable}` is required")?;
                if !message.is_empty() {
                    write!(f, ": {message}")?;
                }
                Ok(())
            }
            Self::CircularReference {
                cycle,
            } => {
                write!(f, "circular reference detected: {}", cycle.join(" -> "))
            }
            Self::InvalidSyntax {
                expression,
                reason,
            } => {
                write!(f, "invalid `${{{expression}}}` expression: {reason}")
            }
        }
    }
}

/// Convenience type alias.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
struct LoadedEntry {
    key: String,
    raw_value: String,
    line: usize,
    source: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpansionMode {
    Basic,
    DefaultIfUnsetOrEmpty,
    DefaultIfUnset,
    AlternativeIfSetAndNotEmpty,
    AlternativeIfSet,
    RequiredIfUnsetOrEmpty,
    RequiredIfUnset,
}

#[derive(Debug, Clone, Copy)]
struct Expansion<'a> {
    name: &'a str,
    mode: ExpansionMode,
    suffix: &'a str,
}

struct Resolver<'a> {
    entries: &'a [LoadedEntry],
    entry_index: HashMap<&'a str, usize>,
    env_snapshot: HashMap<String, String>,
    cache: HashMap<usize, String>,
    stack: Vec<usize>,
}

impl<'a> Resolver<'a> {
    fn new(entries: &'a [LoadedEntry]) -> Self {
        let entry_index = entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| (entry.key.as_str(), idx))
            .collect();

        Self {
            entries,
            entry_index,
            env_snapshot: env::vars().collect(),
            cache: HashMap::with_capacity(entries.len()),
            stack: Vec::with_capacity(entries.len()),
        }
    }

    fn resolve_all(mut self) -> Result<Vec<EnvPair>> {
        let mut pairs = Vec::with_capacity(self.entries.len());
        for idx in 0..self.entries.len() {
            let value = self.resolve_entry(idx)?;
            let entry = &self.entries[idx];
            pairs.push(EnvPair {
                key: entry.key.clone(),
                value,
                line: entry.line,
            });
        }
        Ok(pairs)
    }

    fn resolve_entry(&mut self, idx: usize) -> Result<String> {
        if let Some(value) = self.cache.get(&idx) {
            return Ok(value.clone());
        }

        if let Some(position) = self.stack.iter().position(|active| *active == idx) {
            let mut cycle = self.stack[position..]
                .iter()
                .map(|active| self.entries[*active].key.clone())
                .collect::<Vec<_>>();
            cycle.push(self.entries[idx].key.clone());

            return Err(self.error(
                idx,
                InterpolationErrorKind::CircularReference {
                    cycle,
                },
            ));
        }

        self.stack.push(idx);
        let value = self.expand_text(idx, &self.entries[idx].raw_value)?;
        self.stack.pop();
        self.cache.insert(idx, value.clone());
        Ok(value)
    }

    fn expand_text(&mut self, idx: usize, raw: &str) -> Result<String> {
        let mut expanded = String::with_capacity(raw.len());
        let mut cursor = 0;

        while cursor < raw.len() {
            let tail = &raw[cursor..];

            if tail.starts_with("$$") {
                expanded.push('$');
                cursor += 2;
                continue;
            }

            if tail.starts_with("${") {
                let (inner, next_cursor) = take_expansion(raw, cursor)
                    .map_err(|reason| self.syntax_error(idx, &raw[cursor + 2..], reason))?;
                expanded.push_str(&self.expand_expression(idx, inner)?);
                cursor = next_cursor;
                continue;
            }

            let Some(ch) = tail.chars().next() else {
                break;
            };
            expanded.push(ch);
            cursor += ch.len_utf8();
        }

        Ok(expanded)
    }

    fn expand_expression(&mut self, idx: usize, expression: &str) -> Result<String> {
        let expansion = parse_expansion(expression)
            .map_err(|reason| self.syntax_error(idx, expression, reason))?;
        let value = self.lookup(expansion.name)?;

        match expansion.mode {
            ExpansionMode::Basic => Ok(value.unwrap_or_default()),
            ExpansionMode::DefaultIfUnsetOrEmpty => match value {
                Some(resolved) if !resolved.is_empty() => Ok(resolved),
                _ => self.expand_text(idx, expansion.suffix),
            },
            ExpansionMode::DefaultIfUnset => {
                if let Some(resolved) = value {
                    Ok(resolved)
                } else {
                    self.expand_text(idx, expansion.suffix)
                }
            }
            ExpansionMode::AlternativeIfSetAndNotEmpty => {
                if value.as_deref().is_some_and(|resolved| !resolved.is_empty()) {
                    self.expand_text(idx, expansion.suffix)
                } else {
                    Ok(String::new())
                }
            }
            ExpansionMode::AlternativeIfSet => {
                if value.is_some() {
                    self.expand_text(idx, expansion.suffix)
                } else {
                    Ok(String::new())
                }
            }
            ExpansionMode::RequiredIfUnsetOrEmpty => match value {
                Some(resolved) if !resolved.is_empty() => Ok(resolved),
                _ => {
                    let message = self.expand_required_message(idx, expansion)?;
                    Err(self.error(
                        idx,
                        InterpolationErrorKind::MissingRequiredVariable {
                            variable: expansion.name.to_owned(),
                            message,
                        },
                    ))
                }
            },
            ExpansionMode::RequiredIfUnset => {
                if let Some(resolved) = value {
                    Ok(resolved)
                } else {
                    let message = self.expand_required_message(idx, expansion)?;
                    Err(self.error(
                        idx,
                        InterpolationErrorKind::MissingRequiredVariable {
                            variable: expansion.name.to_owned(),
                            message,
                        },
                    ))
                }
            }
        }
    }

    fn expand_required_message(&mut self, idx: usize, expansion: Expansion<'_>) -> Result<String> {
        if expansion.suffix.is_empty() {
            Ok(String::new())
        } else {
            self.expand_text(idx, expansion.suffix)
        }
    }

    fn lookup(&mut self, name: &str) -> Result<Option<String>> {
        if let Some(idx) = self.entry_index.get(name) {
            return self.resolve_entry(*idx).map(Some);
        }

        Ok(self.env_snapshot.get(name).cloned())
    }

    fn error(&self, idx: usize, kind: InterpolationErrorKind) -> Error {
        Error::Interpolation(InterpolationError {
            key: self.entries[idx].key.clone(),
            line: self.entries[idx].line,
            source: self.entries[idx].source.clone(),
            kind,
        })
    }

    fn syntax_error(&self, idx: usize, expression: &str, reason: &'static str) -> Error {
        self.error(
            idx,
            InterpolationErrorKind::InvalidSyntax {
                expression: expression.to_owned(),
                reason,
            },
        )
    }
}

/// Load layered environment files from the current directory.
///
/// Existing process variables are preserved. When no environment is
/// selected, DotenvPP loads `.env` and `.env.local` if present.
pub fn load() -> Result<Vec<EnvPair>> {
    let pairs = from_layered_env(None)?;
    apply_pairs(&pairs, false);
    Ok(pairs)
}

/// Load layered environment files from the current directory and
/// override existing process variables.
pub fn load_override() -> Result<Vec<EnvPair>> {
    let pairs = from_layered_env(None)?;
    apply_pairs(&pairs, true);
    Ok(pairs)
}

/// Load layered environment files for a specific environment name.
///
/// Layering order follows dotenvx-style precedence:
/// `.env` < `.env.{ENV}` < `.env.local` < `.env.{ENV}.local`.
pub fn load_with_env(environment: &str) -> Result<Vec<EnvPair>> {
    let pairs = from_layered_env(Some(environment))?;
    apply_pairs(&pairs, false);
    Ok(pairs)
}

/// Load layered environment files for a specific environment name and
/// override existing process variables.
pub fn load_with_env_override(environment: &str) -> Result<Vec<EnvPair>> {
    let pairs = from_layered_env(Some(environment))?;
    apply_pairs(&pairs, true);
    Ok(pairs)
}

/// Resolve layered `.env` files from the current directory without
/// mutating the process environment.
pub fn from_layered_env(environment: Option<&str>) -> Result<Vec<EnvPair>> {
    resolve_layered_from_dir(Path::new("."), environment)
}

/// Load environment variables from a specific file path.
///
/// Existing environment variables are preserved.
pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Vec<EnvPair>> {
    let pairs = resolve_entries(read_entries_from_path(path.as_ref())?)?;
    apply_pairs(&pairs, false);
    Ok(pairs)
}

/// Load environment variables from a specific file path and override
/// existing process variables.
pub fn from_path_override<P: AsRef<Path>>(path: P) -> Result<Vec<EnvPair>> {
    let pairs = resolve_entries(read_entries_from_path(path.as_ref())?)?;
    apply_pairs(&pairs, true);
    Ok(pairs)
}

/// Resolve a `.env` file and return an iterator over the final key/value
/// pairs without setting them in the environment.
pub fn from_path_iter<P: AsRef<Path>>(path: P) -> Result<impl Iterator<Item = EnvPair>> {
    let pairs = resolve_entries(read_entries_from_path(path.as_ref())?)?;
    Ok(pairs.into_iter())
}

/// Parse `.env` content from any reader without setting env vars.
///
/// Duplicate keys are merged with last-assignment-wins semantics before
/// interpolation is evaluated.
pub fn from_read<R: Read>(mut reader: R) -> Result<Vec<EnvPair>> {
    let mut content = String::new();
    reader.read_to_string(&mut content)?;
    let pairs = dotenvpp_parser::parse(&content)?;
    resolve_entries(merge_entries([(None, pairs)]))
}

/// Get a single environment variable's value.
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
pub fn vars() -> env::Vars {
    env::vars()
}

/// Returns an iterator over all environment variables as
/// `(OsString, OsString)` pairs.
pub fn vars_os() -> env::VarsOs {
    env::vars_os()
}

/// Returns the crate version.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn apply_pairs(pairs: &[EnvPair], override_existing: bool) {
    if override_existing {
        for pair in pairs {
            // SAFETY: intended use is process configuration at startup.
            unsafe { env::set_var(&pair.key, &pair.value) };
        }
        return;
    }

    let existing_keys: HashSet<OsString> = env::vars_os().map(|(key, _)| key).collect();
    for pair in pairs {
        if !existing_keys.contains(OsStr::new(&pair.key)) {
            // SAFETY: intended use is process configuration at startup.
            unsafe { env::set_var(&pair.key, &pair.value) };
        }
    }
}

fn resolve_layered_from_dir(dir: &Path, environment: Option<&str>) -> Result<Vec<EnvPair>> {
    let mut groups = Vec::new();
    let mut found_any = false;
    let paths = layered_paths(dir, environment);

    for path in &paths {
        if let Some(entries) = maybe_read_entries_from_path(path)? {
            found_any = true;
            groups.push((Some(path.clone()), entries));
        }
    }

    if !found_any {
        let missing = paths.first().cloned().unwrap_or_else(|| dir.join(".env"));
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("no environment files found starting at {}", missing.display()),
        )
        .into());
    }

    resolve_entries(merge_entries(groups))
}

fn layered_paths(dir: &Path, environment: Option<&str>) -> Vec<PathBuf> {
    let mut paths = vec![dir.join(".env")];

    if let Some(environment) = environment.filter(|value| !value.is_empty()) {
        paths.push(dir.join(format!(".env.{environment}")));
    }

    paths.push(dir.join(".env.local"));

    if let Some(environment) = environment.filter(|value| !value.is_empty()) {
        paths.push(dir.join(format!(".env.{environment}.local")));
    }

    paths
}

fn maybe_read_entries_from_path(path: &Path) -> Result<Option<Vec<EnvPair>>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(dotenvpp_parser::parse(&content)?)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn read_entries_from_path(path: &Path) -> Result<Vec<LoadedEntry>> {
    let content = fs::read_to_string(path)?;
    let pairs = dotenvpp_parser::parse(&content)?;
    Ok(merge_entries([(Some(path.to_path_buf()), pairs)]))
}

fn resolve_entries(entries: Vec<LoadedEntry>) -> Result<Vec<EnvPair>> {
    Resolver::new(&entries).resolve_all()
}

fn merge_entries<I>(groups: I) -> Vec<LoadedEntry>
where
    I: IntoIterator<Item = (Option<PathBuf>, Vec<EnvPair>)>,
{
    let mut merged = Vec::new();
    let mut positions = HashMap::new();

    for (source, pairs) in groups {
        for pair in pairs {
            let entry = LoadedEntry {
                key: pair.key,
                raw_value: pair.value,
                line: pair.line,
                source: source.clone(),
            };

            if let Some(position) = positions.get(&entry.key) {
                merged[*position] = entry;
            } else {
                positions.insert(entry.key.clone(), merged.len());
                merged.push(entry);
            }
        }
    }

    merged
}

fn take_expansion(
    raw: &str,
    dollar_index: usize,
) -> std::result::Result<(&str, usize), &'static str> {
    let mut depth = 1;
    let mut cursor = dollar_index + 2;

    while cursor < raw.len() {
        let tail = &raw[cursor..];

        if tail.starts_with("${") {
            depth += 1;
            cursor += 2;
            continue;
        }

        let Some(ch) = tail.chars().next() else {
            break;
        };
        if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Ok((&raw[dollar_index + 2..cursor], cursor + 1));
            }
        }

        cursor += ch.len_utf8();
    }

    Err("missing closing `}`")
}

fn parse_expansion(expression: &str) -> std::result::Result<Expansion<'_>, &'static str> {
    if expression.is_empty() {
        return Err("variable name is empty");
    }

    let name_end = expression
        .char_indices()
        .find_map(|(idx, ch)| matches!(ch, ':' | '-' | '?' | '+').then_some(idx))
        .unwrap_or(expression.len());

    let name = &expression[..name_end];
    if !is_valid_var_name(name) {
        return Err("variable name is invalid");
    }

    let suffix = &expression[name_end..];
    if suffix.is_empty() {
        return Ok(Expansion {
            name,
            mode: ExpansionMode::Basic,
            suffix: "",
        });
    }

    if let Some(value) = suffix.strip_prefix(":-") {
        return Ok(Expansion {
            name,
            mode: ExpansionMode::DefaultIfUnsetOrEmpty,
            suffix: value,
        });
    }

    if let Some(value) = suffix.strip_prefix(":+") {
        return Ok(Expansion {
            name,
            mode: ExpansionMode::AlternativeIfSetAndNotEmpty,
            suffix: value,
        });
    }

    if let Some(value) = suffix.strip_prefix(":?") {
        return Ok(Expansion {
            name,
            mode: ExpansionMode::RequiredIfUnsetOrEmpty,
            suffix: value,
        });
    }

    if let Some(value) = suffix.strip_prefix('-') {
        return Ok(Expansion {
            name,
            mode: ExpansionMode::DefaultIfUnset,
            suffix: value,
        });
    }

    if let Some(value) = suffix.strip_prefix('+') {
        return Ok(Expansion {
            name,
            mode: ExpansionMode::AlternativeIfSet,
            suffix: value,
        });
    }

    if let Some(value) = suffix.strip_prefix('?') {
        return Ok(Expansion {
            name,
            mode: ExpansionMode::RequiredIfUnset,
            suffix: value,
        });
    }

    Err("unsupported interpolation operator")
}

fn is_valid_var_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.')
}
