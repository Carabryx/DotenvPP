# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
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

[Unreleased]: https://github.com/Carabryx/DotenvPP/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/Carabryx/DotenvPP/releases/tag/v0.0.2
