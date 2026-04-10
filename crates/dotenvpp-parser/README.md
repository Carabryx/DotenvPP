# dotenvpp-parser

Core `.env` parser used by DotenvPP.

It focuses on syntax parsing only:

- `KEY=VALUE` parsing
- Comments, blank lines, and `export`
- Single-quoted, double-quoted, and unquoted values
- Multiline quoted values
- `no_std` support with the default `std` feature enabled

If you want interpolation, layered loading, or process environment helpers, use the top-level [`dotenvpp`](../../README.md) crate instead.
