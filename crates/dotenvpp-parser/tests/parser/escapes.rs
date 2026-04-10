//! Escape sequence tests (double-quoted values).

use crate::parser::parse;

#[test]
fn all_supported() {
    let pairs = parse("KEY=\"\\n\\t\\r\\\\\\\"\\$\"").unwrap();
    assert_eq!(pairs[0].value, "\n\t\r\\\"$");
}

#[test]
fn consecutive_backslashes() {
    let pairs = parse("KEY=\"\\\\\\\\\"").unwrap();
    assert_eq!(pairs[0].value, "\\\\");
}

#[test]
fn at_end_of_quoted() {
    let pairs = parse("KEY=\"end\\n\"").unwrap();
    assert_eq!(pairs[0].value, "end\n");
}

#[test]
fn newline_only() {
    let pairs = parse("KEY=\"\\n\"").unwrap();
    assert_eq!(pairs[0].value, "\n");
}

#[test]
fn tab_only() {
    let pairs = parse("KEY=\"\\t\"").unwrap();
    assert_eq!(pairs[0].value, "\t");
}

#[test]
fn mixed_with_text() {
    let pairs = parse("KEY=\"hello\\nworld\\ttab\"").unwrap();
    assert_eq!(pairs[0].value, "hello\nworld\ttab");
}

#[test]
fn unknown_sequence_a() {
    let pairs = parse("KEY=\"\\a\"").unwrap();
    assert_eq!(pairs[0].value, "\\a");
}

#[test]
fn unknown_sequence_zero() {
    let pairs = parse("KEY=\"\\0\"").unwrap();
    assert_eq!(pairs[0].value, "\\0");
}

#[test]
fn escapes_in_unquoted() {
    let pairs = parse("KEY=hello\\nworld\\\\backslash\\$cash\\ space").unwrap();
    assert_eq!(pairs[0].value, "hello\nworld\\backslash$cash space");
}

#[test]
fn no_escapes_in_single_quoted() {
    let pairs = parse("KEY='hello\\nworld'").unwrap();
    assert_eq!(pairs[0].value, "hello\\nworld");
}

#[test]
fn value_starts_with_escape() {
    let pairs = parse("KEY=\"\\nhello\"").unwrap();
    assert_eq!(pairs[0].value, "\nhello");
}

#[test]
fn value_ends_with_escape() {
    let pairs = parse("KEY=\"hello\\n\"").unwrap();
    assert_eq!(pairs[0].value, "hello\n");
}
