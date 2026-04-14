# DotenvPP Usage Examples

This directory contains examples that use the `dotenvpp` crate, demonstrating real-world usage as an external dependency with **real `.env` files**.

## What's Included

### Environment Files

| File | Purpose |
|---|---|
| `.env` | Default development config with interpolation, defaults, multiline, export |
| `.env.production` | Production overrides |
| `.env.local` | Local developer overrides (normally gitignored) |
| `.env.production.local` | Production-local secrets (normally gitignored) |

### Real-World Scenarios (`src/scenarios/`)

| Scenario | Run With | What It Shows |
|---|---|---|
| `01_basic_loading` | `cargo run --bin basic-loading` | `load()` + `var()` — the most common usage |
| `02_from_read` | `cargo run --bin from-read` | Parse from a byte string, no file needed |
| `03_interpolation` | `cargo run --bin interpolation` | `${VAR}`, `${VAR:-default}`, `${VAR:+alt}`, `$$`, chaining |
| `04_layered_env` | `cargo run --bin layered-env` | Default layered stack (`.env` + `.env.local`) |
| `05_production` | `cargo run --bin production` | Full production stack (all 4 layers) |
| `06_preview_without_loading` | `cargo run --bin preview-without-loading` | `from_path_iter()` — filter without setting env |
| `07_error_handling` | `cargo run --bin error-handling` | Required vars, circular refs, parse errors, missing files |

### Legacy Examples (`src/`)

| File | Purpose |
|---|---|
| `src/main.rs` | Runs the original basic, custom path, and iterator examples |
| `src/basic_loading.rs` | Load `.env` and print all variables |
| `src/custom_path.rs` | Load from `.env.production` with override semantics |
| `src/iterator_usage.rs` | Use iterators to filter and inspect variables |

## Running

```powershell
cd usage-examples

# Run a specific scenario
cargo run --bin basic-loading
cargo run --bin production
cargo run --bin interpolation

# Run all legacy examples
cargo run
```

## Notes

- The `.env` files are **tracked in git** — they contain demo data, not real secrets
- This crate is a workspace member, but marked `publish = false` so it is not published to crates.io
- Automatic environment layering is available via `dotenvpp::load()` / `load_with_env()` / `from_layered_env()`
- Depends on `dotenvpp` via `path = ".."`
