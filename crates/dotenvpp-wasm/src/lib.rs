//! WASM bindings for DotenvPP.

use std::collections::HashMap;

use dotenvpp::policy::{PolicyReport, Severity};
use dotenvpp::schema::{DiagnosticSeverity, TypedValue, ValidationReport, ValueSource};
use wasm_bindgen::prelude::*;

/// Return the DotenvPP crate version.
#[wasm_bindgen]
pub fn version() -> String {
    dotenvpp::version().to_owned()
}

/// Parse `.env` content and return JSON array of `{ key, value, line }`.
#[wasm_bindgen(js_name = parse)]
pub fn parse(env_content: &str) -> Result<String, JsValue> {
    let pairs = dotenvpp::from_read(env_content.as_bytes()).map_err(js_error)?;
    let mut out = String::from("[");
    for (index, pair) in pairs.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"key\":");
        push_json_string(&mut out, &pair.key);
        out.push_str(",\"value\":");
        push_json_string(&mut out, &pair.value);
        out.push_str(",\"line\":");
        out.push_str(&pair.line.to_string());
        out.push('}');
    }
    out.push(']');
    Ok(out)
}

/// Validate `.env` content against `.env.schema` TOML and return a JSON report.
#[wasm_bindgen]
pub fn validate(env_content: &str, schema_content: &str) -> Result<String, JsValue> {
    let pairs = dotenvpp::from_read(env_content.as_bytes()).map_err(js_error)?;
    let schema =
        dotenvpp::schema::SchemaDocument::from_toml_str(schema_content).map_err(js_error)?;
    let report = schema.validate_pairs(&pairs);
    Ok(validation_report_json(&report))
}

/// Evaluate `.env.policy` TOML against `.env` content and return a JSON report.
#[wasm_bindgen(js_name = checkPolicy)]
pub fn check_policy(env_content: &str, policy_content: &str) -> Result<String, JsValue> {
    let pairs = dotenvpp::from_read(env_content.as_bytes()).map_err(js_error)?;
    let policy =
        dotenvpp::policy::PolicyDocument::from_toml_str(policy_content).map_err(js_error)?;
    let variables = pairs.into_iter().map(|pair| (pair.key, pair.value)).collect::<HashMap<_, _>>();
    let report = policy.evaluate(&variables);
    Ok(policy_report_json(&report))
}

fn js_error(err: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&err.to_string())
}

fn validation_report_json(report: &ValidationReport) -> String {
    let mut out = String::from("{\"entries\":{");
    for (index, (key, entry)) in report.entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(&mut out, key);
        out.push_str(":{\"key\":");
        push_json_string(&mut out, &entry.key);
        out.push_str(",\"value\":");
        push_typed_value_json(&mut out, &entry.value);
        out.push_str(",\"secret\":");
        out.push_str(if entry.secret {
            "true"
        } else {
            "false"
        });
        out.push_str(",\"source\":");
        push_json_string(
            &mut out,
            match entry.source {
                ValueSource::File => "file",
                ValueSource::Default => "default",
            },
        );
        out.push('}');
    }
    out.push_str("},\"diagnostics\":[");
    for (index, diagnostic) in report.diagnostics.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"severity\":");
        push_json_string(
            &mut out,
            match diagnostic.severity {
                DiagnosticSeverity::Error => "error",
                DiagnosticSeverity::Warning => "warning",
                DiagnosticSeverity::Info => "info",
            },
        );
        out.push_str(",\"key\":");
        if let Some(key) = &diagnostic.key {
            push_json_string(&mut out, key);
        } else {
            out.push_str("null");
        }
        out.push_str(",\"message\":");
        push_json_string(&mut out, &diagnostic.message);
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn policy_report_json(report: &PolicyReport) -> String {
    let mut out = String::from("{\"violations\":[");
    for (index, violation) in report.violations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"rule\":");
        push_json_string(&mut out, &violation.rule);
        out.push_str(",\"severity\":");
        push_json_string(
            &mut out,
            match violation.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            },
        );
        out.push_str(",\"message\":");
        push_json_string(&mut out, &violation.message);
        out.push('}');
    }
    out.push_str("]}");
    out
}

fn push_typed_value_json(out: &mut String, value: &TypedValue) {
    match value {
        TypedValue::String(value) => {
            out.push_str("{\"type\":\"String\",\"value\":");
            push_json_string(out, value);
            out.push('}');
        }
        TypedValue::Bool(value) => {
            out.push_str("{\"type\":\"Bool\",\"value\":");
            out.push_str(if *value {
                "true"
            } else {
                "false"
            });
            out.push('}');
        }
        TypedValue::I64(value) => {
            out.push_str("{\"type\":\"I64\",\"value\":");
            out.push_str(&value.to_string());
            out.push('}');
        }
        TypedValue::U64(value) => {
            out.push_str("{\"type\":\"U64\",\"value\":");
            out.push_str(&value.to_string());
            out.push('}');
        }
        TypedValue::F64(value) => {
            out.push_str("{\"type\":\"F64\",\"value\":");
            if value.is_finite() {
                out.push_str(&value.to_string());
            } else {
                out.push_str("null");
            }
            out.push('}');
        }
        TypedValue::Array(values) => {
            out.push_str("{\"type\":\"Array\",\"value\":[");
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                push_typed_value_json(out, item);
            }
            out.push_str("]}");
        }
        TypedValue::Duration {
            seconds,
            original,
        } => {
            out.push_str("{\"type\":\"Duration\",\"value\":{\"seconds\":");
            out.push_str(&seconds.to_string());
            out.push_str(",\"original\":");
            push_json_string(out, original);
            out.push_str("}}");
        }
    }
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            ch if ch < ' ' => {
                out.push_str("\\u00");
                let byte = ch as u8;
                out.push(nibble_to_hex(byte >> 4));
                out.push(nibble_to_hex(byte & 0x0f));
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + value - 10) as char,
        _ => '0',
    }
}
