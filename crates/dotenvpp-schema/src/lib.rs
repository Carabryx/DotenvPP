//! Typed schema parsing and validation for DotenvPP.
//!
//! The schema format is TOML:
//!
//! ```toml
//! [vars.PORT]
//! type = "u16"
//! default = 8080
//! range = [1024, 65535]
//! ```

use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::path::PathBuf;

use dotenvpp_parser::EnvPair;
use regex_lite::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use thiserror::Error;

/// Errors produced while parsing schema definitions.
#[derive(Debug, Error)]
pub enum SchemaError {
    /// TOML syntax or shape is invalid.
    #[error("schema TOML error: {0}")]
    Toml(#[from] toml::de::Error),
    /// A field contains an invalid value.
    #[error("{0}")]
    Invalid(String),
}

/// Severity used by validation diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    /// Validation failed.
    Error,
    /// Validation succeeded but should be reviewed.
    Warning,
    /// Informational note.
    Info,
}

/// A structured schema validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Severity for this diagnostic.
    pub severity: DiagnosticSeverity,
    /// Variable key when the diagnostic is tied to a variable.
    pub key: Option<String>,
    /// Human-readable message.
    pub message: String,
}

impl Diagnostic {
    fn error(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Error,
            key: Some(key.into()),
            message: message.into(),
        }
    }

    fn warning(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: DiagnosticSeverity::Warning,
            key: Some(key.into()),
            message: message.into(),
        }
    }
}

/// Supported DotenvPP schema types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigType {
    /// Arbitrary string.
    String,
    /// Boolean.
    Bool,
    /// Signed 32-bit integer.
    I32,
    /// Signed 64-bit integer.
    I64,
    /// Unsigned 16-bit integer.
    U16,
    /// Unsigned 32-bit integer.
    U32,
    /// Unsigned 64-bit integer.
    U64,
    /// 64-bit floating point number.
    F64,
    /// Absolute or relative URL.
    Url,
    /// Email address.
    Email,
    /// IPv4 or IPv6 address.
    Ip,
    /// TCP/UDP port.
    Port,
    /// Duration such as `30s`, `5m`, or `1h`.
    Duration,
    /// RFC3339 timestamp.
    Datetime,
    /// String validated by a regular expression.
    Regex,
    /// Filesystem path.
    Path,
    /// One of an allowed list of strings.
    Enum,
    /// Array of strings.
    StringArray,
    /// Array of signed 32-bit integers.
    I32Array,
}

impl ConfigType {
    /// Returns the canonical schema type string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::String => "string",
            Self::Bool => "bool",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F64 => "f64",
            Self::Url => "url",
            Self::Email => "email",
            Self::Ip => "ip",
            Self::Port => "port",
            Self::Duration => "duration",
            Self::Datetime => "datetime",
            Self::Regex => "regex",
            Self::Path => "path",
            Self::Enum => "enum",
            Self::StringArray => "string[]",
            Self::I32Array => "i32[]",
        }
    }
}

impl<'de> Deserialize<'de> for ConfigType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "string" => Ok(Self::String),
            "bool" => Ok(Self::Bool),
            "i32" => Ok(Self::I32),
            "i64" => Ok(Self::I64),
            "u16" => Ok(Self::U16),
            "u32" => Ok(Self::U32),
            "u64" => Ok(Self::U64),
            "f64" => Ok(Self::F64),
            "url" => Ok(Self::Url),
            "email" => Ok(Self::Email),
            "ip" => Ok(Self::Ip),
            "port" => Ok(Self::Port),
            "duration" => Ok(Self::Duration),
            "datetime" => Ok(Self::Datetime),
            "regex" => Ok(Self::Regex),
            "path" => Ok(Self::Path),
            "enum" => Ok(Self::Enum),
            "string[]" => Ok(Self::StringArray),
            "i32[]" => Ok(Self::I32Array),
            other => Err(serde::de::Error::custom(format!("unsupported schema type `{other}`"))),
        }
    }
}

/// Schema metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaMeta {
    /// Human-readable schema name.
    pub name: Option<String>,
    /// Schema version.
    pub version: Option<String>,
    /// Schema description.
    pub description: Option<String>,
}

