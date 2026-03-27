//! Blank line handling tests.

use crate::parser::parse;

#[test]
fn between_pairs() {
    let input = "A=1\n\n\nB=2";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 2);
}

#[test]
fn at_start() {
    let input = "\n\nKEY=value";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 1);
}

#[test]
fn at_end() {
    let input = "KEY=value\n\n\n";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 1);
}

#[test]
fn with_whitespace() {
    let input = "A=1\n   \n  \nB=2";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 2);
}

#[test]
fn tabs_only() {
    let input = "A=1\n\t\t\nB=2";
    let pairs = parse(input).unwrap();
    assert_eq!(pairs.len(), 2);
}
