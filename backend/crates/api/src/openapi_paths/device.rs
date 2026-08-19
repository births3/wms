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
    request_body = RegisterDeviceRequest,
    responses(
        (status = 200, description = "设备注册成功", body = DeviceResponse),
        (status = 409, description = "设备编码重复"),
        (status = 422, description = "设备类型非法"),
    ),
)]
#[allow(dead_code)]
pub(crate) fn register_iot_device() {}

#[utoipa::path(
    get,
    path = "/api/v1/iot-devices",
    tag = "device",
    params(
        ("device_type" = Option<String>, Query, description = "设备类型"),
        ("online_status" = Option<String>, Query, description = "在线状态"),
        ("enabled" = Option<bool>, Query, description = "启停"),
    ),
    responses((status = 200, description = "设备列表", body = Vec<DeviceResponse>)),
)]
#[allow(dead_code)]
pub(crate) fn list_iot_devices() {}

#[utoipa::path(
    get,
    path = "/api/v1/iot-devices/{id}",
    tag = "device",
    params(("id" = Uuid, Path, description = "设备 ID")),
    responses((status = 200, description = "设备详情", body = DeviceResponse)),
)]
#[allow(dead_code)]
pub(crate) fn get_iot_device() {}

#[utoipa::path(
    patch,
    path = "/api/v1/iot-devices/{id}",
    tag = "device",
    params(("id" = Uuid, Path, description = "设备 ID")),
    request_body = UpdateDeviceRequest,
    responses((status = 200, description = "设备维护/启停", body = DeviceResponse)),
)]
#[allow(dead_code)]
pub(crate) fn update_iot_device() {}

#[utoipa::path(
    post,
    path = "/api/v1/iot-devices/{id}/heartbeat",
    tag = "device",
    params(("id" = Uuid, Path, description = "设备 ID")),
    responses((status = 200, description = "心跳上报", body = DeviceResponse)),
)]
#[allow(dead_code)]
pub(crate) fn heartbeat_iot_device() {}

#[utoipa::path(
    post,
    path = "/api/v1/location-device-bindings",
    tag = "device",
    request_body = BindDeviceRequest,
    responses(
        (status = 200, description = "绑定成功", body = DeviceBindingResponse),
        (status = 409, description = "绑定冲突"),
    ),
)]
#[allow(dead_code)]
pub(crate) fn bind_location_device() {}

#[utoipa::path(
    post,
    path = "/api/v1/location-device-bindings/{id}/unbind",
    tag = "device",
    params(("id" = Uuid, Path, description = "绑定 ID")),
    request_body = UnbindRequest,
    responses((status = 204, description = "软解绑成功")),
)]
#[allow(dead_code)]
pub(crate) fn unbind_location_device() {}
