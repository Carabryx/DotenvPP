//! Export prefix tests.

use crate::error::ParseError;
use crate::parser::parse;

#[test]
fn basic() {
    let pairs = parse("export KEY=value").unwrap();
    assert_eq!(pairs[0].key, "KEY");
    assert_eq!(pairs[0].value, "value");
}

#[test]
fn with_double_quotes() {
    let pairs = parse("export KEY=\"hello world\"").unwrap();
    assert_eq!(pairs[0].value, "hello world");
}

#[test]
fn with_single_quotes() {
    let pairs = parse("export KEY='hello world'").unwrap();
    assert_eq!(pairs[0].value, "hello world");
}

#[test]
fn with_extra_spaces() {
    let pairs = parse("export   KEY=value").unwrap();
    assert_eq!(pairs[0].key, "KEY");
}

#[test]
fn with_tab() {
    let pairs = parse("export\tKEY=value").unwrap();
    assert_eq!(pairs[0].key, "KEY");
}

#[test]
fn mixed_with_regular() {
    let input = "export A=1\nB=2\nexport C=3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].key, "A");
    assert_eq!(pairs[1].key, "B");
    assert_eq!(pairs[2].key, "C");
}

#[test]
fn empty_value() {
    let pairs = parse("export KEY=").unwrap();
    assert_eq!(pairs[0].key, "KEY");
    assert_eq!(pairs[0].value, "");
}

#[test]
fn key_named_export_value() {
    let pairs = parse("exportKEY=value").unwrap();
    assert_eq!(pairs[0].key, "exportKEY");
}

#[test]
fn as_value_content() {
    let pairs = parse("KEY=export").unwrap();
    assert_eq!(pairs[0].value, "export");
}

#[test]
fn uppercase_not_matched() {
    // `EXPORT KEY=value` → key is "EXPORT KEY" which has a space → InvalidKey.
    let err = parse("EXPORT KEY=value").unwrap_err();
    assert!(matches!(err, ParseError::InvalidKey { .. }));
}
