//! Double-quoted value tests.

use crate::error::ParseError;
use crate::parser::parse;

#[test]
fn basic() {
    let pairs = parse("KEY=\"hello world\"").unwrap();
    assert_eq!(pairs[0].value, "hello world");
}

#[test]
fn preserves_spaces() {
    let pairs = parse("KEY=\"  spaces  \"").unwrap();
    assert_eq!(pairs[0].value, "  spaces  ");
}

#[test]
fn escape_newline() {
    let pairs = parse("KEY=\"line1\\nline2\"").unwrap();
    assert_eq!(pairs[0].value, "line1\nline2");
}

#[test]
fn escape_tab() {
    let pairs = parse("KEY=\"col1\\tcol2\"").unwrap();
    assert_eq!(pairs[0].value, "col1\tcol2");
}

#[test]
fn escape_carriage_return() {
    let pairs = parse("KEY=\"line\\rend\"").unwrap();
    assert_eq!(pairs[0].value, "line\rend");
}

#[test]
fn escape_backslash() {
    let pairs = parse("KEY=\"path\\\\to\"").unwrap();
    assert_eq!(pairs[0].value, "path\\to");
}

#[test]
fn escape_double_quote() {
    let pairs = parse("KEY=\"he said \\\"hello\\\"\"").unwrap();
    assert_eq!(pairs[0].value, "he said \"hello\"");
}

#[test]
fn escape_dollar() {
    let pairs = parse("KEY=\"price is \\$100\"").unwrap();
    assert_eq!(pairs[0].value, "price is $100");
}

#[test]
fn unknown_escape_kept() {
    let pairs = parse("KEY=\"unk\\xown\"").unwrap();
    assert_eq!(pairs[0].value, "unk\\xown");
}

#[test]
fn empty() {
    let pairs = parse("KEY=\"\"").unwrap();
    assert_eq!(pairs[0].value, "");
}

#[test]
fn with_single_quotes() {
    let pairs = parse("KEY=\"it's fine\"").unwrap();
    assert_eq!(pairs[0].value, "it's fine");
}

#[test]
fn unterminated() {
    let err = parse("KEY=\"unterminated").unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnterminatedQuote {
            line: 1,
            quote: '"'
        }
    ));
}

#[test]
fn with_equals() {
    let pairs = parse("KEY=\"a=b=c\"").unwrap();
    assert_eq!(pairs[0].value, "a=b=c");
}

#[test]
fn multiple_escapes() {
    let pairs = parse("KEY=\"line1\\nline2\\ttab\\\\backslash\"").unwrap();
    assert_eq!(pairs[0].value, "line1\nline2\ttab\\backslash");
}

#[test]
fn with_hash() {
    let pairs = parse("KEY=\"hash # inside\"").unwrap();
    assert_eq!(pairs[0].value, "hash # inside");
}
