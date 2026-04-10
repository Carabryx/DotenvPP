#![allow(clippy::panic, clippy::unwrap_used)]

use dotenvpp::{Error, InterpolationError, InterpolationErrorKind, ParseError};
use std::env;
use std::error::Error as _;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|e| e.into_inner())
}

struct TempEnvPath {
    path: PathBuf,
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
            Some(val) => unsafe { env::set_var(&self.key, val) },
            None => unsafe { env::remove_var(&self.key) },
        }
    }
}

struct CwdRestore {
    orig: PathBuf,
}

impl CwdRestore {
    fn capture() -> Self {
        Self {
            orig: env::current_dir().unwrap(),
        }
    }
}

impl Drop for CwdRestore {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.orig);
    }
}

fn write_env_file(dir: &TempEnvPath, name: &str, contents: &str) {
    fs::write(dir.path.join(name), contents).unwrap();
}

#[test]
fn test_version() {
    assert_eq!(dotenvpp::version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_from_read() {
    let input = b"KEY=value\nNAME=\"hello\"";
    let pairs = dotenvpp::from_read(&input[..]).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].key, "KEY");
    assert_eq!(pairs[0].value, "value");
    assert_eq!(pairs[1].key, "NAME");
    assert_eq!(pairs[1].value, "hello");
}

#[test]
fn test_from_read_interpolates_nested_values() {
    let input = br#"
HOST=localhost
PORT=8080
BASE_URL=http://${HOST}:${PORT}
USERS_URL=${BASE_URL}/users
"#;
    let pairs = dotenvpp::from_read(&input[..]).unwrap();
    assert_eq!(pairs.len(), 4);
    assert_eq!(pairs[2].value, "http://localhost:8080");
    assert_eq!(pairs[3].value, "http://localhost:8080/users");
}

#[test]
fn test_from_read_supports_shell_style_parameter_variants() {
    let input = br#"
SET=hello
EMPTY=
DEFAULT=${MISSING:-fallback}
DEFAULT_UNSET_ONLY=${EMPTY-fallback}
ALT_SET=${SET:+present}
ALT_EMPTY=${EMPTY:+present}
ALT_UNSET_ONLY=${EMPTY+present}
NESTED=${MISSING:-${SET}-world}
PRICE=$$5
MISSING=${NOT_SET}
"#;
    let pairs = dotenvpp::from_read(&input[..]).unwrap();
    assert_eq!(pairs[2].value, "fallback");
    assert_eq!(pairs[3].value, "");
    assert_eq!(pairs[4].value, "present");
    assert_eq!(pairs[5].value, "");
    assert_eq!(pairs[6].value, "present");
    assert_eq!(pairs[7].value, "hello-world");
    assert_eq!(pairs[8].value, "$5");
    assert_eq!(pairs[9].value, "");
}

#[test]
fn test_from_read_uses_current_process_env_as_fallback() {
    let _guard = test_lock();
    let key = "DOTENVPP_LIB_FALLBACK";
    let _env_guard = TempEnvVar::new(key);

    unsafe { env::set_var(key, "from_process") };

    let input = format!("VALUE=${{{key}}}\n");
    let pairs = dotenvpp::from_read(input.as_bytes()).unwrap();
    assert_eq!(pairs[0].value, "from_process");
}