/// One variable definition from `.env.schema`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VarSpec {
    /// Variable type.
    #[serde(rename = "type")]
    pub ty: ConfigType,
    /// Whether the key must be present when no default is defined.
    #[serde(default)]
    pub required: bool,
    /// Default value used when the variable is absent.
    pub default: Option<toml::Value>,
    /// Marks the value as sensitive for redaction and example generation.
    #[serde(default)]
    pub secret: bool,
    /// Documentation string.
    pub description: Option<String>,
    /// Allowed enum values.
    #[serde(default)]
    pub values: Vec<String>,
    /// Separator used for array values.
    pub separator: Option<String>,
    /// Numeric inclusive range `[min, max]`.
    pub range: Option<Vec<toml::Value>>,
    /// Minimum string length.
    pub min_length: Option<usize>,
    /// Maximum string length.
    pub max_length: Option<usize>,
    /// Regex pattern for `regex` type or string pattern validation.
    pub pattern: Option<String>,
    /// Allowed URL schemes.
    #[serde(default)]
    pub protocols: Vec<String>,
    /// Minimum duration, when `type = "duration"`.
    pub min: Option<toml::Value>,
    /// Maximum duration, when `type = "duration"`.
    pub max: Option<toml::Value>,
}

/// Parsed schema document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaDocument {
    /// Optional schema metadata.
    #[serde(default)]
    pub meta: SchemaMeta,
    /// Variable definitions, keyed by env var name.
    #[serde(default)]
    pub vars: BTreeMap<String, VarSpec>,
}

/// Implemented by `#[derive(dotenvpp::Schema)]`.
pub trait ConfigSchema {
    /// Return this type's DotenvPP schema document.
    fn schema() -> SchemaDocument;
}

/// Typed value returned by validation.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum TypedValue {
    /// String-like value.
    String(String),
    /// Boolean.
    Bool(bool),
    /// Signed integer.
    I64(i64),
    /// Unsigned integer.
    U64(u64),
    /// Floating-point value.
    F64(f64),
    /// Array value.
    Array(Vec<TypedValue>),
    /// Duration in whole seconds plus the original source string.
    Duration {
        /// Parsed whole seconds.
        seconds: u64,
        /// Original source value.
        original: String,
    },
}

/// Whether a validated entry came from the file or schema defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueSource {
    /// Present in a parsed env file.
    File,
    /// Supplied by the schema default.
    Default,
}

/// One validated config entry.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TypedEntry {
    /// Variable name.
    pub key: String,
    /// Parsed typed value.
    pub value: TypedValue,
    /// Whether the schema marked this variable as secret.
    pub secret: bool,
    /// Where this value came from.
    pub source: ValueSource,
}

/// Result of validating a config map against a schema.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ValidationReport {
    /// Parsed entries.
    pub entries: BTreeMap<String, TypedEntry>,
    /// Errors, warnings, and informational notes.
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    /// Returns true when at least one error diagnostic is present.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    /// Returns true if validation had no error diagnostics.
    pub fn is_ok(&self) -> bool {
        !self.has_errors()
    }

    /// Returns error diagnostics only.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    /// Returns warning diagnostics only.
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
    }
}

impl SchemaDocument {
    /// Parse a schema document from TOML.
    pub fn from_toml_str(input: &str) -> Result<Self, SchemaError> {
        let schema: Self = toml::from_str(input)?;
        schema.validate_shape()?;
        Ok(schema)
    }

