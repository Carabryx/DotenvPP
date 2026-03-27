//! Integration tests for the `dotenvpp` facade crate.

use std::io::Cursor;

#[test]
fn test_from_read_basic() {
    let input = b"A=hello\nB=world";
    let pairs = dotenvpp::from_read(Cursor::new(input)).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].key, "A");
    assert_eq!(pairs[0].value, "hello");
    assert_eq!(pairs[1].key, "B");
    assert_eq!(pairs[1].value, "world");
}

#[test]
fn test_from_read_with_comments() {
    let input = b"# comment\nKEY=value\n\n# another\nNAME=\"test\"";
    let pairs = dotenvpp::from_read(Cursor::new(input)).unwrap();
    assert_eq!(pairs.len(), 2);
}

#[test]
fn test_from_read_empty() {
    let input = b"";
    let pairs = dotenvpp::from_read(Cursor::new(input)).unwrap();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn test_from_read_complex_env() {
    let input = br#"
# App settings
APP_NAME=dotenvpp
APP_PORT=8080
DB_URL="postgres://user:pass@localhost/db"
export SECRET='my-secret'
MULTI="line1
line2"
EMPTY=
"#;
    let pairs = dotenvpp::from_read(Cursor::new(input)).unwrap();
    assert_eq!(pairs.len(), 6);
}

#[test]
fn test_from_path_missing_file() {
    let result = dotenvpp::from_path("nonexistent_file_that_does_not_exist.env");
    assert!(result.is_err());
}

#[test]
fn test_from_path_iter_missing_file() {
    let result = dotenvpp::from_path_iter("nonexistent_file_that_does_not_exist.env");
    assert!(result.is_err());
}

#[test]
fn test_version_format() {
    let version = dotenvpp::version();
    assert!(
        version.chars().all(|c| c.is_ascii_digit() || c == '.'),
        "version string should only contain digits and dots: {version}"
    );
}

#[test]
fn test_var_not_present() {
    // Requesting a var that doesn't exist should give NotPresent error.
    let err = dotenvpp::var("DOTENVPP_THIS_KEY_SHOULD_NOT_EXIST");
    assert!(err.is_err());
    let msg = format!("{}", err.unwrap_err());
    assert!(msg.contains("not found"));
}
