//! Benchmarks comparing dotenvpp-parser performance.

use std::hint::black_box;
use std::io::Cursor;

use criterion::{criterion_group, criterion_main, Criterion};
use dotenvpp_parser::parse;

fn generate_env(count: usize) -> String {
    let mut s = String::new();
    for i in 0..count {
        s.push_str(&format!("VAR_{i}=\"value_{i} with some content\"\n"));
    }
    s
}

fn bench_pair(c: &mut Criterion, label: &str, input: &str) {
    c.bench_function(&format!("dotenvpp/{label}"), |b| {
        b.iter(|| {
            let pairs = parse(black_box(input)).unwrap();
            black_box(pairs.len())
        })
    });

    c.bench_function(&format!("dotenvy/{label}"), |b| {
        b.iter(|| {
            let count = dotenvy::from_read_iter(Cursor::new(black_box(input.as_bytes()))).count();
            black_box(count)
        })
    });
}

fn bench_parse_small(c: &mut Criterion) {
    let input = generate_env(5);
    bench_pair(c, "parse_small_5_vars", &input);
}

fn bench_parse_medium(c: &mut Criterion) {
    let input = generate_env(50);
    bench_pair(c, "parse_medium_50_vars", &input);
}

fn bench_parse_large(c: &mut Criterion) {
    let input = generate_env(500);
    bench_pair(c, "parse_large_500_vars", &input);
}

fn bench_parse_mixed_styles(c: &mut Criterion) {
    let input = (0..100)
        .map(|i| match i % 4 {
            0 => format!("UNQUOTED_{i}=value_{i}\n"),
            1 => format!("DOUBLE_{i}=\"value {i}\"\n"),
            2 => format!("SINGLE_{i}='value {i}'\n"),
            3 => format!("export EXPORT_{i}=value_{i}\n"),
            _ => unreachable!(),
        })
        .collect::<String>();

    bench_pair(c, "parse_mixed_100_vars", &input);
}

fn bench_parse_multiline(c: &mut Criterion) {
    let input = (0..50)
        .map(|i| format!("VAR_{i}=\"line1\nline2\nline3\"\n"))
        .collect::<String>();

    bench_pair(c, "parse_multiline_50_vars", &input);
}

criterion_group!(
    benches,
    bench_parse_small,
    bench_parse_medium,
    bench_parse_large,
    bench_parse_mixed_styles,
    bench_parse_multiline,
);
criterion_main!(benches);
