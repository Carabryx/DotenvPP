# DotenvPP Usage Examples

This directory contains examples that use the `dotenvpp` crate, demonstrating real-world usage as an external dependency with **real `.env` files**.

## What's Included

| File | Purpose |
|---|---|
| `.env` | Default development configuration |
| `.env.production` | Alternate configuration file for manual override demos |
| `src/basic_loading.rs` | Load `.env` and print all variables |
| `src/custom_path.rs` | Load from `.env.production` with override semantics |
| `src/iterator_usage.rs` | Use iterators to filter and inspect variables |

## Running

```powershell
cd usage-examples
cargo run
```

## Notes

- The `.env` files are **tracked in git** — they contain demo data, not real secrets
- This crate is a workspace member, but marked `publish = false` so it is not published to crates.io
- Automatic environment layering is a Phase 1 roadmap item; these examples use explicit file selection in Phase 0
- Depends on `dotenvpp` via `path = ".."`