    /// Parse a schema and validate `.env` pairs against it.
    pub fn validate_pairs(&self, pairs: &[EnvPair]) -> ValidationReport {
        let mut report = ValidationReport::default();
        let env_map = pairs
            .iter()
            .map(|pair| (pair.key.as_str(), pair.value.as_str()))
            .collect::<HashMap<_, _>>();

        for (key, spec) in &self.vars {
            let (raw, source) = match env_map.get(key.as_str()) {
                Some(value) => ((*value).to_owned(), ValueSource::File),
                None => match &spec.default {
                    Some(default) => (toml_value_to_env_string(default), ValueSource::Default),
                    None if spec.required => {
                        report
                            .diagnostics
                            .push(Diagnostic::error(key, "required variable is missing"));
                        continue;
                    }
                    None => continue,
                },
            };

            match validate_value(key, spec, &raw) {
                Ok(value) => {
                    report.entries.insert(
                        key.clone(),
                        TypedEntry {
                            key: key.clone(),
                            value,
                            secret: spec.secret,
                            source,
                        },
                    );
                }
                Err(message) => report.diagnostics.push(Diagnostic::error(key, message)),
            }
        }

        for pair in pairs {
            if !self.vars.contains_key(&pair.key) {
                report
                    .diagnostics
                    .push(Diagnostic::warning(&pair.key, "variable is not declared in schema"));
            }
        }

        report
    }

    /// Generate a safe `.env.example` body from this schema.
    pub fn generate_example(&self) -> String {
        let mut out = String::new();

        for (key, spec) in &self.vars {
            if let Some(description) = &spec.description {
                for line in description.lines() {
                    out.push_str("# ");
                    out.push_str(line);
                    out.push('\n');
                }
            }

            let value = if spec.secret {
                String::new()
            } else if let Some(default) = &spec.default {
                toml_value_to_env_string(default)
            } else if spec.ty == ConfigType::Enum {
                spec.values.first().cloned().unwrap_or_default()
            } else {
                placeholder_for_type(&spec.ty).to_owned()
            };

            out.push_str(key);
            out.push('=');
            out.push_str(&value);
            out.push_str("\n\n");
        }

        out
    }

    /// Generate Markdown documentation for this schema.
    pub fn generate_docs(&self) -> String {
        let title = self.meta.name.as_deref().unwrap_or("DotenvPP Configuration");
        let mut out = format!("# {title}\n\n");

        if let Some(description) = &self.meta.description {
            out.push_str(description);
            out.push_str("\n\n");
        }

        out.push_str("| Variable | Type | Required | Default | Secret | Description |\n");
        out.push_str("|---|---|---:|---|---:|---|\n");

        for (key, spec) in &self.vars {
            let default = spec.default.as_ref().map(toml_value_to_env_string).unwrap_or_default();
            let description = spec.description.clone().unwrap_or_default();
            out.push_str(&format!(
                "| `{}` | `{}` | {} | `{}` | {} | {} |\n",
                key,
                spec.ty.as_str(),
                spec.required,
                markdown_escape(&default),
                spec.secret,
                markdown_escape(&description)
            ));
        }

        out
    }

    /// Export this schema as JSON Schema draft-07 compatible JSON.
    pub fn to_json_schema_value(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for (key, spec) in &self.vars {
            if spec.required && spec.default.is_none() {
                required.push(serde_json::Value::String(key.clone()));
            }

            let mut property = json_schema_for_spec(spec);
            if let Some(description) = &spec.description {
                property["description"] = serde_json::Value::String(description.clone());
            }
            if let Some(default) = &spec.default {
                property["default"] = toml_to_json(default);
            }
            properties.insert(key.clone(), property);
        }

        json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": self.meta.name.as_deref().unwrap_or("DotenvPP schema"),
            "description": self.meta.description.as_deref().unwrap_or(""),
            "type": "object",
            "required": required,
            "properties": properties,
            "additionalProperties": true
        })
    }

    /// Export this schema as pretty JSON Schema.
    pub fn to_json_schema_string(&self) -> String {
        serde_json::to_string_pretty(&self.to_json_schema_value())
            .unwrap_or_else(|_| "{}".to_owned())
    }

    fn validate_shape(&self) -> Result<(), SchemaError> {
        for (key, spec) in &self.vars {
            if !is_valid_key(key) {
                return Err(SchemaError::Invalid(format!(
                    "invalid schema key `{key}`; keys must match dotenv variable naming"
                )));
            }

            if spec.ty == ConfigType::Enum && spec.values.is_empty() {
                return Err(SchemaError::Invalid(format!(
                    "`{key}` uses type `enum` but has no `values`"
                )));
            }

            if spec.ty == ConfigType::Regex && spec.pattern.is_none() {
                return Err(SchemaError::Invalid(format!(
                    "`{key}` uses type `regex` but has no `pattern`"
                )));
            }

            if let Some(range) = &spec.range {
                if range.len() != 2 {
                    return Err(SchemaError::Invalid(format!(
                        "`{key}` range must contain exactly two values"
                    )));
                }
            }

            if let Some(pattern) = &spec.pattern {
                Regex::new(pattern).map_err(|err| {
                    SchemaError::Invalid(format!("`{key}` has invalid regex pattern: {err}"))
                })?;
            }
        }

        Ok(())
    }
}

