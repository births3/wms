use std::collections::BTreeSet;

use serde_json::Value;

use super::SystemDictionaryError;

pub(super) fn validate_params(schema: &Value, params: &Value) -> Result<(), SystemDictionaryError> {
    let Some(params_object) = params.as_object() else {
        return Err(SystemDictionaryError::ParamInvalid {
            field: "$".to_string(),
            message: "params 必须是 JSON object".to_string(),
        });
    };

    if let Some(required) = schema.get("required").and_then(Value::as_array) {
        for field in required.iter().filter_map(Value::as_str) {
            if !params_object.contains_key(field) {
                return Err(SystemDictionaryError::ParamInvalid {
                    field: field.to_string(),
                    message: "缺少必填参数".to_string(),
                });
            }
        }
    }

    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return Ok(());
    };
    for (field, property) in properties {
        let Some(value) = params_object.get(field) else {
            continue;
        };
        if let Some(expected_type) = property.get("type").and_then(Value::as_str) {
            let valid = match expected_type {
                "string" => value.is_string(),
                "boolean" => value.is_boolean(),
                "array" => value.is_array(),
                "object" => value.is_object(),
                "number" => value.is_number(),
                "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
                _ => {
                    return Err(SystemDictionaryError::ParamInvalid {
                        field: field.clone(),
                        message: format!("不支持的参数类型 {expected_type}"),
                    });
                }
            };
            if !valid {
                return Err(SystemDictionaryError::ParamInvalid {
                    field: field.clone(),
                    message: format!("参数类型必须是 {expected_type}"),
                });
            }
        }
        if let Some(number) = value.as_f64() {
            if let Some(minimum) = property.get("minimum").and_then(Value::as_f64) {
                if number < minimum {
                    return Err(SystemDictionaryError::ParamInvalid {
                        field: field.clone(),
                        message: format!("参数不能小于 {minimum}"),
                    });
                }
            }
            if let Some(maximum) = property.get("maximum").and_then(Value::as_f64) {
                if number > maximum {
                    return Err(SystemDictionaryError::ParamInvalid {
                        field: field.clone(),
                        message: format!("参数不能大于 {maximum}"),
                    });
                }
            }
        }
        if let Some(allowed) = property.get("enum").and_then(Value::as_array) {
            let Some(text) = value.as_str() else {
                return Err(SystemDictionaryError::ParamInvalid {
                    field: field.clone(),
                    message: "参数必须是字符串".to_string(),
                });
            };
            let ok = allowed
                .iter()
                .filter_map(Value::as_str)
                .any(|item| item == text);
            if !ok {
                return Err(SystemDictionaryError::ParamInvalid {
                    field: field.clone(),
                    message: "参数值不在允许枚举中".to_string(),
                });
            }
        }
    }
    Ok(())
}

pub(super) fn allowed_owner_params(policy: &Value) -> BTreeSet<String> {
    policy
        .get("allowed_owner_params")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}
