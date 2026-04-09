//! Error types for the DotenvPP parser.

#[cfg(not(feature = "std"))]
use alloc::string::String;
use core::fmt;
#[cfg(feature = "std")]
use std::string::String;

/// Errors that can occur while parsing `.env` content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// A line is missing the `=` separator between key and value.
    MissingSeparator {
        /// The 1-based line number where the error occurred.
        line: usize,
        /// The raw content of the offending line.
        content: String,
    },

    /// A key is empty (nothing before `=`).
    EmptyKey {
        /// The 1-based line number where the error occurred.
        line: usize,
    },

    /// A key contains invalid characters.
    InvalidKey {
        /// The 1-based line number where the error occurred.
        line: usize,
        /// The invalid key.
        key: String,
    },

    /// An unterminated quoted value (missing closing quote).
    UnterminatedQuote {
        /// The 1-based line number where the quote started.
        line: usize,
        /// The quote character (' or ").
        quote: char,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSeparator {
                line,
                ..
            } => {
                write!(f, "line {line}: missing `=` separator")
            }
            Self::EmptyKey {
                line,
            } => {
                write!(f, "line {line}: key is empty")
            }
            Self::InvalidKey {
                line,
                key,
            } => {
                write!(
                    f,
                    "line {line}: invalid key `{key}` — keys must be \
                     ASCII alphanumeric, underscores, or dots"
                )
            }
            Self::UnterminatedQuote {
                line,
                quote,
            } => {
                write!(f, "line {line}: unterminated {quote}-quoted value")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ParseError {}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::ParseError;

    #[test]
    fn missing_separator_display_redacts_content() {
        let err = ParseError::MissingSeparator {
            line: 3,
            content: "API_KEY abc123".into(),
        };
        let msg = format!("{err}");

        assert!(msg.contains("line 3"));
        assert!(msg.contains("missing `=` separator"));
        assert!(!msg.contains("API_KEY"));
        assert!(!msg.contains("abc123"));
    }
}
