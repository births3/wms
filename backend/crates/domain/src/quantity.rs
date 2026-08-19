//! WMS 数量统一使用四位小数十进制定点数，并以 JSON 字符串传输。

pub type Quantity = rust_decimal::Decimal;

#[cfg(test)]
mod tests {
    use super::Quantity;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    struct Payload {
        quantity: Quantity,
    }

    #[test]
    fn quantity_json_is_a_decimal_string() {
        let value: Payload = serde_json::from_str(r#"{"quantity":"50.5000"}"#).unwrap();
        assert_eq!(value.quantity.scale(), 4);
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            r#"{"quantity":"50.5000"}"#
        );
    }
}