/// Infer a schema from parsed `.env` pairs.
pub fn infer_schema(pairs: &[EnvPair]) -> SchemaDocument {
    let mut vars = BTreeMap::new();

    for pair in pairs {
        vars.insert(pair.key.clone(), infer_spec(&pair.key, &pair.value));
    }

    SchemaDocument {
        meta: SchemaMeta {
            name: Some("generated".to_owned()),
            version: Some("1.0".to_owned()),
            description: Some("Generated by dotenvpp schema init".to_owned()),
        },
        vars,
    }
}

/// Generate a TOML schema from parsed `.env` pairs.
pub fn infer_schema_toml(pairs: &[EnvPair]) -> String {
    let schema = infer_schema(pairs);
    let mut out = String::new();
    out.push_str("[meta]\n");
    out.push_str("name = \"generated\"\n");
    out.push_str("version = \"1.0\"\n");
    out.push_str("description = \"Generated by dotenvpp schema init\"\n\n");

    for (key, spec) in schema.vars {
        out.push_str("[vars.");
        out.push_str(&key);
        out.push_str("]\n");
        out.push_str("type = \"");
        out.push_str(spec.ty.as_str());
        out.push_str("\"\n");
        out.push_str("required = true\n");
        if spec.secret {
            out.push_str("secret = true\n");
        }
        if spec.ty == ConfigType::Enum && !spec.values.is_empty() {
            out.push_str("values = [");
            out.push_str(
                &spec
                    .values
                    .iter()
                    .map(|value| format!("\"{}\"", toml_escape(value)))
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            out.push_str("]\n");
        }
        out.push('\n');
    }

    out
}

fn infer_spec(key: &str, value: &str) -> VarSpec {
    let ty = if parse_bool(value).is_some() {
        ConfigType::Bool
    } else if value.parse::<u16>().is_ok() && key.ends_with("PORT") {
        ConfigType::Port
    } else if value.parse::<i32>().is_ok() {
        ConfigType::I32
    } else if value.parse::<f64>().is_ok() {
        ConfigType::F64
    } else if parse_url_scheme(value).is_some() {
        ConfigType::Url
    } else if value.parse::<IpAddr>().is_ok() {
        ConfigType::Ip
    } else if parse_duration_seconds(value).is_ok() {
        ConfigType::Duration
    } else {
        ConfigType::String
    };

    VarSpec {
        ty,
        required: true,
        default: None,
        secret: looks_secret(key),
        description: None,
        values: Vec::new(),
        separator: None,
        range: None,
        min_length: None,
        max_length: None,
        pattern: None,
        protocols: Vec::new(),
        min: None,
        max: None,
    }
}

fn validate_value(key: &str, spec: &VarSpec, raw: &str) -> Result<TypedValue, String> {
    validate_length(spec, raw)?;
    validate_pattern(spec, raw)?;

    let typed = match spec.ty {
        ConfigType::String => TypedValue::String(raw.to_owned()),
        ConfigType::Bool => {
            TypedValue::Bool(parse_bool(raw).ok_or_else(|| "expected a boolean".to_owned())?)
        }
        ConfigType::I32 => {
            let value = raw.parse::<i32>().map_err(|_| "expected i32".to_owned())?;
            validate_numeric_range(spec, value as f64)?;
            TypedValue::I64(i64::from(value))
        }
        ConfigType::I64 => {
            let value = raw.parse::<i64>().map_err(|_| "expected i64".to_owned())?;
            validate_numeric_range(spec, value as f64)?;
            TypedValue::I64(value)
        }
        ConfigType::U16 => {
            let value = raw.parse::<u16>().map_err(|_| "expected u16".to_owned())?;
            validate_numeric_range(spec, f64::from(value))?;
            TypedValue::U64(u64::from(value))
        }
        ConfigType::U32 => {
            let value = raw.parse::<u32>().map_err(|_| "expected u32".to_owned())?;
            validate_numeric_range(spec, value as f64)?;
            TypedValue::U64(u64::from(value))
        }
        ConfigType::U64 => {
            let value = raw.parse::<u64>().map_err(|_| "expected u64".to_owned())?;
            validate_numeric_range(spec, value as f64)?;
            TypedValue::U64(value)
        }
        ConfigType::F64 => {
            let value = raw.parse::<f64>().map_err(|_| "expected f64".to_owned())?;
            validate_numeric_range(spec, value)?;
            TypedValue::F64(value)
        }
        ConfigType::Url => {
            let scheme = parse_url_scheme(raw).ok_or_else(|| "expected URL".to_owned())?;
            if !spec.protocols.is_empty() && !spec.protocols.iter().any(|p| p == scheme) {
                return Err(format!(
                    "URL scheme `{}` is not allowed; expected one of {}",
                    scheme,
                    spec.protocols.join(", ")
                ));
            }
            TypedValue::String(raw.to_owned())
        }
        ConfigType::Email => {
            if !is_email(raw) {
                return Err("expected email address".to_owned());
            }
            TypedValue::String(raw.to_owned())
        }
        ConfigType::Ip => {
            let ip = raw.parse::<IpAddr>().map_err(|_| "expected IP address".to_owned())?;
            TypedValue::String(ip.to_string())
        }
        ConfigType::Port => {
            let value = raw.parse::<u16>().map_err(|_| "expected port".to_owned())?;
            if value == 0 {
                return Err("port must be between 1 and 65535".to_owned());
            }
            validate_numeric_range(spec, f64::from(value))?;
            TypedValue::U64(u64::from(value))
        }
        ConfigType::Duration => {
            let seconds = parse_duration_seconds(raw)?;
            validate_duration_range(spec, seconds)?;
            TypedValue::Duration {
                seconds,
                original: raw.to_owned(),
            }
        }
        ConfigType::Datetime => {
            if !looks_like_rfc3339(raw) {
                return Err("expected RFC3339 datetime".to_owned());
            }
            TypedValue::String(raw.to_owned())
        }
        ConfigType::Regex => {
            validate_pattern(spec, raw)?;
            TypedValue::String(raw.to_owned())
        }
        ConfigType::Path => {
            if raw.is_empty() {
                return Err("path must not be empty".to_owned());
            }
            let path = PathBuf::from(raw);
            TypedValue::String(path.to_string_lossy().into_owned())
        }
        ConfigType::Enum => {
            if !spec.values.iter().any(|value| value == raw) {
                return Err(format!("expected one of {}", spec.values.join(", ")));
            }
            TypedValue::String(raw.to_owned())
        }
        ConfigType::StringArray => {
            let separator = spec.separator.as_deref().unwrap_or(",");
            let values = split_array(raw, separator).into_iter().map(TypedValue::String).collect();
            TypedValue::Array(values)
        }
        ConfigType::I32Array => {
            let separator = spec.separator.as_deref().unwrap_or(",");
            let mut values = Vec::new();
            for item in split_array(raw, separator) {
                let parsed =
                    item.parse::<i32>().map_err(|_| format!("array item `{item}` is not i32"))?;
                values.push(TypedValue::I64(i64::from(parsed)));
            }
            TypedValue::Array(values)
        }
    };

    if spec.secret && raw.is_empty() && spec.required {
        return Err(format!("secret `{key}` must not be empty"));
    }

    Ok(typed)
}

fn validate_length(spec: &VarSpec, raw: &str) -> Result<(), String> {
    let len = raw.chars().count();

    if let Some(min) = spec.min_length {
        if len < min {
            return Err(format!("length must be at least {min} characters"));
        }
    }

    if let Some(max) = spec.max_length {
        if len > max {
            return Err(format!("length must be at most {max} characters"));
        }
    }

    Ok(())
}

fn validate_pattern(spec: &VarSpec, raw: &str) -> Result<(), String> {
    let Some(pattern) = &spec.pattern else {
        return Ok(());
    };
    let regex = Regex::new(pattern).map_err(|err| format!("invalid regex pattern: {err}"))?;

    if regex.is_match(raw) {
        Ok(())
    } else {
        Err(format!("value does not match pattern `{pattern}`"))
    }
}

fn validate_numeric_range(spec: &VarSpec, value: f64) -> Result<(), String> {
    let Some(range) = &spec.range else {
        return Ok(());
    };
    if range.len() != 2 {
        return Err("range must contain exactly two values".to_owned());
    }

    let min = toml_value_as_f64(&range[0]).ok_or_else(|| "range min is not numeric".to_owned())?;
    let max = toml_value_as_f64(&range[1]).ok_or_else(|| "range max is not numeric".to_owned())?;
    if value < min || value > max {
        return Err(format!("value must be in range [{min}, {max}]"));
    }

    Ok(())
}

fn validate_duration_range(spec: &VarSpec, seconds: u64) -> Result<(), String> {
    if let Some(min) = &spec.min {
        let min_seconds = toml_value_as_duration(min)?;
        if seconds < min_seconds {
            return Err(format!("duration must be at least {min_seconds}s"));
        }
    }

    if let Some(max) = &spec.max {
        let max_seconds = toml_value_as_duration(max)?;
        if seconds > max_seconds {
            return Err(format!("duration must be at most {max_seconds}s"));
        }
    }

    Ok(())
}

/// Parse duration strings such as `30s`, `5m`, `1h`, and `2d`.
pub fn parse_duration_seconds(input: &str) -> Result<u64, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("duration must not be empty".to_owned());
    }

    let split_at = trimmed.find(|ch: char| !ch.is_ascii_digit()).unwrap_or(trimmed.len());
    let number = &trimmed[..split_at];
    let unit = &trimmed[split_at..];
    let amount = number
        .parse::<u64>()
        .map_err(|_| "duration amount must be an unsigned integer".to_owned())?;

    let multiplier = match unit {
        "" | "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => 60,
        "h" | "hr" | "hrs" | "hour" | "hours" => 60 * 60,
        "d" | "day" | "days" => 24 * 60 * 60,
        "ms" => {
            return Ok(amount / 1000);
        }
        other => return Err(format!("unsupported duration unit `{other}`")),
    };

    amount.checked_mul(multiplier).ok_or_else(|| "duration overflow".to_owned())
}

