use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;
use wms_domain::{
    CompleteArchiveRevisionRequest, MapParameterRequest, ParameterMappingStatus, Product,
    ProductMappingTraceInput, ProductPackagingLevelInput, UpdateProductRequest,
};

use crate::quality_liaison::QualityLiaisonError;

use super::{
    fail_message,
    lifecycle::{
        idempotency_key, payload_digest, prepare_message, record_convert_message, succeed_message,
        InboundMetadata,
    },
    map_parameter, map_parameter_error, response, validate_envelope, AuthContext,
    H8InboundAppState, H8InboundError, H8InboundResponse,
};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8PhysicalDimensionsInput {
    pub length_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ProductChangeInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub product_id: Option<Uuid>,
    pub product_code: String,
    pub field_name: String,
    pub new_value: Option<String>,
    pub physical_dimensions: Option<H8PhysicalDimensionsInput>,
    pub liaison_id: Option<Uuid>,
    pub asn_id: Option<Uuid>,
}

pub(super) async fn push_product_change(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8ProductChangeInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m1.master_data.write")?;
    validate_request(&ctx, &body)?;
    let idempotency_key = idempotency_key(&headers)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        idempotency_key,
        InboundMetadata {
            message_type: "product_change",
            schema_version: body.schema_version.clone(),
            external_ref: body.external_ref.clone(),
            correlation_id: body.correlation_id.clone(),
            warehouse_id: None,
            payload_digest: payload_digest(&body)?,
        },
    )
    .await?;
    if prepared.message.sync_status == "succeeded" {
        return Ok(Json(response(prepared.message, true)?));
    }
    let product = match find_product(&state, &ctx, &body).await {
        Ok(product) => product,
        Err(error) => {
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    let (request, mapping_traces) = match update_request(
        &state,
        &ctx,
        &body,
        &prepared.connector_code,
        idempotency_key,
    )
    .await
    {
        Ok(request) => request,
        Err(error) => {
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    prepared.message = record_convert_message(&state, &ctx, prepared.message).await?;
    let product = match state
        .master_data
        .update_product_with_mapping_traces(
            &ctx,
            product.id,
            request,
            mapping_traces,
            Utc::now(),
            idempotency_key,
        )
        .await
    {
        Ok(product) => product,
        Err(error) => {
            let error = super::map_master_data_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    };
    if let (Some(liaison_id), Some(asn_id)) = (body.liaison_id, body.asn_id) {
        let new_value = match normalized_new_value(&body, &product) {
            Ok(value) => value,
            Err(error) => {
                fail_message(&state, &ctx, prepared.message.id, &error).await?;
                return Err(error);
            }
        };
        let callback = CompleteArchiveRevisionRequest {
            asn_id,
            product_id: product.id,
            product_code: body.product_code.trim().to_string(),
            field_name: body.field_name.trim().to_string(),
            new_value,
        };
        if let Err(error) = state
            .quality_liaison
            .complete_archive_revision_sync(
                &ctx,
                liaison_id,
                callback,
                Utc::now(),
                &format!("{idempotency_key}:archive-closeout"),
            )
            .await
        {
            let error = map_quality_liaison_error(error);
            fail_message(&state, &ctx, prepared.message.id, &error).await?;
            return Err(error);
        }
    }
    let replayed = prepared.replayed;
    prepared.message =
        succeed_message(&state, &ctx, prepared.message.id, &product.id.to_string()).await?;
    Ok(Json(response(prepared.message, replayed)?))
}

fn validate_request(
    ctx: &AuthContext,
    body: &H8ProductChangeInboundRequest,
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        None,
    )?;
    let field_name = body.field_name.trim();
    let value_valid = if field_name == "physical_dimensions" {
        body.new_value.is_none()
            && body.physical_dimensions.as_ref().is_some_and(|dimensions| {
                [
                    dimensions.length_mm,
                    dimensions.width_mm,
                    dimensions.height_mm,
                ]
                .into_iter()
                .all(|value| value.is_finite() && value > 0.0)
            })
    } else {
        body.physical_dimensions.is_none()
            && body
                .new_value
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    };
    if body.product_code.trim().is_empty()
        || field_name.is_empty()
        || !value_valid
        || body.liaison_id.is_some() != body.asn_id.is_some()
    {
        return Err(H8InboundError::Unprocessable(
            "required product change field is invalid".to_string(),
        ));
    }
    Ok(())
}

async fn find_product(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    body: &H8ProductChangeInboundRequest,
) -> Result<Product, H8InboundError> {
    let result = match body.product_id {
        Some(product_id) => state.master_data.get_product(ctx, product_id).await,
        None => {
            state
                .master_data
                .get_product_by_code(ctx, body.product_code.trim())
                .await
        }
    };
    let product = result.map_err(super::map_master_data_error)?;
    if product.product_code != body.product_code.trim() {
        return Err(H8InboundError::Unprocessable(
            "product id and code do not match".to_string(),
        ));
    }
    Ok(product)
}

async fn update_request(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    body: &H8ProductChangeInboundRequest,
    connector_code: &str,
    idempotency_key: &str,
) -> Result<(UpdateProductRequest, Vec<ProductMappingTraceInput>), H8InboundError> {
    let source_field = body.field_name.trim();
    let field = match source_field {
        "approval_number" => "approval_no",
        "specification" => "spec",
        "special_drug_category" => "special_drug_category_code",
        value => value,
    };
    if field == "physical_dimensions" {
        let dimensions = body.physical_dimensions.as_ref().ok_or_else(|| {
            H8InboundError::Unprocessable("physical_dimensions is required".to_string())
        })?;
        let mut request = empty_update_product_request();
        set_product_dimensions(&mut request, dimensions);
        return Ok((request, Vec::new()));
    }
    let source_value = body
        .new_value
        .as_deref()
        .ok_or_else(|| H8InboundError::Unprocessable("new_value is required".to_string()))?
        .trim();
    let mut value = source_value.to_string();
    let mut mapping_traces = Vec::new();
    if field == "packaging_levels" {
        let source_levels: Vec<super::product_master::H8ProductPackagingLevelInput> =
            serde_json::from_str(source_value).map_err(|_| {
                H8InboundError::Unprocessable("packaging_levels must be a JSON array".to_string())
            })?;
        if source_levels.is_empty() {
            return Err(H8InboundError::Unprocessable(
                "packaging_levels must not be empty".to_string(),
            ));
        }
        let mut levels = Vec::with_capacity(source_levels.len());
        for (index, level) in source_levels.into_iter().enumerate() {
            let source_unit = level.unit.trim();
            if source_unit.is_empty() {
                return Err(H8InboundError::Unprocessable(
                    "packaging unit is required".to_string(),
                ));
            }
            let mapped = map_value(
                state,
                ctx,
                "unit_pack",
                source_unit,
                body,
                connector_code,
                idempotency_key,
                &format!("unit_pack:{index}"),
                true,
            )
            .await?;
            mapping_traces.push(ProductMappingTraceInput {
                field_name: format!("packaging_levels[{index}].unit_code"),
                rule_id: mapped.rule_id,
                source_system: connector_code.to_string(),
                source_value: source_unit.to_string(),
                target_value: Some(mapped.value.clone()),
            });
            levels.push(ProductPackagingLevelInput {
                unit_name: super::product_master::unit_name(&mapped.value).to_string(),
                unit_code: mapped.value,
                ratio_to_base: level.ratio_to_base,
                is_base: level.is_base,
                is_default: level.is_default,
                sort_order: level.sort_order,
            });
        }
        let mut request = empty_update_product_request();
        request.packaging_levels = Some(levels);
        return Ok((request, mapping_traces));
    }
    let dict_code = match field {
        "storage_condition" => Some(("storage_condition", true)),
        "dosage_form" => Some(("dosage_form", false)),
        "special_drug_category_code" => Some(("special_drug_category", true)),
        "status" => Some(("product_status", true)),
        _ => None,
    };
    if let Some((dict_code, required)) = dict_code {
        let mapped = map_value(
            state,
            ctx,
            dict_code,
            &value,
            body,
            connector_code,
            idempotency_key,
            dict_code,
            required,
        )
        .await?;
        value = mapped.value.clone();
        mapping_traces.push(ProductMappingTraceInput {
            field_name: field.to_string(),
            rule_id: mapped.rule_id,
            source_system: connector_code.to_string(),
            source_value: source_value.to_string(),
            target_value: Some(mapped.value),
        });
    }
    let mut request = empty_update_product_request();
    match field {
        "volume_cm3" => request.volume_cm3 = Some(Some(positive_number(source_value, field)?)),
        "weight_g" => request.weight_g = Some(Some(positive_number(source_value, field)?)),
        _ => {}
    }
    match field {
        "product_name" => request.product_name = Some(value),
        "approval_no" => request.approval_no = Some(Some(value)),
        "spec" => request.spec = Some(value),
        "dosage_form" => request.dosage_form = Some(Some(value)),
        "manufacturer" => request.manufacturer = Some(Some(value)),
        "special_drug_category_code" => request.special_drug_category_code = Some(value),
        "udi_code" => request.udi_code = Some(Some(value)),
        "electronic_regulatory_code" => request.electronic_regulatory_code = Some(Some(value)),
        "status" => request.status = Some(value),
        "storage_condition" => request.attrs = Some(json!({"storage_condition": value})),
        "volume_cm3" | "weight_g" => {}
        _ => {
            return Err(H8InboundError::Unprocessable(format!(
                "unsupported product change field: {source_field}"
            )));
        }
    }
    Ok((request, mapping_traces))
}

fn empty_update_product_request() -> UpdateProductRequest {
    UpdateProductRequest {
        product_name: None,
        is_external_use: None,
        is_fragrant: None,
        approval_no: None,
        spec: None,
        dosage_form: None,
        manufacturer: None,
        special_drug_category_code: None,
        udi_code: None,
        electronic_regulatory_code: None,
        barcode_69: None,
        length_mm: None,
        width_mm: None,
        height_mm: None,
        volume_cm3: None,
        weight_g: None,
        packaging_levels: None,
        status: None,
        attrs: None,
    }
}

fn positive_number(value: &str, field: &str) -> Result<f64, H8InboundError> {
    let parsed = value
        .parse::<f64>()
        .map_err(|_| H8InboundError::Unprocessable(format!("{field} must be a positive number")))?;
    if !parsed.is_finite() || parsed <= 0.0 {
        return Err(H8InboundError::Unprocessable(format!(
            "{field} must be a positive number"
        )));
    }
    Ok(parsed)
}

fn set_product_dimensions(
    request: &mut UpdateProductRequest,
    dimensions: &H8PhysicalDimensionsInput,
) {
    request.length_mm = Some(Some(dimensions.length_mm));
    request.width_mm = Some(Some(dimensions.width_mm));
    request.height_mm = Some(Some(dimensions.height_mm));
}

struct MappedProductChangeValue {
    value: String,
    rule_id: Option<Uuid>,
}

async fn map_value(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    dict_code: &str,
    source_value: &str,
    body: &H8ProductChangeInboundRequest,
    connector_code: &str,
    idempotency_key: &str,
    mapping_key: &str,
    required: bool,
) -> Result<MappedProductChangeValue, H8InboundError> {
    let mapped = map_parameter(
        &state.mappings,
        ctx,
        &MapParameterRequest {
            dict_code: dict_code.to_string(),
            source_value: source_value.to_string(),
            source_system: Some(connector_code.to_string()),
            source_record_id: Some(body.external_ref.clone()),
        },
        &format!("{idempotency_key}:mpm:{mapping_key}"),
    )
    .await
    .map_err(map_parameter_error)?;
    if mapped.status == ParameterMappingStatus::Matched {
        let value = mapped.target_value.ok_or_else(|| {
            H8InboundError::Unprocessable(format!("{dict_code} mapping has no target"))
        })?;
        return Ok(MappedProductChangeValue {
            value,
            rule_id: mapped.rule_id,
        });
    }
    if required {
        return Err(H8InboundError::Unprocessable(format!(
            "{dict_code} mapping is unresolved"
        )));
    }
    Ok(MappedProductChangeValue {
        value: source_value.to_string(),
        rule_id: mapped.rule_id,
    })
}

fn normalized_new_value(
    body: &H8ProductChangeInboundRequest,
    product: &Product,
) -> Result<String, H8InboundError> {
    let value = match body.field_name.trim() {
        "approval_number" | "approval_no" => product.approval_no.as_deref(),
        "specification" | "spec" => Some(product.spec.as_str()),
        "dosage_form" => product.dosage_form.as_deref(),
        "manufacturer" => product.manufacturer.as_deref(),
        "udi_code" => product.udi_code.as_deref(),
        "electronic_regulatory_code" => product.electronic_regulatory_code.as_deref(),
        "product_name" => Some(product.product_name.as_str()),
        "status" => Some(product.status.as_str()),
        "special_drug_category" | "special_drug_category_code" => {
            product.special_drug_category_code.as_deref()
        }
        "storage_condition" => product
            .attrs
            .get("storage_condition")
            .and_then(serde_json::Value::as_str),
        "physical_dimensions" => {
            return serde_json::to_string(&json!({
                "length_mm": product.length_mm,
                "width_mm": product.width_mm,
                "height_mm": product.height_mm,
            }))
            .map_err(|error| H8InboundError::Internal(error.to_string()))
        }
        "volume_cm3" => return numeric_new_value(product.volume_cm3),
        "weight_g" => return numeric_new_value(product.weight_g),
        "packaging_levels" => {
            return serde_json::to_string(&product.packaging_levels)
                .map_err(|error| H8InboundError::Internal(error.to_string()))
        }
        _ => None,
    };
    value.map(str::to_string).ok_or_else(|| {
        H8InboundError::Unprocessable("updated product field is unavailable".to_string())
    })
}

fn numeric_new_value(value: Option<f64>) -> Result<String, H8InboundError> {
    value.map(|number| number.to_string()).ok_or_else(|| {
        H8InboundError::Unprocessable("updated numeric field is unavailable".to_string())
    })
}

fn map_quality_liaison_error(error: QualityLiaisonError) -> H8InboundError {
    match error {
        QualityLiaisonError::IdempotencyConflict => {
            H8InboundError::Conflict("quality liaison idempotency conflict")
        }
        QualityLiaisonError::NotFound
        | QualityLiaisonError::TypeConfigNotFound
        | QualityLiaisonError::TypeNotFound
        | QualityLiaisonError::InvalidRequest
        | QualityLiaisonError::ApprovalOpinionRequired
        | QualityLiaisonError::UnauthorizedApprover
        | QualityLiaisonError::AlreadyClosed
        | QualityLiaisonError::BusinessActionInvalid => {
            H8InboundError::Unprocessable("archive revision closeout is invalid".to_string())
        }
        QualityLiaisonError::BusinessAction(message)
        | QualityLiaisonError::DocumentNumbering(message)
        | QualityLiaisonError::Audit(message)
        | QualityLiaisonError::Database(message)
        | QualityLiaisonError::Serialize(message) => H8InboundError::Internal(message),
    }
}
