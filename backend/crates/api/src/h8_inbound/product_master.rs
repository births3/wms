use axum::{extract::State, http::HeaderMap, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use wms_domain::{
    CreateProductRequest, MapParameterRequest, ParameterMappingStatus, ProductMappingTraceInput,
    ProductPackagingLevelInput,
};

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
pub struct H8ProductMasterInboundRequest {
    pub schema_version: String,
    pub external_ref: String,
    pub correlation_id: String,
    pub occurred_at: DateTime<Utc>,
    pub product_code: String,
    pub product_name: String,
    pub approval_no: Option<String>,
    pub spec: String,
    pub dosage_form: Option<String>,
    pub manufacturer: Option<String>,
    pub special_drug_category: String,
    pub udi_code: Option<String>,
    pub electronic_regulatory_code: Option<String>,
    pub length_mm: Option<f64>,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub volume_cm3: Option<f64>,
    pub weight_g: Option<f64>,
    pub packaging_levels: Vec<H8ProductPackagingLevelInput>,
    pub storage_condition: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct H8ProductPackagingLevelInput {
    /// ERP 包装单位源值，进入 M1 前必须经 M-PM `unit_pack` 规整。
    pub unit: String,
    pub ratio_to_base: i64,
    pub is_base: bool,
    pub is_default: bool,
    pub sort_order: i32,
}

pub(super) async fn push_product_master(
    ctx: AuthContext,
    State(state): State<H8InboundAppState>,
    headers: HeaderMap,
    Json(body): Json<H8ProductMasterInboundRequest>,
) -> Result<Json<H8InboundResponse>, H8InboundError> {
    ctx.require_permission("m1.master_data.write")?;
    validate_request(&ctx, &body)?;
    let idempotency_key = idempotency_key(&headers)?;
    let mut prepared = prepare_message(
        &state,
        &ctx,
        idempotency_key,
        InboundMetadata {
            message_type: "product_master",
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
    let (request, mapping_traces, pending_mapping) = match product_request(
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
    let outcome = match state
        .master_data
        .create_product_with_mapping_traces_status(
            &ctx,
            request,
            mapping_traces,
            if pending_mapping {
                "pending_mapping"
            } else {
                "active"
            },
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
    let replayed = prepared.replayed;
    prepared.message =
        succeed_message(&state, &ctx, prepared.message.id, &outcome.id.to_string()).await?;
    Ok(Json(response(prepared.message, replayed)?))
}

fn validate_request(
    ctx: &AuthContext,
    body: &H8ProductMasterInboundRequest,
) -> Result<(), H8InboundError> {
    validate_envelope(
        ctx,
        &body.schema_version,
        &body.external_ref,
        &body.correlation_id,
        None,
    )?;
    if body.product_code.trim().is_empty()
        || body.product_name.trim().is_empty()
        || body.spec.trim().is_empty()
        || body.storage_condition.trim().is_empty()
        || body.special_drug_category.trim().is_empty()
        || body.packaging_levels.is_empty()
        || body
            .packaging_levels
            .iter()
            .any(|level| level.unit.trim().is_empty())
    {
        return Err(H8InboundError::Unprocessable(
            "required product master field is invalid".to_string(),
        ));
    }
    Ok(())
}

async fn product_request(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    body: &H8ProductMasterInboundRequest,
    connector_code: &str,
    idempotency_key: &str,
) -> Result<(CreateProductRequest, Vec<ProductMappingTraceInput>, bool), H8InboundError> {
    let specification = body.spec.trim().to_string();
    let storage_condition = map_value(
        state,
        ctx,
        "storage_condition",
        &body.storage_condition,
        body,
        connector_code,
        idempotency_key,
        "storage_condition",
        true,
    )
    .await?;
    let dosage_form = match body.dosage_form.as_deref().map(str::trim) {
        Some(value) if !value.is_empty() => Some(
            map_value(
                state,
                ctx,
                "dosage_form",
                value,
                body,
                connector_code,
                idempotency_key,
                "dosage_form",
                false,
            )
            .await?,
        ),
        _ => None,
    };
    let special_drug_category = map_value(
        state,
        ctx,
        "special_drug_category",
        &body.special_drug_category,
        body,
        connector_code,
        idempotency_key,
        "special_drug_category",
        true,
    )
    .await?;
    let mut mapping_traces = vec![
        storage_condition.trace.clone(),
        special_drug_category.trace.clone(),
    ];
    let mut pending_mapping = storage_condition.unresolved || special_drug_category.unresolved;
    if let Some(mapped) = &dosage_form {
        mapping_traces.push(mapped.trace.clone());
    }
    let mut packaging_levels = Vec::with_capacity(body.packaging_levels.len());
    for (index, level) in body.packaging_levels.iter().enumerate() {
        let mapped = map_value(
            state,
            ctx,
            "unit_pack",
            &level.unit,
            body,
            connector_code,
            idempotency_key,
            &format!("unit_pack:{index}"),
            true,
        )
        .await?;
        pending_mapping |= mapped.unresolved;
        let unit_code = mapped.value.clone();
        let mut trace = mapped.trace;
        trace.field_name = format!("packaging_levels[{index}].unit_code");
        mapping_traces.push(trace);
        if !mapped.unresolved {
            packaging_levels.push(ProductPackagingLevelInput {
                unit_name: unit_name(&unit_code).to_string(),
                unit_code,
                ratio_to_base: level.ratio_to_base,
                is_base: level.is_base,
                is_default: level.is_default,
                sort_order: level.sort_order,
            });
        }
    }
    if pending_mapping {
        packaging_levels.clear();
    }
    Ok((
        CreateProductRequest {
            product_code: body.product_code.trim().to_string(),
            product_name: body.product_name.trim().to_string(),
            approval_no: body.approval_no.clone(),
            spec: specification,
            dosage_form: dosage_form.map(|mapped| mapped.value),
            manufacturer: body.manufacturer.clone(),
            special_drug_category_code: (!special_drug_category.value.is_empty())
                .then_some(special_drug_category.value),
            udi_code: body.udi_code.clone(),
            electronic_regulatory_code: body.electronic_regulatory_code.clone(),
            length_mm: body.length_mm,
            width_mm: body.width_mm,
            height_mm: body.height_mm,
            volume_cm3: body.volume_cm3,
            weight_g: body.weight_g,
            packaging_levels,
            attrs: json!({
                "storage_condition": (!storage_condition.value.is_empty())
                    .then_some(storage_condition.value),
                "source": "erp_rest"
            }),
        },
        mapping_traces,
        pending_mapping,
    ))
}

#[derive(Clone)]
struct MappedProductValue {
    value: String,
    trace: ProductMappingTraceInput,
    unresolved: bool,
}

async fn map_value(
    state: &H8InboundAppState,
    ctx: &AuthContext,
    dict_code: &str,
    source_value: &str,
    body: &H8ProductMasterInboundRequest,
    connector_code: &str,
    idempotency_key: &str,
    mapping_key: &str,
    required: bool,
) -> Result<MappedProductValue, H8InboundError> {
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
    let unresolved = mapped.status != ParameterMappingStatus::Matched;
    let target_value = if !unresolved {
        Some(mapped.target_value.clone().ok_or_else(|| {
            H8InboundError::Unprocessable(format!("{dict_code} mapping has no target"))
        })?)
    } else if required {
        None
    } else {
        Some(source_value.to_string())
    };
    Ok(MappedProductValue {
        value: target_value.clone().unwrap_or_default(),
        trace: ProductMappingTraceInput {
            field_name: dict_code.to_string(),
            rule_id: mapped.rule_id,
            source_system: connector_code.to_string(),
            source_value: source_value.to_string(),
            target_value,
        },
        unresolved,
    })
}

pub(super) fn unit_name(unit_code: &str) -> &str {
    match unit_code {
        "piece" => "支",
        "tablet" => "片",
        "board" => "板",
        "bottle" => "瓶",
        "bag" => "袋",
        "box" => "盒",
        "case" => "件",
        "pallet" => "托",
        _ => unit_code,
    }
}
