#[derive(Clone, Debug)]
struct GeneratedPrintField {
    field_path: String,
    field_type: String,
    source_schema: String,
    display_name: String,
    group_code: String,
    group_name: String,
    description: String,
    example_value: Option<Value>,
    is_table_detail: bool,
    sort_order: i32,
}

fn generate_openapi_fields(
    openapi: &Value,
    source_schema: &str,
) -> Result<Vec<GeneratedPrintField>, PrintTemplateError> {
    let schema = schema_by_name(openapi, source_schema).ok_or_else(|| {
        PrintTemplateError::InvalidRequest(format!(
            "OpenAPI schema does not exist: {source_schema}"
        ))
    })?;
    let mut fields = Vec::new();
    collect_schema_fields(
        openapi,
        schema,
        source_schema,
        "",
        false,
        &mut vec![source_schema.to_string()],
        &mut fields,
    );
    fields.sort_by(|left, right| left.field_path.cmp(&right.field_path));
    fields.dedup_by(|left, right| left.field_path == right.field_path);
    for (index, field) in fields.iter_mut().enumerate() {
        field.sort_order = (index as i32 + 1) * 10;
    }
    if fields.is_empty() {
        return Err(PrintTemplateError::InvalidRequest(format!(
            "OpenAPI schema has no printable fields: {source_schema}"
        )));
    }
    Ok(fields)
}

fn collect_schema_fields(
    openapi: &Value,
    schema: &Value,
    source_schema: &str,
    prefix: &str,
    in_table: bool,
    stack: &mut Vec<String>,
    fields: &mut Vec<GeneratedPrintField>,
) {
    if let Some(reference) = reference_schema_name(schema) {
        collect_referenced_schema(openapi, reference, prefix, in_table, stack, fields);
        return;
    }
    if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
        for part in parts {
            collect_schema_fields(
                openapi,
                part,
                source_schema,
                prefix,
                in_table,
                stack,
                fields,
            );
        }
    }
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return;
    };
    for (name, property) in properties {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        if let Some(reference) = reference_schema_name(property) {
            collect_referenced_schema(openapi, reference, &path, in_table, stack, fields);
            continue;
        }
        if schema_type(property) == "array" {
            let array_path = format!("{path}[]");
            let Some(items) = property.get("items") else {
                continue;
            };
            if let Some(reference) = reference_schema_name(items) {
                collect_referenced_schema(openapi, reference, &array_path, true, stack, fields);
            } else if items.get("properties").is_some() {
                collect_schema_fields(
                    openapi,
                    items,
                    source_schema,
                    &array_path,
                    true,
                    stack,
                    fields,
                );
            } else {
                fields.push(generated_field(
                    &array_path,
                    schema_type(items),
                    source_schema,
                    property,
                    true,
                ));
            }
            continue;
        }
        if property.get("properties").is_some() || schema_type(property) == "object" {
            collect_schema_fields(
                openapi,
                property,
                source_schema,
                &path,
                in_table,
                stack,
                fields,
            );
            continue;
        }
        fields.push(generated_field(
            &path,
            schema_type(property),
            source_schema,
            property,
            in_table,
        ));
    }
}

fn collect_referenced_schema(
    openapi: &Value,
    reference: &str,
    prefix: &str,
    in_table: bool,
    stack: &mut Vec<String>,
    fields: &mut Vec<GeneratedPrintField>,
) {
    if stack.iter().any(|item| item == reference) || stack.len() >= 16 {
        return;
    }
    let Some(schema) = schema_by_name(openapi, reference) else {
        return;
    };
    stack.push(reference.to_string());
    collect_schema_fields(openapi, schema, reference, prefix, in_table, stack, fields);
    stack.pop();
}

fn generated_field(
    field_path: &str,
    field_type: &str,
    source_schema: &str,
    schema: &Value,
    is_table_detail: bool,
) -> GeneratedPrintField {
    let group_code = field_path
        .split('.')
        .next()
        .unwrap_or("base")
        .trim_end_matches("[]")
        .to_string();
    let group_code = if is_table_detail {
        group_code
    } else {
        "base".to_string()
    };
    GeneratedPrintField {
        field_path: field_path.to_string(),
        field_type: field_type.to_string(),
        source_schema: source_schema.to_string(),
        display_name: schema
            .get("title")
            .and_then(Value::as_str)
            .or_else(|| schema.get("description").and_then(Value::as_str))
            .unwrap_or_else(|| field_path.rsplit('.').next().unwrap_or(field_path))
            .to_string(),
        group_name: if is_table_detail {
            "明细信息".to_string()
        } else {
            "基本信息".to_string()
        },
        group_code,
        description: schema
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        example_value: schema.get("example").cloned(),
        is_table_detail,
        sort_order: 0,
    }
}

fn schema_by_name<'a>(openapi: &'a Value, name: &str) -> Option<&'a Value> {
    openapi.get("components")?.get("schemas")?.get(name)
}

fn reference_schema_name(schema: &Value) -> Option<&str> {
    schema
        .get("$ref")
        .and_then(Value::as_str)?
        .strip_prefix("#/components/schemas/")
}

fn schema_type(schema: &Value) -> &str {
    if let Some(value) = schema.get("type").and_then(Value::as_str) {
        return value;
    }
    schema
        .get("type")
        .and_then(Value::as_array)
        .and_then(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .find(|value| *value != "null")
        })
        .unwrap_or("unknown")
}
