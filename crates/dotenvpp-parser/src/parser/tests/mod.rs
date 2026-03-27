//! Comprehensive test suite for the `.env` parser.
//!
//! Organized by feature area as separate submodules.
//! Target: 100+ test cases covering all parsing behaviors.

mod basic;
mod blank_lines;
mod comments;
mod double_quotes;
mod edge_cases;
mod escapes;
mod export;
mod mixed;
mod multiline;
mod single_quotes;
