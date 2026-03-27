//! Core `.env` parser for DotenvPP.
//!
//! Parses `.env` content into a list of key-value pairs, handling:
//! - `KEY=VALUE` basic assignment
//! - `# comments` (full-line and inline for unquoted values)
//! - Blank line skipping
//! - Single-quoted values, including multiline content
//! - Double-quoted values (escape sequences + multiline support)
//! - Unquoted values (trim trailing whitespace, strip inline comments,
//!   and decode common backslash escapes)
//! - `export KEY=VALUE` prefix stripping
//! - Escape sequences in double quotes: `\\`, `\"`, `\n`, `\t`, `\r`

use alloc::string::String;
use alloc::vec::Vec;

use crate::error::ParseError;

/// A parsed key-value pair from a `.env` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvPair {
    /// The environment variable key.
    pub key: String,
    /// The environment variable value.
    pub value: String,
    /// The 1-based line number where this pair was found.
    pub line: usize,
}

/// Parse `.env` file content into a list of [`EnvPair`]s.
///
/// # Errors
///
/// Returns [`ParseError`] if the input contains syntax errors such as
/// missing `=` separators, empty keys, invalid key names, or unterminated
/// quoted values.
///
/// # Examples
///
/// ```
/// use dotenvpp_parser::parse;
///
/// let input = "KEY=value\nNAME=\"hello world\"\n";
/// let pairs = parse(input).unwrap();
/// assert_eq!(pairs.len(), 2);
/// assert_eq!(pairs[0].key, "KEY");
/// assert_eq!(pairs[0].value, "value");
/// assert_eq!(pairs[1].key, "NAME");
/// assert_eq!(pairs[1].value, "hello world");
/// ```
pub fn parse(input: &str) -> Result<Vec<EnvPair>, ParseError> {
    let mut pairs = Vec::new();
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut lines = input.lines().enumerate().peekable();

    while let Some((line_idx, raw_line)) = lines.next() {
        let line_num = line_idx + 1;
        let trimmed = raw_line.trim();

        // Skip blank lines and comments.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip optional `export` prefix.
        let effective = strip_export_prefix(trimmed);

        // Find the `=` separator.
        let eq_pos = match effective.find('=') {
            Some(pos) => pos,
            None => {
                return Err(ParseError::MissingSeparator {
                    line: line_num,
                    content: String::from(trimmed),
                });
            }
        };

        let raw_key = &effective[..eq_pos];
        let key = raw_key.trim();

        if key.is_empty() {
            return Err(ParseError::EmptyKey {
                line: line_num,
            });
        }

        if !is_valid_key(key) {
            return Err(ParseError::InvalidKey {
                line: line_num,
                key: String::from(key),
            });
        }

        let after_eq = &effective[eq_pos + 1..];
        let value = parse_value(after_eq, line_num, &mut lines)?;

        pairs.push(EnvPair {
            key: String::from(key),
            value,
            line: line_num,
        });
    }

    Ok(pairs)
}

/// Strip the `export ` prefix from a line, if present.
fn strip_export_prefix(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix("export ") {
        rest.trim_start()
    } else if let Some(rest) = line.strip_prefix("export\t") {
        rest.trim_start()
    } else {
        line
    }
}

/// Check if a key is valid: must be ASCII alphanumeric, underscores,
/// or dots, and must not start with a digit.
fn is_valid_key(key: &str) -> bool {
    if key.is_empty() {
        return false;
    }

    let first = key.as_bytes()[0];
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }

    key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.')
}

/// Parse the value portion of a `KEY=VALUE` line.
///
/// Handles single-quoted, double-quoted, and unquoted values,
/// including multiline double-quoted values across multiple lines.
fn parse_value<'a, I>(
    value_start: &str,
    line_num: usize,
    lines: &mut core::iter::Peekable<I>,
) -> Result<String, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let trimmed_start = value_start.trim_start_matches(|c: char| c == ' ' || c == '\t');
    if trimmed_start.is_empty() {
        return Ok(String::new());
    }

    if trimmed_start.starts_with('#') && trimmed_start.len() != value_start.len() {
        return Ok(String::new());
    }

    let first_char = trimmed_start.as_bytes()[0];

    match first_char {
        b'\'' => parse_single_quoted(trimmed_start, line_num, lines),
        b'"' => parse_double_quoted(trimmed_start, line_num, lines),
        _ => Ok(parse_unquoted(trimmed_start)),
    }
}

