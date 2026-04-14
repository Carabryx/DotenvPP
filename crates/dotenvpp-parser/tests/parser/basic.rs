//! Basic KEY=VALUE parsing tests.

use crate::parser::parse;

#[test]
fn key_value() {
    let pairs = parse("KEY=value").unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].key, "KEY");
    assert_eq!(pairs[0].value, "value");
}

#[test]
fn multiple_pairs() {
    let input = "A=1\nB=2\nC=3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 3);
    assert_eq!(pairs[0].key, "A");
    assert_eq!(pairs[0].value, "1");
    assert_eq!(pairs[1].key, "B");
    assert_eq!(pairs[1].value, "2");
    assert_eq!(pairs[2].key, "C");
    assert_eq!(pairs[2].value, "3");
}

#[test]
fn empty_value() {
    let pairs = parse("KEY=").unwrap();
    assert_eq!(pairs[0].value, "");
}

#[test]
fn empty_value_double_quoted() {
    let pairs = parse("KEY=\"\"").unwrap();
    assert_eq!(pairs[0].value, "");
}

#[test]
fn empty_value_single_quoted() {
    let pairs = parse("KEY=''").unwrap();
    assert_eq!(pairs[0].value, "");
}

#[test]
fn value_with_equals() {
    let pairs = parse("URL=postgres://host:5432/db?sslmode=require").unwrap();
    assert_eq!(pairs[0].value, "postgres://host:5432/db?sslmode=require");
}

#[test]
fn whitespace_around_key() {
    let pairs = parse("  KEY  =value").unwrap();
    assert_eq!(pairs[0].key, "KEY");
    assert_eq!(pairs[0].value, "value");
}

#[test]
fn whitespace_around_value() {
    let pairs = parse("KEY=  value  ").unwrap();
    assert_eq!(pairs[0].value, "value");
}

#[test]
fn underscore_key() {
    let pairs = parse("MY_KEY=value").unwrap();
    assert_eq!(pairs[0].key, "MY_KEY");
}

#[test]
fn numeric_value() {
    let pairs = parse("PORT=8080").unwrap();
    assert_eq!(pairs[0].value, "8080");
}

#[test]
fn line_numbers() {
    let input = "A=1\n\nB=2\n# comment\nC=3";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs[0].line, 1);
    assert_eq!(pairs[1].line, 3);
    assert_eq!(pairs[2].line, 5);
}

#[test]
fn dotted_key() {
    let pairs = parse("app.name=dotenvpp").unwrap();
    assert_eq!(pairs[0].key, "app.name");
}

#[test]
fn lowercase_key() {
    let pairs = parse("key=value").unwrap();
    assert_eq!(pairs[0].key, "key");
}

#[test]
fn mixed_case_key() {
    let pairs = parse("myKey=value").unwrap();
    assert_eq!(pairs[0].key, "myKey");
}

#[test]
fn trailing_newline() {
    let pairs = parse("KEY=value\n").unwrap();
    assert_eq!(pairs.len(), 1);
    assert_eq!(pairs[0].value, "value");
}
