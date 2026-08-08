use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
};

use h8_erp_worker::{config::BootstrapSettings, control_plane::ControlPlaneClient};
use serde_json::{json, Value};

fn spawn_control_plane() -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("测试控制面应能监听随机端口");
    let address = listener.local_addr().expect("测试控制面应能读取监听地址");
    let (sender, receiver) = mpsc::channel();
    let handle = thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().expect("应收到 Worker 请求");
            let mut raw = Vec::new();
            let mut buffer = [0_u8; 4096];
            loop {
                let count = stream.read(&mut buffer).expect("应能读取 HTTP 请求");
                if count == 0 {
                    break;
                }
                raw.extend_from_slice(&buffer[..count]);
                if request_complete(&raw) {
                    break;
                }
            }
            let request = String::from_utf8(raw).expect("测试请求应为 UTF-8");
            sender.send(request.clone()).expect("应能保存请求证据");
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .expect("请求行必须包含 path");
            let body = match path {
                "/api/v1/config/erp-connectors/334c3ff7-1018-40c6-b1f4-c19b2d2c88e5" => json!({
                    "id": "334c3ff7-1018-40c6-b1f4-c19b2d2c88e5",
                    "status": "active",
                    "config_version": 7
                }),
                "/api/v1/config/erp-connectors/334c3ff7-1018-40c6-b1f4-c19b2d2c88e5/versions/7" => {
                    json!({
                        "id": "334c3ff7-1018-40c6-b1f4-c19b2d2c88e5",
                        "owner_id": "00000000-0000-0000-0000-000000000001",
                        "config_version": 7,
                        "channel_mode": "interface_table",
                        "interface_db_host": "10.12.98.254",
                        "interface_db_port": 9631,
                        "interface_db_name": "zbpf7_test",
                        "interface_db_username": "wms_worker_test",
                        "interface_db_password_alias": "vault://h8/zbpf7-test"
                    })
                }
                "/api/v1/integration/erp-messages/worker-runtime/heartbeat" => json!({
                    "health": "healthy"
                }),
                unexpected => panic!("unexpected control-plane path: {unexpected}"),
            };
            write_json(&mut stream, &body);
        }
    });
    (format!("http://{address}"), receiver, handle)
}

#[tokio::test]
async fn bootstrap_snapshot_and_heartbeat_follow_control_plane_contract() {
    let (api_base, requests, server) = spawn_control_plane();
    let env = HashMap::from([
        (
            "H8_CONNECTOR_ID".to_owned(),
            "334c3ff7-1018-40c6-b1f4-c19b2d2c88e5".to_owned(),
        ),
        ("WMS_API_BASE".to_owned(), api_base),
        ("WMS_API_TOKEN".to_owned(), "test-token".to_owned()),
        ("H8_WORKER_ID".to_owned(), "rust-worker-1".to_owned()),
        ("H8_WORKER_VERSION".to_owned(), "rust-1".to_owned()),
    ]);
    let bootstrap = BootstrapSettings::from_map(&env).expect("启动配置应有效");
    let secrets = HashMap::from([(
        "vault://h8/zbpf7-test".to_owned(),
        "resolved-password".to_owned(),
    )]);
    let client = ControlPlaneClient::new(&bootstrap).expect("HTTP client 应创建成功");

    let runtime = client
        .load_runtime_settings(bootstrap, &secrets)
        .await
        .expect("应加载当前连接和冻结快照");
    client
        .post_heartbeat(&runtime, &["inbound", "outbound"], 2)
        .await
        .expect("应成功上报心跳");

    let captured = (0..3)
        .map(|_| requests.recv().expect("应捕获三次请求"))
        .collect::<Vec<_>>();
    server.join().expect("测试控制面应正常退出");
    assert!(captured.iter().all(|request| request
        .to_ascii_lowercase()
        .contains("authorization: bearer test-token")));
    let heartbeat = captured
        .iter()
        .find(|request| request.contains("worker-runtime/heartbeat"))
        .expect("应存在心跳请求");
    let body: Value = serde_json::from_str(
        heartbeat
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("心跳请求应有 body"),
    )
    .expect("心跳 body 应为 JSON");
    assert_eq!(body["worker_id"], "rust-worker-1");
    assert_eq!(body["worker_version"], "rust-1");
    assert_eq!(body["directions"], json!(["inbound", "outbound"]));
    assert_eq!(body["current_claims"], 2);
    assert_eq!(body["heartbeat_ttl_seconds"], 15);
}

fn request_complete(raw: &[u8]) -> bool {
    let text = String::from_utf8_lossy(raw);
    let Some((headers, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    body.len() >= content_length
}

fn write_json(stream: &mut std::net::TcpStream, body: &Value) {
    let body = body.to_string();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("应能写回测试响应");
}
