//! Single-quoted value tests.

use crate::error::ParseError;
use crate::parser::parse;

#[test]
fn basic() {
    let pairs = parse("KEY='hello world'").unwrap();
    assert_eq!(pairs[0].value, "hello world");
}

#[test]
fn preserves_spaces() {
    let pairs = parse("KEY='  spaces  '").unwrap();
    assert_eq!(pairs[0].value, "  spaces  ");
}

#[test]
fn preserves_double_quotes() {
    let pairs = parse("KEY='he said \"hello\"'").unwrap();
    assert_eq!(pairs[0].value, "he said \"hello\"");
}

#[test]
fn no_escape_processing() {
    let pairs = parse("KEY='no\\nescape'").unwrap();
    assert_eq!(pairs[0].value, "no\\nescape");
}

#[test]
fn preserves_dollar() {
    let pairs = parse("KEY='$NOT_INTERPOLATED'").unwrap();
    assert_eq!(pairs[0].value, "$NOT_INTERPOLATED");
}

#[test]
fn empty() {
    let pairs = parse("KEY=''").unwrap();
    assert_eq!(pairs[0].value, "");
}

#[test]
fn with_equals() {
    let pairs = parse("KEY='a=b=c'").unwrap();
    assert_eq!(pairs[0].value, "a=b=c");
}

#[test]
fn with_hash() {
    let pairs = parse("KEY='hash#inside'").unwrap();
    assert_eq!(pairs[0].value, "hash#inside");
}

#[test]
fn unterminated() {
    let err = parse("KEY='unterminated").unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnterminatedQuote {
            line: 1,
            quote: '\''
        }
    ));
}

#[test]
fn with_backslash() {
    let pairs = parse(r"KEY='c:\path\to\file'").unwrap();
    assert_eq!(pairs[0].value, r"c:\path\to\file");
}
