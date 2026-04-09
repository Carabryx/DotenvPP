//! Comprehensive test suite for the `.env` parser.
//!
//! Organized by feature area as separate submodules.
//! Target: 100+ test cases covering all parsing behaviors.

#![allow(clippy::unwrap_used)]

mod basic;
mod blank_lines;
mod comments;
mod double_quotes;
mod edge_cases;
mod escapes;
mod export;
mod internals;
mod mixed;
mod multiline;
mod single_quotes;
