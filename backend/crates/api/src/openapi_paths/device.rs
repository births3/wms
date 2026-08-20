#[allow(unused_imports)]
use super::*;

#[allow(unused_imports)]
use crate::device_service::{
    BindDeviceRequest, DeviceBindingResponse, DeviceResponse, RegisterDeviceRequest, UnbindRequest,
    UpdateDeviceRequest,
};

#[utoipa::path(
    post,
    path = "/api/v1/iot-devices",
    tag = "device",
    params(("Idempotency-Key" = String, Header, description = "注册幂等键")),
    request_body = RegisterDeviceRequest,
    responses(
        (status = 201, description = "设备注册成功", body = DeviceResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 409, description = "设备编码或幂等键冲突", body = ErrorResponse),
        (status = 422, description = "设备参数非法", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn register_iot_device() {}

#[utoipa::path(
    get,
    path = "/api/v1/iot-devices",
    tag = "device",
    params(
        ("warehouse_id" = Uuid, Query, description = "仓库 ID"),
        ("device_type" = Option<String>, Query, description = "设备类型"),
        ("online_status" = Option<String>, Query, description = "在线状态"),
        ("enabled" = Option<bool>, Query, description = "启停"),
    ),
    responses(
        (status = 200, description = "设备列表", body = Vec<DeviceResponse>),
        (status = 400, description = "查询参数非法", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "仓库范围不足", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn list_iot_devices() {}

#[utoipa::path(
    get,
    path = "/api/v1/iot-devices/{id}",
    tag = "device",
    params(("id" = Uuid, Path, description = "设备 ID")),
    responses(
        (status = 200, description = "设备详情", body = DeviceResponse),
        (status = 400, description = "设备 ID 非法", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "仓库范围不足", body = ErrorResponse),
        (status = 404, description = "设备不存在", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn get_iot_device() {}

#[utoipa::path(
    patch,
    path = "/api/v1/iot-devices/{id}",
    tag = "device",
    params(
        ("id" = Uuid, Path, description = "设备 ID"),
        ("Idempotency-Key" = String, Header, description = "设备更新幂等键"),
    ),
    request_body = UpdateDeviceRequest,
    responses(
        (status = 200, description = "设备维护/启停", body = DeviceResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "设备不存在", body = ErrorResponse),
        (status = 409, description = "版本或幂等冲突", body = ErrorResponse),
        (status = 422, description = "设备参数非法", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn update_iot_device() {}

#[utoipa::path(
    post,
    path = "/api/v1/iot-devices/{id}/heartbeat",
    tag = "device",
    params(
        ("id" = Uuid, Path, description = "设备 ID"),
        ("Idempotency-Key" = String, Header, description = "心跳幂等键"),
    ),
    responses(
        (status = 200, description = "心跳上报", body = DeviceResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "设备不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 422, description = "设备已停用", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn heartbeat_iot_device() {}

#[utoipa::path(
    post,
    path = "/api/v1/location-device-bindings",
    tag = "device",
    params(("Idempotency-Key" = String, Header, description = "绑定幂等键")),
    request_body = BindDeviceRequest,
    responses(
        (status = 201, description = "绑定成功", body = DeviceBindingResponse),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "设备或库位不存在", body = ErrorResponse),
        (status = 409, description = "绑定或幂等键冲突", body = ErrorResponse),
        (status = 422, description = "设备、角色或库位不匹配", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn bind_location_device() {}

#[utoipa::path(
    post,
    path = "/api/v1/location-device-bindings/{id}/unbind",
    tag = "device",
    params(
        ("id" = Uuid, Path, description = "绑定 ID"),
        ("Idempotency-Key" = String, Header, description = "解绑幂等键"),
    ),
    request_body = UnbindRequest,
    responses(
        (status = 204, description = "软解绑成功"),
        (status = 400, description = "缺少幂等键", body = ErrorResponse),
        (status = 401, description = "未登录", body = ErrorResponse),
        (status = 403, description = "权限或仓库范围不足", body = ErrorResponse),
        (status = 404, description = "绑定不存在", body = ErrorResponse),
        (status = 409, description = "幂等键冲突", body = ErrorResponse),
        (status = 500, description = "内部错误", body = ErrorResponse),
    ),
)]
#[allow(dead_code)]
pub(crate) fn unbind_location_device() {}