/// Parse a single-quoted value. No escape processing.
/// Content is literal between the opening and closing `'`, including
/// multiline content.
fn parse_single_quoted<'a, I>(
    value_start: &str,
    line_num: usize,
    lines: &mut core::iter::Peekable<I>,
) -> Result<String, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut result = String::new();
    let mut remaining = &value_start[1..];

    loop {
        match remaining.find('\'') {
            Some(close_pos) => {
                result.push_str(&remaining[..close_pos]);
                let tail = &remaining[close_pos + 1..];
                if !tail.is_empty() {
                    result.push_str(&parse_unquoted(tail));
                }
                return Ok(result);
            }
            None => {
                result.push_str(remaining);

                if let Some((_, next_line)) = lines.next() {
                    result.push('\n');
                    remaining = next_line;
                } else {
                    return Err(ParseError::UnterminatedQuote {
                        line: line_num,
                        quote: '\'',
                    });
                }
            }
        }
    }
}

/// Parse a double-quoted value with escape sequence processing.
/// Supports multiline values that span across multiple input lines.
fn parse_double_quoted<'a, I>(
    value_start: &str,
    line_num: usize,
    lines: &mut core::iter::Peekable<I>,
) -> Result<String, ParseError>
where
    I: Iterator<Item = (usize, &'a str)>,
{
    let mut result = String::new();
    // Skip the opening quote.
    let mut remaining = &value_start[1..];

    loop {
        let mut chars = remaining.char_indices();

        while let Some((idx, ch)) = chars.next() {
            match ch {
                '"' => {
                    // Found the closing quote.
                    let tail = &remaining[idx + ch.len_utf8()..];
                    if !tail.is_empty() {
                        result.push_str(&parse_unquoted(tail));
                    }
                    return Ok(result);
                }
                '\\' => {
                    // Process escape sequence.
                    if let Some((_, escaped)) = chars.next() {
                        push_escaped_char(&mut result, escaped);
                    } else {
                        // Backslash at end of line inside double
                        // quotes — the value continues on the next
                        // line (line continuation).
                        result.push('\\');
                        let _ = idx; // suppress unused warning
                    }
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        // We reached the end of this line without finding a closing
        // quote. This is a multiline value — continue to the next line.
        if let Some((_, next_line)) = lines.next() {
            result.push('\n');
            remaining = next_line;
        } else {
            // No more lines — unterminated quote.
            return Err(ParseError::UnterminatedQuote {
                line: line_num,
                quote: '"',
            });
        }
    }
}

/// Parse an unquoted value.
/// Strips inline comments (` #` or `\t#`), trims trailing whitespace,
/// and decodes common backslash escapes.
fn parse_unquoted(value_start: &str) -> String {
    // Find inline comment — must be preceded by whitespace.
    let value = if let Some(pos) = find_inline_comment(value_start) {
        &value_start[..pos]
    } else {
        value_start
    };

    decode_escapes(value.trim_end())
}

/// Find the position of an inline comment (`#` preceded by whitespace).
fn find_inline_comment(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();

    for i in 1..bytes.len() {
        if bytes[i] == b'#' && (bytes[i - 1] == b' ' || bytes[i - 1] == b'\t') {
            return Some(i - 1);
        }
    }

    None
}

/// Decode escape sequences used in unquoted and double-quoted values.
fn decode_escapes(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(escaped) = chars.next() {
                match escaped {
                    'n' => result.push('\n'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    '\'' => result.push('\''),
                    '$' => result.push('$'),
                    ' ' => result.push(' '),
                    '#' => result.push('#'),
                    _ => {
                        result.push('\\');
                        result.push(escaped);
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Push a supported escape sequence into `result`.
///
/// Unknown escapes are preserved verbatim so the parser stays
/// permissive for common dotenv variants.
fn push_escaped_char(result: &mut String, escaped: char) {
    match escaped {
        'n' => result.push('\n'),
        't' => result.push('\t'),
        'r' => result.push('\r'),
        '\\' => result.push('\\'),
        '"' => result.push('"'),
        '\'' => result.push('\''),
        '$' => result.push('$'),
        ' ' => result.push(' '),
        '#' => result.push('#'),
        _ => {
            result.push('\\');
            result.push(escaped);
        }
    }
}

#[cfg(test)]
mod tests;