#[test]
fn test_from_read_required_operator_reports_message() {
    let err = dotenvpp::from_read(b"API_KEY=${MISSING:?set API_KEY before boot}\n".as_slice())
        .unwrap_err();

    match err {
        Error::Interpolation(InterpolationError {
            key,
            line,
            kind:
                InterpolationErrorKind::MissingRequiredVariable {
                    variable,
                    message,
                },
            ..
        }) => {
            assert_eq!(key, "API_KEY");
            assert_eq!(line, 1);
            assert_eq!(variable, "MISSING");
            assert_eq!(message, "set API_KEY before boot");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn test_from_read_required_operator_without_message_uses_default_display() {
    let err = dotenvpp::from_read(b"API_KEY=${MISSING:?}\n".as_slice()).unwrap_err();
    let rendered = format!("{err}");

    match err {
        Error::Interpolation(InterpolationError {
            kind:
                InterpolationErrorKind::MissingRequiredVariable {
                    variable,
                    message,
                },
            ..
        }) => {
            assert_eq!(variable, "MISSING");
            assert!(message.is_empty());
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert!(rendered.contains("variable `MISSING` is required"));
    assert_eq!(rendered.matches("variable `MISSING` is required").count(), 1);
}

#[test]
fn test_from_read_detects_circular_references() {
    let err = dotenvpp::from_read(b"A=${B}\nB=${C}\nC=${A}\n".as_slice()).unwrap_err();

    match err {
        Error::Interpolation(InterpolationError {
            kind: InterpolationErrorKind::CircularReference {
                cycle,
            },
            ..
        }) => {
            assert_eq!(cycle, vec!["A", "B", "C", "A"]);
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn test_from_read_shadowed_duplicate_does_not_trigger_cycle() {
    let pairs = dotenvpp::from_read(b"A=${A}\nA=stable\nB=${A}\n".as_slice()).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].key, "A");
    assert_eq!(pairs[0].value, "stable");
    assert_eq!(pairs[1].value, "stable");
}

#[test]
fn test_from_read_reports_invalid_interpolation_syntax() {
    let err = dotenvpp::from_read(b"VALUE=${1BAD}\n".as_slice()).unwrap_err();

    match err {
        Error::Interpolation(InterpolationError {
            key,
            line,
            kind:
                InterpolationErrorKind::InvalidSyntax {
                    expression,
                    reason,
                },
            ..
        }) => {
            assert_eq!(key, "VALUE");
            assert_eq!(line, 1);
            assert_eq!(expression, "1BAD");
            assert_eq!(reason, "variable name is invalid");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn test_from_read_reports_unclosed_interpolation_against_current_key() {
    let err = dotenvpp::from_read(b"VALUE=${MISSING\n".as_slice()).unwrap_err();

    match err {
        Error::Interpolation(InterpolationError {
            key,
            line,
            kind:
                InterpolationErrorKind::InvalidSyntax {
                    reason,
                    ..
                },
            ..
        }) => {
            assert_eq!(key, "VALUE");
            assert_eq!(line, 1);
            assert_eq!(reason, "missing closing `}`");
        }
        other => panic!("unexpected error: {other:?}"),
    }
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

    unsafe { std::env::set_var(key, &value) };

    let pairs = dotenvpp::from_path(&file.path).unwrap();
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

    let interpolation = Error::from(InterpolationError {
        key: "VALUE".into(),
        line: 1,
        source: None,
        kind: InterpolationErrorKind::InvalidSyntax {
            expression: "BAD".into(),
            reason: "variable name is invalid",
        },
    });
    assert!(matches!(interpolation, Error::Interpolation(_)));
}

#[test]
fn test_from_path_sets_missing_vars_only() {
    let _guard = test_lock();
    let key = "DOTENVPP_LIB_FROM_PATH";
    let _env_guard = TempEnvVar::new(key);
    let file = TempEnvPath::file("from-path.env", &format!("{key}=from_file\n"));

    unsafe { env::remove_var(key) };
    let pairs = dotenvpp::from_path(&file.path).unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(dotenvpp::var(key).unwrap(), "from_file");

    unsafe { env::set_var(key, "existing") };
    let pairs = dotenvpp::from_path(&file.path).unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(dotenvpp::var(key).unwrap(), "existing");
}

#[test]
fn test_from_path_override_replaces_existing_vars() {
    let _guard = test_lock();
    let key = "DOTENVPP_LIB_OVERRIDE";
    let _env_guard = TempEnvVar::new(key);
    let file = TempEnvPath::file("override.env", &format!("{key}=from_file\n"));

    unsafe { env::set_var(key, "old_value") };
    let pairs = dotenvpp::from_path_override(&file.path).unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(dotenvpp::var(key).unwrap(), "from_file");
}

#[test]
fn test_from_path_iter_does_not_set_env() {
    let _guard = test_lock();
    let key = "DOTENVPP_LIB_ITER";
    let _env_guard = TempEnvVar::new(key);
    let file = TempEnvPath::file("iter.env", &format!("{key}=preview_only\n"));

    unsafe { env::remove_var(key) };
    let pairs: Vec<_> = dotenvpp::from_path_iter(&file.path).unwrap().collect();
    assert_eq!(pairs.len(), 1);
    assert!(env::var(key).is_err());
}

#[test]
fn test_var_success_and_vars_snapshot() {
    let _guard = test_lock();
    let key = "DOTENVPP_LIB_VAR";
    let _env_guard = TempEnvVar::new(key);

    unsafe { env::set_var(key, "visible") };
    assert_eq!(dotenvpp::var(key).unwrap(), "visible");
    assert!(dotenvpp::vars().any(|(name, value)| name == key && value == "visible"));
}

#[test]
fn test_load_and_load_override_use_dotenv_in_current_dir() {
    let _guard = test_lock();
    let key = "DOTENVPP_LIB_LOAD";
    let _env_guard = TempEnvVar::new(key);
    let temp_dir = TempEnvPath::directory();
    let _cwd_guard = CwdRestore::capture();

    let env_path = temp_dir.path.join(".env");
    fs::write(&env_path, format!("{key}=from_dotenv\n")).unwrap();
    env::set_current_dir(&temp_dir.path).unwrap();

    unsafe { env::remove_var(key) };
    let loaded = dotenvpp::load().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(dotenvpp::var(key).unwrap(), "from_dotenv");

    fs::write(&env_path, format!("{key}=override_value\n")).unwrap();
    let loaded = dotenvpp::load_override().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(dotenvpp::var(key).unwrap(), "override_value");
}

#[test]
fn test_load_uses_layered_precedence_and_cross_layer_interpolation() {
    let _guard = test_lock();
    let key = "DOTENVPP_LAYERED_KEY";
    let target_key = "DOTENVPP_LAYERED_TARGET";
    let _key_guard = TempEnvVar::new(key);
    let _target_guard = TempEnvVar::new(target_key);
    let temp_dir = TempEnvPath::directory();
    let _cwd_guard = CwdRestore::capture();

    write_env_file(&temp_dir, ".env", &format!("{key}=base\n"));
    write_env_file(&temp_dir, ".env.production", &format!("{target_key}=${{{key}}}-production\n"));
    write_env_file(&temp_dir, ".env.local", &format!("{key}=local\n"));
    write_env_file(&temp_dir, ".env.production.local", &format!("{key}=final\n"));
    env::set_current_dir(&temp_dir.path).unwrap();

    unsafe {
        env::remove_var(key);
        env::remove_var(target_key);
    }

    let pairs = dotenvpp::load_with_env("production").unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].key, key);
    assert_eq!(pairs[0].value, "final");
    assert_eq!(pairs[1].key, target_key);
    assert_eq!(pairs[1].value, "final-production");
    assert_eq!(dotenvpp::var(key).unwrap(), "final");
    assert_eq!(dotenvpp::var(target_key).unwrap(), "final-production");
}

#[test]
fn test_load_preserves_existing_process_values_but_load_override_replaces_them() {
    let _guard = test_lock();
    let key = "DOTENVPP_LAYERED_EXISTING";
    let _env_guard = TempEnvVar::new(key);
    let temp_dir = TempEnvPath::directory();
    let _cwd_guard = CwdRestore::capture();

    write_env_file(&temp_dir, ".env", &format!("{key}=from_file\n"));
    env::set_current_dir(&temp_dir.path).unwrap();

    unsafe { env::set_var(key, "from_process") };

    let loaded = dotenvpp::load().unwrap();
    assert_eq!(loaded[0].value, "from_file");
    assert_eq!(dotenvpp::var(key).unwrap(), "from_process");

    let loaded = dotenvpp::load_override().unwrap();
    assert_eq!(loaded[0].value, "from_file");
    assert_eq!(dotenvpp::var(key).unwrap(), "from_file");
}

#[test]
fn test_from_layered_env_errors_when_no_files_exist() {
    let _guard = test_lock();
    let temp_dir = TempEnvPath::directory();
    let _cwd_guard = CwdRestore::capture();
    env::set_current_dir(&temp_dir.path).unwrap();

    let err = dotenvpp::from_layered_env(None).unwrap_err();
    assert!(matches!(err, Error::Io(_)));
    assert!(format!("{err}").contains("no environment files found"));
}

#[test]
fn test_load_with_env_override_sets_and_overrides() {
    let _guard = test_lock();
    let temp_dir = TempEnvPath::directory();
    let _cwd_guard = CwdRestore::capture();
    env::set_current_dir(&temp_dir.path).unwrap();

    write_env_file(&temp_dir, ".env", "BASE=aaa\n");
    write_env_file(&temp_dir, ".env.staging", "BASE=bbb\nEXTRA=ccc\n");

    let _v1 = TempEnvVar::new("BASE");
    let _v2 = TempEnvVar::new("EXTRA");
    unsafe { env::set_var("BASE", "original") };

    let pairs = dotenvpp::load_with_env_override("staging").unwrap();
    assert_eq!(env::var("BASE").unwrap(), "bbb");
    assert_eq!(env::var("EXTRA").unwrap(), "ccc");
    assert!(pairs.iter().any(|p| p.key == "EXTRA"));
}

#[test]
fn test_vars_os_returns_iterator() {
    let count = dotenvpp::vars_os().count();
    assert!(count > 0);
}

#[test]
fn test_default_without_colon_only_triggers_when_unset() {
    let pairs =
        dotenvpp::from_read(b"VAL=\nA=${VAL-fallback}\nB=${NOPE-fallback}\n".as_slice()).unwrap();
    assert_eq!(pairs[1].value, "");
    assert_eq!(pairs[2].value, "fallback");
}

#[test]
fn test_alternative_without_colon_triggers_when_set_even_if_empty() {
    let pairs = dotenvpp::from_read(b"VAL=\nA=${VAL+yes}\nB=${NOPE+yes}\n".as_slice()).unwrap();
    assert_eq!(pairs[1].value, "yes");
    assert_eq!(pairs[2].value, "");
}

#[test]
fn test_required_without_colon_passes_when_empty() {
    let pairs = dotenvpp::from_read(b"VAL=\nA=${VAL?must exist}\n".as_slice()).unwrap();
    assert_eq!(pairs[1].value, "");
}

#[test]
fn test_required_without_colon_errors_when_unset() {
    let err = dotenvpp::from_read(b"A=${NOPE?var is required}\n".as_slice()).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("var is required"));
}

#[test]
fn test_not_unicode_error_display() {
    let err = Error::NotUnicode("BAD_KEY".to_string());
    let msg = format!("{err}");
    assert!(msg.contains("BAD_KEY"));
    assert!(msg.contains("invalid unicode"));
}

#[test]
fn test_interpolation_error_source_is_some() {
    let err = dotenvpp::from_read(b"A=${B}\nB=${A}\n".as_slice()).unwrap_err();
    let source = std::error::Error::source(&err);
    assert!(source.is_some());
}

#[test]
fn test_interpolation_error_display_with_source_file() {
    let err = InterpolationError {
        key: "DB_URL".to_string(),
        line: 5,
        source: Some(PathBuf::from(".env.production")),
        kind: InterpolationErrorKind::MissingRequiredVariable {
            variable: "DB_PASS".to_string(),
            message: "set it".to_string(),
        },
    };
    let msg = format!("{err}");
    assert!(msg.contains(".env.production"));
    assert!(msg.contains("DB_URL"));
    assert!(msg.contains("DB_PASS"));
}

#[test]
fn test_invalid_syntax_kind_display() {
    let kind = InterpolationErrorKind::InvalidSyntax {
        expression: "1BAD".to_string(),
        reason: "invalid variable name",
    };
    let msg = format!("{kind}");
    assert!(msg.contains("1BAD"));
    assert!(msg.contains("invalid variable name"));
}

#[test]
fn test_missing_required_empty_message_display() {
    let kind = InterpolationErrorKind::MissingRequiredVariable {
        variable: "SECRET".to_string(),
        message: String::new(),
    };
    let msg = format!("{kind}");
    assert!(msg.contains("SECRET"));
    assert!(!msg.contains(":"));
}