fn parse_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Some(true),
        "false" | "0" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn is_email(value: &str) -> bool {
    Regex::new(r"^[^@\s]+@[^@\s]+\.[^@\s]+$")
        .map(|regex| regex.is_match(value))
        .unwrap_or(false)
}

fn parse_url_scheme(value: &str) -> Option<&str> {
    let (scheme, rest) = value.split_once("://")?;
    let mut chars = scheme.chars();
    let first = chars.next()?;

    if !first.is_ascii_alphabetic() {
        return None;
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
        return None;
    }
    if rest.is_empty() || rest.chars().any(char::is_whitespace) {
        return None;
    }

    Some(scheme)
}

fn looks_like_rfc3339(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20 {
        return false;
    }

    let fixed = [(4, b'-'), (7, b'-'), (10, b'T'), (13, b':'), (16, b':')];
    if fixed.iter().any(|(idx, expected)| bytes.get(*idx) != Some(expected)) {
        return false;
    }

    let digit_positions = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18];
    if digit_positions
        .iter()
        .any(|idx| !bytes.get(*idx).is_some_and(u8::is_ascii_digit))
    {
        return false;
    }

    value.ends_with('Z')
        || value.get(19..).is_some_and(|tail| tail.contains('+') || tail.contains('-'))
}

fn split_array(raw: &str, separator: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return Vec::new();
    }
    raw.split(separator)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn toml_value_to_env_string(value: &toml::Value) -> String {
    match value {
        toml::Value::String(value) => value.clone(),
        toml::Value::Integer(value) => value.to_string(),
        toml::Value::Float(value) => value.to_string(),
        toml::Value::Boolean(value) => value.to_string(),
        toml::Value::Datetime(value) => value.to_string(),
        toml::Value::Array(values) => {
            values.iter().map(toml_value_to_env_string).collect::<Vec<_>>().join(",")
        }
        toml::Value::Table(_) => String::new(),
    }
}

