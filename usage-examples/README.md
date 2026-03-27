# DotenvPP Usage Examples

This directory contains examples that use the `dotenvpp` crate, demonstrating real-world usage as an external dependency with **real `.env` files**.

## What's Included

| File | Purpose |
|---|---|
| `.env` | Default development configuration |
| `.env.production` | Production overrides — shows environment layering |
| `src/basic_loading.rs` | Load `.env` and print all variables |
| `src/custom_path.rs` | Load from `.env.production` with overrides |
| `src/iterator_usage.rs` | Use iterators to filter and inspect variables |

## Running

```powershell
cd usage-examples
cargo run
```

## Notes

- The `.env` files are **tracked in git** — they contain demo data, not real secrets
- This directory is excluded from workspace and crates.io publishing
- Depends on `dotenvpp` via `path = ".."`
