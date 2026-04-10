//! Multiline value tests.

use crate::error::ParseError;
use crate::parser::parse;

#[test]
fn double_quoted() {
    let input = "KEY=\"line1\nline2\nline3\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "line1\nline2\nline3");
}

#[test]
fn double_quoted_with_escapes() {
    let input = "KEY=\"line1\\n\nactual_line2\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "line1\n\nactual_line2");
}

#[test]
fn preserves_indentation() {
    let input = "KEY=\"line1\n  indented\n    more\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "line1\n  indented\n    more");
}

#[test]
fn empty_lines() {
    let input = "KEY=\"line1\n\n\nline4\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "line1\n\n\nline4");
}

#[test]
fn with_following_pair() {
    let input = "MULTI=\"line1\nline2\"\nSINGLE=value";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0].value, "line1\nline2");
    assert_eq!(pairs[1].key, "SINGLE");
    assert_eq!(pairs[1].value, "value");
}

#[test]
fn unterminated() {
    let input = "KEY=\"line1\nline2\nline3";
    let err = parse(input).unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnterminatedQuote {
            line: 1,
            quote: '"'
        }
    ));
}

#[test]
fn with_hash_lines() {
    let input = "KEY=\"line1\n# not a comment\nline3\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "line1\n# not a comment\nline3");
}

#[test]
fn with_equals() {
    let input = "KEY=\"a=1\nb=2\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "a=1\nb=2");
}

#[test]
fn escape_at_line_boundary() {
    let input = "KEY=\"end\\n\nstart\"";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "end\n\nstart");
}

#[test]
fn single_quote_multiline() {
    let input = "KEY='line1\nline2\nline3'";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].value, "line1\nline2\nline3");
}

#[test]
fn single_quote_unterminated() {
    let err = parse("KEY='unterminated\nsecond line").unwrap_err();
    assert!(matches!(
        err,
        ParseError::UnterminatedQuote {
            line: 1,
            quote: '\''
        }
    ));
}