fn toml_value_as_f64(value: &toml::Value) -> Option<f64> {
    match value {
        toml::Value::Integer(value) => Some(*value as f64),
        toml::Value::Float(value) => Some(*value),
        toml::Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn toml_value_as_duration(value: &toml::Value) -> Result<u64, String> {
    match value {
        toml::Value::Integer(value) if *value >= 0 => Ok(*value as u64),
        toml::Value::String(value) => parse_duration_seconds(value),
        _ => Err("duration bound must be a non-negative integer or duration string".to_owned()),
    }
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(value) => serde_json::Value::String(value.clone()),
        toml::Value::Integer(value) => json!(value),
        toml::Value::Float(value) => json!(value),
        toml::Value::Boolean(value) => json!(value),
        toml::Value::Datetime(value) => serde_json::Value::String(value.to_string()),
        toml::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(toml_to_json).collect())
        }
        toml::Value::Table(values) => {
            let mut map = serde_json::Map::new();
            for (key, value) in values {
                map.insert(key.clone(), toml_to_json(value));
            }
            serde_json::Value::Object(map)
        }
    }
}

fn json_schema_for_spec(spec: &VarSpec) -> serde_json::Value {
    let mut value = match spec.ty {
        ConfigType::String
        | ConfigType::Url
        | ConfigType::Email
        | ConfigType::Ip
        | ConfigType::Duration
        | ConfigType::Datetime
        | ConfigType::Regex
        | ConfigType::Path
        | ConfigType::Enum => json!({"type": "string"}),
        ConfigType::Bool => json!({"type": "boolean"}),
        ConfigType::I32 | ConfigType::I64 => json!({"type": "integer"}),
        ConfigType::U16 | ConfigType::U32 | ConfigType::U64 | ConfigType::Port => {
            json!({"type": "integer", "minimum": 0})
        }
        ConfigType::F64 => json!({"type": "number"}),
        ConfigType::StringArray => json!({"type": "array", "items": {"type": "string"}}),
        ConfigType::I32Array => json!({"type": "array", "items": {"type": "integer"}}),
    };

    match spec.ty {
        ConfigType::Url => value["format"] = json!("uri"),
        ConfigType::Email => value["format"] = json!("email"),
        ConfigType::Ip => value["format"] = json!("ip"),
        ConfigType::Datetime => value["format"] = json!("date-time"),
        ConfigType::Enum => value["enum"] = json!(spec.values),
        ConfigType::Regex => {
            if let Some(pattern) = &spec.pattern {
                value["pattern"] = json!(pattern);
            }
        }
        _ => {}
    }

    if let Some(min_length) = spec.min_length {
        value["minLength"] = json!(min_length);
    }
    if let Some(max_length) = spec.max_length {
        value["maxLength"] = json!(max_length);
    }
    if let Some(pattern) = &spec.pattern {
        value["pattern"] = json!(pattern);
    }
    if let Some(range) = &spec.range {
        if range.len() == 2 {
            value["minimum"] = toml_to_json(&range[0]);
            value["maximum"] = toml_to_json(&range[1]);
        }
    }

    value
}

