# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.3] - 2026-04-10

### Added
- Variable interpolation: `${VAR}`, `${VAR:-default}`, `${VAR:?error}`,
  `${VAR:+alternative}`, and `$$` literal dollar sign escaping.
- Environment layering: `.env` < `.env.{ENV}` < `.env.local` < `.env.{ENV}.local`.
- `--env` / `-e` CLI flag for environment selection in `check` and `run` commands.
- Circular reference detection with source-file-aware error messages.
- `load_with_env()`, `load_with_env_override()`, `from_layered_env()`, and
  `load_override()` facade APIs.
- Cross-layer interpolation (references resolve across layered files).

### Changed
- `load()` now performs layered loading (previously loaded a single `.env`).
- CLI `check` validates the full layered stack when `--env` is used.
- Clarified README and usage-example docs so Phase 1+ items stay marked as roadmap work, while Phase 0 docs focus on the shipped parser, facade, CLI, and manual override behavior.

## [0.0.2] - 2026-03-27

### Added
- Phase 0 parser foundation for common `.env` syntax, including quoted and unquoted values, comments, blank lines, and `export` prefixes.
- `dotenvpp` facade helpers for reading from strings, files, and the process environment.
- CLI `check` and `run` commands.
- Criterion benchmarks over representative parser workloads.
- `docs/INTRODUCTION.md` and the refreshed release README.

### Fixed
- BOM-prefixed files are parsed correctly.
- Keys starting with `.` are rejected.
- Value lines with leading whitespace before `#` are treated as comments.
- Single-quoted multiline values are supported.
- Common unquoted escapes are decoded while preserving Windows path literals.

[Unreleased]: https://github.com/Carabryx/DotenvPP/compare/v0.0.3...HEAD
[0.0.3]: https://github.com/Carabryx/DotenvPP/releases/tag/v0.0.3
[0.0.2]: https://github.com/Carabryx/DotenvPP/releases/tag/v0.0.2
