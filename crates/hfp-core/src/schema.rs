//! Data extraction and schema validation (Phase 1.1).
//!
//! `extract` returns the `#hfp-data` JSON; `validate` checks it against the `#hfp-schema`.
//! The schema is a deliberate **subset** of JSON Schema (see `spec/format.md`): `type`,
//! `required`, `properties`, `items`, `enum`, `pattern`. It is a *data contract* — it
//! constrains the shape of the data, never the UX. Unknown keywords are ignored so a
//! schema can carry authoring hints without breaking validation.

use serde_json::Value;

use crate::{Error, Result, ValidationError, ValidationReport};

/// Extract the `#hfp-data` JSON as a string (trimmed). Errors if missing or not JSON.
pub(crate) fn extract(raw: &[u8]) -> Result<String> {
    let text = crate::canon::inner_text_by_id(raw, "hfp-data")?;
    serde_json::from_str::<Value>(&text).map_err(|_| Error::InvalidJson("hfp-data"))?;
    Ok(text)
}

/// Validate `#hfp-data` against `#hfp-schema`.
pub(crate) fn validate(raw: &[u8]) -> Result<ValidationReport> {
    let data_text = crate::canon::inner_text_by_id(raw, "hfp-data")?;
    let schema_text = crate::canon::inner_text_by_id(raw, "hfp-schema")?;
    let data: Value =
        serde_json::from_str(&data_text).map_err(|_| Error::InvalidJson("hfp-data"))?;
    let schema: Value =
        serde_json::from_str(&schema_text).map_err(|_| Error::InvalidJson("hfp-schema"))?;

    let mut errors = Vec::new();
    check(&schema, &data, "", &mut errors);
    Ok(ValidationReport {
        valid: errors.is_empty(),
        errors,
    })
}

fn err(errors: &mut Vec<ValidationError>, path: &str, message: impl Into<String>) {
    errors.push(ValidationError {
        path: if path.is_empty() {
            "(root)".into()
        } else {
            path.into()
        },
        message: message.into(),
    });
}

/// Recursively validate `data` against `schema` at `path`.
fn check(schema: &Value, data: &Value, path: &str, errors: &mut Vec<ValidationError>) {
    let Some(schema) = schema.as_object() else {
        return; // A non-object schema constrains nothing.
    };

    // type
    if let Some(ty) = schema.get("type").and_then(Value::as_str) {
        if !type_matches(ty, data) {
            err(
                errors,
                path,
                format!("expected type {ty}, got {}", json_type(data)),
            );
            return; // Further keyword checks assume the type matched.
        }
    }

    // enum
    if let Some(Value::Array(allowed)) = schema.get("enum") {
        if !allowed.contains(data) {
            err(errors, path, "value is not one of the allowed enum values");
        }
    }

    // pattern (strings)
    if let (Some(pat), Some(s)) = (schema.get("pattern").and_then(Value::as_str), data.as_str()) {
        match regex::Regex::new(pat) {
            Ok(re) if !re.is_match(s) => {
                err(errors, path, format!("does not match pattern /{pat}/"))
            }
            Err(_) => err(
                errors,
                path,
                format!("schema has an invalid pattern /{pat}/"),
            ),
            _ => {}
        }
    }

    // object: required + properties
    if let Some(obj) = data.as_object() {
        if let Some(Value::Array(required)) = schema.get("required") {
            for name in required.iter().filter_map(Value::as_str) {
                if !obj.contains_key(name) {
                    err(errors, &join(path, name), "required property is missing");
                }
            }
        }
        if let Some(Value::Object(props)) = schema.get("properties") {
            for (name, subschema) in props {
                if let Some(value) = obj.get(name) {
                    check(subschema, value, &join(path, name), errors);
                }
            }
        }
    }

    // array: items
    if let (Some(items), Some(arr)) = (schema.get("items"), data.as_array()) {
        for (i, value) in arr.iter().enumerate() {
            check(items, value, &format!("{path}[{i}]"), errors);
        }
    }
}

fn type_matches(ty: &str, v: &Value) -> bool {
    match ty {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => true, // Unknown type keyword: do not constrain.
    }
}

fn json_type(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(schema: &str, data: &str) -> String {
        format!(
            r#"<!DOCTYPE html><html><head>
            <script id="hfp-schema" type="application/json">{schema}</script>
            <script id="hfp-data" type="application/json">{data}</script>
            <script id="hfp-data-signature" type="application/pkcs7-signature"></script>
            </head><body></body></html>"#
        )
    }

    const SCHEMA: &str = r#"{
        "type": "object",
        "required": ["technician", "device"],
        "properties": {
            "technician": { "type": "string" },
            "device": { "type": "string", "pattern": "^[A-Z]" },
            "priority": { "enum": ["low", "high"] },
            "faults": { "type": "array", "items": { "type": "string" } }
        }
    }"#;

    fn validate_data(data: &str) -> ValidationReport {
        validate(doc(SCHEMA, data).as_bytes()).unwrap()
    }

    #[test]
    fn valid_data_passes() {
        let r = validate_data(r#"{"technician":"Jan","device":"Pump A","faults":["leak"]}"#);
        assert!(r.valid, "{:?}", r.errors);
    }

    #[test]
    fn missing_required_is_reported() {
        let r = validate_data(r#"{"technician":"Jan"}"#);
        assert!(!r.valid);
        assert!(r.errors.iter().any(|e| e.path == "device"));
    }

    #[test]
    fn type_mismatch_is_reported() {
        let r = validate_data(r#"{"technician":42,"device":"Pump A"}"#);
        assert!(r.errors.iter().any(|e| e.path == "technician"));
    }

    #[test]
    fn pattern_and_enum_and_array_items() {
        let r = validate_data(
            r#"{"technician":"Jan","device":"pump","priority":"urgent","faults":["ok",7]}"#,
        );
        assert!(r.errors.iter().any(|e| e.path == "device")); // pattern ^[A-Z]
        assert!(r.errors.iter().any(|e| e.path == "priority")); // enum
        assert!(r.errors.iter().any(|e| e.path == "faults[1]")); // item type
    }

    #[test]
    fn extract_returns_data_json() {
        let d = doc(SCHEMA, r#"{"technician":"Jan","device":"Pump A"}"#);
        let got = extract(d.as_bytes()).unwrap();
        assert_eq!(got, r#"{"technician":"Jan","device":"Pump A"}"#);
    }

    #[test]
    fn extract_rejects_non_json() {
        let d = doc(SCHEMA, "not json");
        assert_eq!(extract(d.as_bytes()), Err(Error::InvalidJson("hfp-data")));
    }
}