fn placeholder_for_type(ty: &ConfigType) -> &'static str {
    match ty {
        ConfigType::String | ConfigType::Regex => "value",
        ConfigType::Bool => "false",
        ConfigType::I32 | ConfigType::I64 => "0",
        ConfigType::U16 | ConfigType::U32 | ConfigType::U64 => "0",
        ConfigType::F64 => "0.0",
        ConfigType::Url => "https://example.com",
        ConfigType::Email => "user@example.com",
        ConfigType::Ip => "127.0.0.1",
        ConfigType::Port => "8080",
        ConfigType::Duration => "30s",
        ConfigType::Datetime => "2026-01-01T00:00:00Z",
        ConfigType::Path => "./path",
        ConfigType::Enum => "",
        ConfigType::StringArray => "one,two",
        ConfigType::I32Array => "1,2",
    }
}

fn looks_secret(key: &str) -> bool {
    let key = key.to_ascii_uppercase();
    ["SECRET", "TOKEN", "PASSWORD", "PASS", "API_KEY", "PRIVATE_KEY"]
        .iter()
        .any(|needle| key.contains(needle))
}

fn is_valid_key(key: &str) -> bool {
    let mut bytes = key.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != b'_' {
        return false;
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'.')
}

fn markdown_escape(input: &str) -> String {
    input.replace('|', "\\|").replace('\n', " ")
}

