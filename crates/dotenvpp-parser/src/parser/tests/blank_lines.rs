//! Blank line handling tests.

use crate::parser::parse;

#[test]
fn between_pairs() {
    let input = "A=1\n\n\nB=2";
    let pairs = parse(input).unwrap();
    assert_eq!(
        pairs.iter().map(|p| (p.key.as_str(), p.value.as_str())).collect::<Vec<_>>(),
        vec![("A", "1"), ("B", "2")]
    );
}

#[test]
fn at_start() {
    let input = "\n\nKEY=value";
    let pairs = parse(input).unwrap();
    assert_eq!(
        pairs.iter().map(|p| (p.key.as_str(), p.value.as_str())).collect::<Vec<_>>(),
        vec![("KEY", "value")]
    );
}

#[test]
fn at_end() {
    let input = "KEY=value\n\n\n";
    let pairs = parse(input).unwrap();
    assert_eq!(
        pairs.iter().map(|p| (p.key.as_str(), p.value.as_str())).collect::<Vec<_>>(),
        vec![("KEY", "value")]
    );
}

#[test]
fn with_whitespace() {
    let input = "A=1\n   \n  \nB=2";
    let pairs = parse(input).unwrap();
    assert_eq!(
        pairs.iter().map(|p| (p.key.as_str(), p.value.as_str())).collect::<Vec<_>>(),
        vec![("A", "1"), ("B", "2")]
    );
}

#[test]
fn tabs_only() {
    let input = "A=1\n\t\t\nB=2";
    let pairs = parse(input).unwrap();
    assert_eq!(
        pairs.iter().map(|p| (p.key.as_str(), p.value.as_str())).collect::<Vec<_>>(),
        vec![("A", "1"), ("B", "2")]
    );
}
