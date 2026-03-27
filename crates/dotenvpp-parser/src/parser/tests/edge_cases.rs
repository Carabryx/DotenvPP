//! Edge case tests.

use alloc::format;
use alloc::string::String;

use crate::error::ParseError;
use crate::parser::parse;

#[test]
fn empty_input() {
    let pairs = parse("").unwrap();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn only_whitespace() {
    let pairs = parse("   \n  \n   ").unwrap();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn only_newlines() {
    let pairs = parse("\n\n\n").unwrap();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn equals_in_double_quoted_value() {
    let pairs = parse("URL=\"host?key=val&foo=bar\"").unwrap();
    assert_eq!(pairs[0].value, "host?key=val&foo=bar");
}

#[test]
fn multiple_equals_unquoted() {
    let pairs = parse("KEY=a=b=c=d").unwrap();
    assert_eq!(pairs[0].value, "a=b=c=d");
}

#[test]
fn unicode_value() {
    let pairs = parse("GREETING=こんにちは").unwrap();
    assert_eq!(pairs[0].value, "こんにちは");
}

#[test]
fn unicode_in_double_quotes() {
    let pairs = parse("GREETING=\"🌍 hello 世界\"").unwrap();
    assert_eq!(pairs[0].value, "🌍 hello 世界");
}

#[test]
fn unicode_in_single_quotes() {
    let pairs = parse("KEY='émoji 🎉'").unwrap();
    assert_eq!(pairs[0].value, "émoji 🎉");
}

#[test]
fn crlf_line_endings() {
    let input = "A=1\r\nB=2\r\nC=3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].value, "1");
    assert_eq!(pairs[1].value, "2");
}

#[test]
fn missing_equals_error() {
    let err = parse("INVALID_LINE").unwrap_err();
    assert!(matches!(
        err,
        ParseError::MissingSeparator {
            line: 1,
            ..
        }
    ));
}

#[test]
fn empty_key_error() {
    let err = parse("=value").unwrap_err();
    assert!(matches!(
        err,
        ParseError::EmptyKey {
            line: 1
        }
    ));
}

#[test]
fn key_starting_with_digit() {
    let err = parse("1KEY=value").unwrap_err();
    assert!(matches!(err, ParseError::InvalidKey { .. }));
}

#[test]
fn key_starting_with_dot() {
    let err = parse(".KEY=value").unwrap_err();
    assert!(matches!(err, ParseError::InvalidKey { .. }));
}

#[test]
fn key_with_spaces() {
    // `MY KEY=value` → key is "MY KEY" which has a space → InvalidKey.
    let err = parse("MY KEY=value").unwrap_err();
    assert!(matches!(err, ParseError::InvalidKey { .. }));
}

#[test]
fn key_with_hyphen() {
    let err = parse("MY-KEY=value").unwrap_err();
    assert!(matches!(err, ParseError::InvalidKey { .. }));
}

#[test]
fn windows_path_unquoted() {
    let pairs = parse("PATH=C:\\Users\\test").unwrap();
    assert_eq!(pairs[0].value, "C:\\Users\\test");
}

#[test]
fn url_with_port() {
    let pairs = parse("URL=https://localhost:3000/api").unwrap();
    assert_eq!(pairs[0].value, "https://localhost:3000/api");
}

#[test]
fn json_value_double_quoted() {
    let pairs = parse("JSON=\"{\\\"key\\\": \\\"value\\\"}\"").unwrap();
    assert_eq!(pairs[0].value, "{\"key\": \"value\"}");
}

#[test]
fn very_long_value() {
    let long_val = "x".repeat(10_000);
    let input = format!("KEY={}", long_val);
    let pairs = parse(&input).unwrap();
    assert_eq!(pairs[0].value.len(), 10_000);
}

#[test]
fn many_pairs() {
    let mut input = String::new();
    for i in 0..500 {
        input.push_str(&format!("KEY_{}={}\n", i, i));
    }
    let pairs = parse(&input).unwrap();
    assert_eq!(pairs.len(), 500);
}

#[test]
fn value_with_leading_whitespace_quoted() {
    let pairs = parse("KEY=\"  leading\"").unwrap();
    assert_eq!(pairs[0].value, "  leading");
}

#[test]
fn bom_prefix_is_ignored() {
    let pairs = parse("\u{feff}KEY=value").unwrap();
    assert_eq!(pairs[0].key, "KEY");
    assert_eq!(pairs[0].value, "value");
}