fn toml_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn validates_core_types_defaults_and_warnings() {
        let schema = SchemaDocument::from_toml_str(
            r#"
            [vars.PORT]
            type = "u16"
            default = 8080
            range = [1024, 65535]

            [vars.LOG_LEVEL]
            type = "enum"
            values = ["debug", "info", "warn"]
            required = true

            [vars.API_KEY]
            type = "string"
            required = true
            secret = true
            min_length = 8
            "#,
        )
        .unwrap();

        let pairs = vec![
            EnvPair {
                key: "LOG_LEVEL".to_owned(),
                value: "info".to_owned(),
                line: 1,
            },
            EnvPair {
                key: "API_KEY".to_owned(),
                value: "abcdefgh".to_owned(),
                line: 2,
            },
            EnvPair {
                key: "EXTRA".to_owned(),
                value: "yes".to_owned(),
                line: 3,
            },
        ];

        let report = schema.validate_pairs(&pairs);
        assert!(report.is_ok());
        assert_eq!(report.warnings().count(), 1);
        assert_eq!(report.entries["PORT"].source, ValueSource::Default);
    }

    #[test]
    fn rejects_invalid_values() {
        let schema = SchemaDocument::from_toml_str(
            r#"
            [vars.PORT]
            type = "port"
            required = true

            [vars.EMAIL]
            type = "email"
            required = true
            "#,
        )
        .unwrap();

        let pairs = vec![
            EnvPair {
                key: "PORT".to_owned(),
                value: "0".to_owned(),
                line: 1,
            },
            EnvPair {
                key: "EMAIL".to_owned(),
                value: "not-email".to_owned(),
                line: 2,
            },
        ];

        let report = schema.validate_pairs(&pairs);
        assert_eq!(report.errors().count(), 2);
    }

    #[test]
    fn generates_example_docs_and_json_schema() {
        let schema = SchemaDocument::from_toml_str(
            r#"
            [meta]
            name = "app"
            description = "Example app"

            [vars.API_KEY]
            type = "string"
            required = true
            secret = true
            description = "API key"

            [vars.PORT]
            type = "port"
            default = 8080
            "#,
        )
        .unwrap();

        let example = schema.generate_example();
        assert!(example.contains("API_KEY="));
        assert!(!example.contains("secret"));
        assert!(schema.generate_docs().contains("| `PORT` |"));
        assert!(schema.to_json_schema_string().contains("\"PORT\""));
    }

    #[test]
    fn parses_duration_units() {
        assert_eq!(parse_duration_seconds("30s").unwrap(), 30);
        assert_eq!(parse_duration_seconds("2m").unwrap(), 120);
        assert_eq!(parse_duration_seconds("1h").unwrap(), 3600);
        assert_eq!(parse_duration_seconds("1d").unwrap(), 86_400);
    }

    #[test]
    fn infers_schema_from_pairs() {
        let pairs = vec![
            EnvPair {
                key: "PORT".to_owned(),
                value: "3000".to_owned(),
                line: 1,
            },
            EnvPair {
                key: "API_TOKEN".to_owned(),
                value: "secret".to_owned(),
                line: 2,
            },
        ];

        let toml = infer_schema_toml(&pairs);
        assert!(toml.contains("[vars.PORT]"));
        assert!(toml.contains("type = \"port\""));
        assert!(toml.contains("secret = true"));
    }
}
