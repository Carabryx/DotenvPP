//! Policy-as-code support for DotenvPP.
//!
//! A rule condition is interpreted as a violation predicate: when it evaluates
//! to true, the rule is reported.

use std::collections::HashMap;

use dotenvpp_expr::{evaluate, EvalOptions, Value};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors returned while parsing policy files.
#[derive(Debug, Error)]
pub enum PolicyError {
    /// TOML syntax or shape is invalid.
    #[error("policy TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    /// Policy content is invalid.
    #[error("{0}")]
    Invalid(String),
}

/// Policy severity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Hard failure.
    #[default]
    Error,
    /// Report but do not fail non-strict checks.
    Warning,
    /// Informational note.
    Info,
}

/// Policy file metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyMeta {
    /// Policy name.
    pub name: Option<String>,
    /// Policy description.
    pub description: Option<String>,
}

/// One policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Stable rule name.
    pub name: String,
    /// Human-readable description.
    pub description: Option<String>,
    /// Expression that evaluates to true when the rule is violated.
    pub condition: String,
    /// Violation severity.
    #[serde(default)]
    pub severity: Severity,
}

/// Parsed policy document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyDocument {
    /// Optional metadata.
    #[serde(default)]
    pub meta: PolicyMeta,
    /// Rules to evaluate.
    #[serde(default)]
    pub rules: Vec<PolicyRule>,
}

/// A policy violation or evaluation problem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyViolation {
    /// Rule name.
    pub rule: String,
    /// Rule severity.
    pub severity: Severity,
    /// Description or evaluation failure.
    pub message: String,
}

/// Policy evaluation report.
#[derive(Debug, Clone, Default, Serialize)]
pub struct PolicyReport {
    /// Reported policy violations.
    pub violations: Vec<PolicyViolation>,
}

impl PolicyReport {
    /// Returns true if any error-level violation exists.
    pub fn has_errors(&self) -> bool {
        self.violations.iter().any(|violation| violation.severity == Severity::Error)
    }

    /// Returns true if there are no error-level violations.
    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }
}

impl PolicyDocument {
    /// Parse a policy from TOML.
    pub fn from_toml_str(input: &str) -> Result<Self, PolicyError> {
        let policy: Self = toml::from_str(input)?;
        policy.validate_shape()?;
        Ok(policy)
    }

    /// Evaluate all rules against a variable map.
    pub fn evaluate(&self, variables: &HashMap<String, String>) -> PolicyReport {
        let mut report = PolicyReport::default();
        let options = EvalOptions {
            variables: variables.clone(),
            ..Default::default()
        };

        for rule in &self.rules {
            match evaluate(&rule.condition, &options) {
                Ok(output) => {
                    if output.value.as_bool_for_policy() {
                        report.violations.push(PolicyViolation {
                            rule: rule.name.clone(),
                            severity: rule.severity,
                            message: rule
                                .description
                                .clone()
                                .unwrap_or_else(|| "policy rule violated".to_owned()),
                        });
                    }
                }
                Err(err) => report.violations.push(PolicyViolation {
                    rule: rule.name.clone(),
                    severity: Severity::Error,
                    message: format!("policy condition failed to evaluate: {err}"),
                }),
            }
        }

        report
    }

    fn validate_shape(&self) -> Result<(), PolicyError> {
        for rule in &self.rules {
            if rule.name.trim().is_empty() {
                return Err(PolicyError::Invalid("policy rule name must not be empty".to_owned()));
            }
            if rule.condition.trim().is_empty() {
                return Err(PolicyError::Invalid(format!(
                    "policy rule `{}` condition must not be empty",
                    rule.name
                )));
            }
        }
        Ok(())
    }
}

/// A small built-in security policy library for common `.env` mistakes.
pub fn standard_security_policy() -> PolicyDocument {
    PolicyDocument {
        meta: PolicyMeta {
            name: Some("dotenvpp-standard-security".to_owned()),
            description: Some("Common DotenvPP security checks".to_owned()),
        },
        rules: vec![
            PolicyRule {
                name: "no-debug-in-prod".to_owned(),
                description: Some("Debug logging is forbidden in production".to_owned()),
                condition: r#"ENV == "production" && LOG_LEVEL == "debug""#.to_owned(),
                severity: Severity::Error,
            },
            PolicyRule {
                name: "ssl-required-for-production-database".to_owned(),
                description: Some("Production database URLs must require SSL".to_owned()),
                condition: r#"ENV == "production" && contains(DATABASE_URL, "postgres") && !contains(DATABASE_URL, "sslmode=require")"#.to_owned(),
                severity: Severity::Error,
            },
            PolicyRule {
                name: "no-localhost-outside-development".to_owned(),
                description: Some("Localhost URLs are forbidden outside development".to_owned()),
                condition: r#"ENV != "development" && (contains(DATABASE_URL, "localhost") || contains(API_URL, "localhost"))"#.to_owned(),
                severity: Severity::Warning,
            },
            PolicyRule {
                name: "no-obvious-default-passwords".to_owned(),
                description: Some("Default-looking credentials should be replaced".to_owned()),
                condition: r#"contains(lower(PASSWORD), "password") || contains(lower(DATABASE_URL), "password") || contains(lower(API_KEY), "changeme")"#.to_owned(),
                severity: Severity::Warning,
            },
        ],
    }
}

trait PolicyBool {
    fn as_bool_for_policy(&self) -> bool;
}

impl PolicyBool for Value {
    fn as_bool_for_policy(&self) -> bool {
        match self {
            Value::Bool(value) => *value,
            Value::Number(value) => *value != 0.0,
            Value::String(value) => {
                !value.is_empty() && !matches!(value.to_ascii_lowercase().as_str(), "false" | "0")
            }
            Value::Null => false,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parses_and_evaluates_policy() {
        let policy = PolicyDocument::from_toml_str(
            r#"
            [meta]
            name = "prod"

            [[rules]]
            name = "no-debug"
            description = "No debug in prod"
            condition = "ENV == 'production' && LOG_LEVEL == 'debug'"
            severity = "error"
            "#,
        )
        .unwrap();

        let report = policy.evaluate(&HashMap::from([
            ("ENV".to_owned(), "production".to_owned()),
            ("LOG_LEVEL".to_owned(), "debug".to_owned()),
        ]));

        assert!(report.has_errors());
        assert_eq!(report.violations[0].rule, "no-debug");
    }

    #[test]
    fn standard_policy_allows_secure_config() {
        let report = standard_security_policy().evaluate(&HashMap::from([
            ("ENV".to_owned(), "production".to_owned()),
            ("LOG_LEVEL".to_owned(), "info".to_owned()),
            ("DATABASE_URL".to_owned(), "postgres://db/app?sslmode=require".to_owned()),
            ("API_URL".to_owned(), "https://api.example.com".to_owned()),
            ("PASSWORD".to_owned(), "long-random-value".to_owned()),
            ("API_KEY".to_owned(), "sk-live".to_owned()),
        ]));

        assert!(report.is_ok());
        assert!(report.violations.is_empty());
    }
}
