use super::super::{is_valid_key, parse};

#[test]
fn empty_key_is_invalid() {
    assert!(!is_valid_key(""));
}

#[test]
fn single_quoted_tail_is_appended_as_unquoted() {
    let pairs = parse("KEY='value' tail").unwrap();
    assert_eq!(pairs[0].value, "value tail");
}

#[test]
fn double_quoted_tail_is_appended_as_unquoted() {
    let pairs = parse("KEY=\"value\" suffix").unwrap();
    assert_eq!(pairs[0].value, "value suffix");
}

#[test]
fn double_quoted_backslash_at_line_end_is_preserved() {
    let pairs = parse("KEY=\"line1\\\nline2\"").unwrap();
    assert_eq!(pairs[0].value, "line1\\\nline2");
}

#[test]
fn unquoted_escape_variants_are_decoded() {
    let pairs = parse("KEY=quote\\\" single\\' hash\\# space\\ done").unwrap();
    assert_eq!(pairs[0].value, "quote\" single' hash# space done");
}

#[test]
fn trailing_backslash_in_unquoted_value_is_preserved() {
    let pairs = parse("KEY=value\\").unwrap();
    assert_eq!(pairs[0].value, "value\\");
}

#[test]
fn double_quoted_extra_escape_variants_are_decoded() {
    let pairs = parse("KEY=\"\\'\\ \\#\"").unwrap();
    assert_eq!(pairs[0].value, "' #");
}
