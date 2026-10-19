use std::{fs, path::Path};

use jsonschema::{Keyword, ValidationError, Validator, paths::Location};
use serde_json::{Map, Value};

pub struct AssetRefValidator;

impl Keyword for AssetRefValidator {
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        if let Some(value) = instance.as_str() {
            if namespaced_format(value) {
                return Ok(());
            }
        }
        Err(ValidationError::custom(
            "value must a string that has a namespace:name format",
        ))
    }

    fn is_valid(&self, instance: &Value) -> bool {
        instance.is_string()
    }
}

pub fn asset_ref_factory<'a>(
    _parent: &'a Map<String, Value>,
    value: &'a Value,
    _path: Location,
) -> Result<Box<dyn Keyword>, ValidationError<'a>> {
    if value.as_str().is_some() {
        Ok(Box::new(AssetRefValidator))
    } else {
        Err(ValidationError::schema(
            "The 'asset ref' keyword must have asset assigned to it",
        ))
    }
}

pub struct ColorValidator;

impl Keyword for ColorValidator {
    fn validate<'i>(&self, instance: &'i Value) -> Result<(), ValidationError<'i>> {
        match instance {
            serde_json::Value::Array(arr) => {
                if arr.len() == 3 || arr.len() == 4 {
                    return Ok(());
                }
            }

            serde_json::Value::String(s) => {
                if hex_format(s) {
                    return Ok(());
                }
            }

            _ => {}
        }

        Err(ValidationError::custom(
            "Must be a vec 3 or vec 4 or a hex string",
        ))
    }

    fn is_valid(&self, instance: &Value) -> bool {
        instance.is_array() || instance.is_string()
    }
}

fn color_factory<'a>(
    _parent: &'a Map<String, Value>,
    value: &'a Value,
    _path: Location,
) -> Result<Box<dyn Keyword>, ValidationError<'a>> {
    if value.as_bool() == Some(true) {
        Ok(Box::new(ColorValidator))
    } else {
        Err(ValidationError::schema(
            "The 'color' keyword must be set to true",
        ))
    }
}

pub fn build_validator(schema_path: &Path) -> Validator {
    let schema_text = fs::read_to_string(schema_path).unwrap();

    let schema_json: Value = serde_json::from_str(&schema_text).unwrap();

    jsonschema::options()
        .with_keyword("color", color_factory)
        .with_keyword("asset_ref", asset_ref_factory)
        .with_format("namespaced", namespaced_format)
        .with_format("hex", hex_format)
        .build(&schema_json)
        .expect("Invalid schema")
}

pub fn hex_format(s: &str) -> bool {
    //check if starts with # and has 6 letters after it with 0 to 9 or a to f
    s.starts_with('#') && s.len() == 7 && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn namespaced_format(s: &str) -> bool {
    let parts = s.split(":");
    parts.count() == 2
}
