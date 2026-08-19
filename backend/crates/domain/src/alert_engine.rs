use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AlertConditionError {
    InvalidExpression,
    UnsupportedOperator,
}

pub fn matches_alert_condition(
    expression: &str,
    payload: &Value,
) -> Result<bool, AlertConditionError> {
    let expression = expression.trim();
    if expression.is_empty() || expression == "{}" {
        return Ok(true);
    }
    let condition: Value =
        serde_json::from_str(expression).map_err(|_| AlertConditionError::InvalidExpression)?;
    let object = condition
        .as_object()
        .ok_or(AlertConditionError::InvalidExpression)?;
    let field = object
        .get("field")
        .and_then(Value::as_str)
        .ok_or(AlertConditionError::InvalidExpression)?;
    let left = value_at_path(payload, field).ok_or(AlertConditionError::InvalidExpression)?;
    let right = if let Some(value_field) = object.get("value_field").and_then(Value::as_str) {
        value_at_path(payload, value_field).ok_or(AlertConditionError::InvalidExpression)?
    } else {
        object
            .get("value")
            .ok_or(AlertConditionError::InvalidExpression)?
    };
    match object.get("op").and_then(Value::as_str) {
        Some("eq") => Ok(left == right),
        Some("ne") => Ok(left != right),
        Some("lt") => compare(left, right).map(|ordering| ordering.is_lt()),
        Some("lte") => compare(left, right).map(|ordering| !ordering.is_gt()),
        Some("gt") => compare(left, right).map(|ordering| ordering.is_gt()),
        Some("gte") => compare(left, right).map(|ordering| !ordering.is_lt()),
        _ => Err(AlertConditionError::UnsupportedOperator),
    }
}

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(value, |current, segment| current.get(segment))
}

fn compare(left: &Value, right: &Value) -> Result<std::cmp::Ordering, AlertConditionError> {
    if let (Some(left), Some(right)) = (left.as_f64(), right.as_f64()) {
        return left
            .partial_cmp(&right)
            .ok_or(AlertConditionError::InvalidExpression);
    }
    if let (Some(left), Some(right)) = (left.as_str(), right.as_str()) {
        return Ok(left.cmp(right));
    }
    Err(AlertConditionError::InvalidExpression)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_condition_matches_literal_and_payload_fields() {
        let payload = serde_json::json!({"quantity": 5, "safety_stock": 10});
        assert!(
            matches_alert_condition(r#"{"field":"quantity","op":"lt","value":10}"#, &payload,)
                .expect("literal condition should evaluate")
        );
        assert!(matches_alert_condition(
            r#"{"field":"quantity","op":"lt","value_field":"safety_stock"}"#,
            &payload,
        )
        .expect("field condition should evaluate"));
        assert!(
            !matches_alert_condition(r#"{"field":"quantity","op":"gt","value":10}"#, &payload,)
                .expect("non-matching condition should evaluate")
        );
        assert!(matches_alert_condition("", &payload).expect("empty condition should match"));
    }
}
