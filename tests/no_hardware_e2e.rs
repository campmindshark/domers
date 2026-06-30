//! No-hardware integration smoke for migration and simulator runtime behavior.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
};

use domers_core::{analyze_spectrum_xml, WarningKind};
use domers_outputs::DomeCommand;
use domers_server::{DomeConfigPatch, ServerState};
use tokio::net::TcpListener;

#[test]
fn no_hardware_server_migration_and_simulator_smoke() {
    let xml = include_str!("../fixtures/config/spectrum_default_config.xml");
    let report = analyze_spectrum_xml(xml);
    assert!(report.contains(WarningKind::StaleField, "kickT"));
    assert!(report.contains(WarningKind::InvalidMidiBindingTarget, "snareT"));

    let mut server = ServerState::default();
    server.start();
    server.patch_dome_config(DomeConfigPatch {
        active_visualizer: Some(1),
        flash_speed: None,
        color_palette_index: None,
    });

    for _ in 0..60 {
        let frame = server.simulator_frame();
        assert!(frame
            .dome
            .iter()
            .any(|command| matches!(command, DomeCommand::Flush)));
    }

    assert_eq!(server.metrics().frames, 60);
    assert_eq!(server.metrics().simulator_frames, 60);
}

#[tokio::test(flavor = "multi_thread")]
async fn browser_shell_http_flows_work_without_hardware() {
    let config = domers_core::DomersConfig::from_toml_str(include_str!("../examples/domers.toml"))
        .expect("checked example config parses");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("test listener binds");
    let addr = listener.local_addr().expect("listener has address");
    let server = tokio::spawn(async move { domers_server::serve_listener(listener, config).await });

    let index = http_request(addr, "GET", "/", None);
    assert!(index.contains("200 OK"));
    assert!(index.contains("MindShark Dome Control Panel"));
    assert!(index.contains("/assets/main.js"));

    let simulator = http_request(addr, "GET", "/simulator", None);
    assert!(simulator.contains("200 OK"));
    assert!(simulator.contains("MindShark Dome Simulator"));

    let start = http_request(addr, "POST", "/api/start", Some("{}"));
    assert!(start.contains("\"running\":true"));

    let tap = http_request(addr, "POST", "/api/input/tap", Some("{}"));
    assert!(tap.contains("\"taps\":1"));

    let dome_patch = http_request(
        addr,
        "PATCH",
        "/api/config/dome",
        Some(r#"{"active_visualizer":1,"color_palette_index":2}"#),
    );
    assert!(dome_patch.contains("\"dome_active_vis\":1"));
    assert!(dome_patch.contains("\"color_palette_index\":2"));

    let palette_patch = http_request(
        addr,
        "PATCH",
        "/api/config/palette",
        Some(r#"{"relative_index":0,"color1":1122867,"color2":4478310,"color2_enabled":true}"#),
    );
    assert!(palette_patch.contains("\"color_palette_index\":2"));

    let sandbox_frame = http_request(
        addr,
        "POST",
        "/api/simulator/sandbox-frame",
        Some(r#"{"active_visualizer":1,"volume":0.8,"beat_progress":0.25,"flash_active":true}"#),
    );
    assert!(sandbox_frame.contains("\"commands\""));
    assert!(sandbox_frame.contains("\"bar_commands\""));
    assert!(sandbox_frame.contains("\"stage_commands\""));

    let stop = http_request(addr, "POST", "/api/stop", Some("{}"));
    assert!(stop.contains("\"running\":false"));

    server.abort();
}

fn http_request(addr: SocketAddr, method: &str, path: &str, body: Option<&str>) -> String {
    let body = body.unwrap_or("");
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = TcpStream::connect(addr).expect("test server accepts connections");
    stream
        .write_all(request.as_bytes())
        .expect("request writes");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("response reads");
    response
}
