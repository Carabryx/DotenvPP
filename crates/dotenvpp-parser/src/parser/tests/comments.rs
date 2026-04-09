//! Comment handling tests.

use crate::parser::parse;

#[test]
fn full_line() {
    let input = "# this is a comment\nKEY=value";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].key, "KEY");
}

#[test]
fn full_line_with_spaces() {
    let input = "  # indented comment\nKEY=value";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 1);
}

#[test]
fn comment_after_whitespace_on_value_line() {
    let pairs = parse("KEY=   # this is a comment").unwrap();
    assert_eq!(pairs[0].value, "");
}

#[test]
fn inline_unquoted() {
    let pairs = parse("KEY=value # this is a comment").unwrap();
    assert_eq!(pairs[0].value, "value");
}

#[test]
fn inline_with_tab() {
    let pairs = parse("KEY=value\t# tab comment").unwrap();
    assert_eq!(pairs[0].value, "value");
}

#[test]
fn hash_in_double_quoted_not_stripped() {
    let pairs = parse("KEY=\"value # not a comment\"").unwrap();
    assert_eq!(pairs[0].value, "value # not a comment");
}

#[test]
fn hash_in_single_quoted_not_stripped() {
    let pairs = parse("KEY='value # not a comment'").unwrap();
    assert_eq!(pairs[0].value, "value # not a comment");
}

#[test]
fn only_comments_file() {
    let input = "# comment 1\n# comment 2\n# comment 3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 0);
}

#[test]
fn hash_without_preceding_space() {
    let pairs = parse("KEY=val#ue").unwrap();
    assert_eq!(pairs[0].value, "val#ue");
}

#[test]
fn multiple_hashes() {
    let pairs = parse("KEY=value ## double hash comment").unwrap();
    assert_eq!(pairs[0].value, "value");
}

#[test]
fn hash_at_start_of_value() {
    let pairs = parse("KEY=#value").unwrap();
    assert_eq!(pairs[0].value, "#value");
}
