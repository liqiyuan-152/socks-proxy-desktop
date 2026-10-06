use super::*;
use crate::models::{ProxyProfile, ProxyProtocol, RoutingRule, RuleAction, RuleMatcher};
use std::collections::HashMap;
use std::io::Read;
use std::{
    net::{TcpListener, UdpSocket},
    thread,
};

#[test]
fn fixed_core_exposes_verified_active_connection_fields_on_loopback() {
    let Some(binary) = crate::test_core::binary() else {
        return;
    };
    let binary = Path::new(&binary);
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(binary).unwrap(), &mut hasher).unwrap();
    let checksum = hex::encode(hasher.finalize());

    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let upstream_port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = [0u8; 512];
        let count = stream.read(&mut request).unwrap();
        assert!(String::from_utf8_lossy(&request[..count]).contains("CONNECT example.com:443"));
        stream
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .unwrap();
        let _ = stream.read(&mut request);
    });
    let mut config = PersistedConfiguration::default();
    config.profiles.push(ProxyProfile {
        id: "upstream".into(),
        name: "Upstream".into(),
        protocol: ProxyProtocol::Http,
        host: "127.0.0.1".into(),
        port: upstream_port,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    config.active_profile_id = Some("upstream".into());
    config.rules.push(RoutingRule {
        id: "domain-rule".into(),
        name: "Domain".into(),
        matcher: RuleMatcher::Domain,
        target: "example.com".into(),
        port_start: Some(443),
        port_end: None,
        action: RuleAction::Proxy,
        proxy_profile_id: Some("upstream".into()),
        enabled: true,
    });
    let directory = tempfile::tempdir().unwrap();
    let mut process = SingBoxProcess::start(
        binary,
        &checksum,
        directory.path(),
        &config,
        crate::models::TEST_RULES_MODE,
        &HashMap::new(),
    )
    .unwrap();
    let mut client = TcpStream::connect(("127.0.0.1", process.proxy_port)).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    client
        .write_all(b"CONNECT example.com:443 HTTP/1.1\r\nHost: example.com:443\r\n\r\n")
        .unwrap();
    let mut response = [0u8; 512];
    let count = client.read(&mut response).unwrap();
    assert!(String::from_utf8_lossy(&response[..count]).contains("200"));

    let deadline = Instant::now() + Duration::from_secs(3);
    let connection = loop {
        let response = process.connections_json().unwrap();
        if let Some(item) = response["connections"]
            .as_array()
            .and_then(|items| items.first())
        {
            break item.clone();
        }
        assert!(Instant::now() < deadline, "活跃连接未出现在内核接口");
        thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(connection["metadata"]["host"], "example.com");
    assert_eq!(connection["metadata"]["destinationPort"], "443");
    assert_eq!(
        connection["rule"],
        format!(
            "domain=example.com port=443 => route({})",
            crate::sing_box_config::proxy_tag("upstream")
        )
    );
    assert!(connection["chains"].as_array().is_some_and(|chain| chain
        .iter()
        .any(|tag| tag == &crate::sing_box_config::proxy_tag("upstream"))));
    let observed = crate::observability::parse_connections(
        &serde_json::json!({ "connections": [connection] }),
    )
    .unwrap();
    assert_eq!(observed.active_count, Some(1));
    assert_eq!(observed.recent[0].target_host, "example.com");
    assert_eq!(observed.recent[0].target_port, 443);
    assert!(observed.recent[0]
        .outbound_chain
        .iter()
        .any(|tag| tag == &crate::sing_box_config::proxy_tag("upstream")));

    // The API is bound to 127.0.0.1, not the machine's non-loopback interface.
    let socket = UdpSocket::bind("0.0.0.0:0").unwrap();
    socket.connect("192.0.2.1:9").unwrap();
    let local = socket.local_addr().unwrap();
    assert!(!local.ip().is_loopback() && !local.ip().is_unspecified());
    assert!(TcpStream::connect_timeout(
        &(local.ip(), process.control_port).into(),
        Duration::from_millis(250),
    )
    .is_err());
    drop(client);
    drop(process);
    server.join().unwrap();
}

// Prediction verifies the pinned Windows executable; this capability is Windows-only.
#[cfg(windows)]
#[test]
fn china_preset_routes_unlisted_domains_to_proxy_and_literal_private_ip_direct() {
    let Some(binary) = crate::test_core::binary() else {
        return;
    };
    let binary = Path::new(&binary);
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(binary).unwrap(), &mut hasher).unwrap();
    let checksum = hex::encode(hasher.finalize());
    let upstream = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let upstream_port = upstream.local_addr().unwrap().port();
    let mut config = PersistedConfiguration::default();
    config.profiles.push(ProxyProfile {
        id: "upstream".into(),
        name: "Upstream".into(),
        protocol: ProxyProtocol::Http,
        host: "127.0.0.1".into(),
        port: upstream_port,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    config.active_profile_id = Some("upstream".into());
    config.runtime_mode = crate::models::RuntimeMode::Rules {
        use_china_direct: true,
        default_action: crate::models::RuleAction::Proxy,
    };
    let rule_root = crate::china_rules::resource_root(binary).unwrap();
    for (target, action) in [
        ("unknown.invalid", RuleAction::Proxy),
        ("192.0.2.1", RuleAction::Proxy),
        ("127.0.0.1", RuleAction::Direct),
    ] {
        let predicted =
            crate::route_test::evaluate(&config, Some(&rule_root), target, 443).unwrap();
        assert_eq!(predicted.action, action, "{target}");
    }
    let (received_tx, received_rx) = std::sync::mpsc::channel();
    let observed = thread::spawn(move || {
        upstream.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut targets = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = loop {
                match upstream.accept() {
                    Ok(connection) => break connection,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(20))
                    }
                    Err(error) => panic!("upstream did not receive both requests: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
                assert!(request.len() <= 8192, "oversized CONNECT header");
            }
            targets.push(String::from_utf8_lossy(&request).into_owned());
            stream
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
            received_tx.send(()).unwrap();
        }
        targets
    });
    let destination = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let destination_port = destination.local_addr().unwrap().port();
    let direct = thread::spawn(move || {
        destination.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        let (mut stream, _) = loop {
            match destination.accept() {
                Ok(connection) => break connection,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(20))
                }
                Err(error) => panic!("direct target was not reached: {error}"),
            }
        };
        stream.write_all(b"direct").unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let process = SingBoxProcess::start(
        binary,
        &checksum,
        directory.path(),
        &config,
        crate::models::TEST_RULES_MODE,
        &HashMap::new(),
    )
    .unwrap();
    let connect = |target: &str| {
        let mut client = TcpStream::connect(("127.0.0.1", process.proxy_port)).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        client
            .write_all(format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n\r\n").as_bytes())
            .unwrap();
        let mut response = [0u8; 1024];
        let count = client.read(&mut response).unwrap();
        assert!(String::from_utf8_lossy(&response[..count]).contains("200"));
        client
    };
    // Local CONNECT acknowledgement precedes upstream dialing. Keep each
    // client alive until the upstream receives the complete request.
    let first = connect("unknown.invalid:443");
    received_rx.recv_timeout(Duration::from_secs(8)).unwrap();
    drop(first);
    let second = connect("192.0.2.1:443");
    received_rx.recv_timeout(Duration::from_secs(8)).unwrap();
    drop(second);
    let mut client = connect(&format!("127.0.0.1:{destination_port}"));
    let mut payload = [0u8; 64];
    let count = client.read(&mut payload).unwrap();
    assert_eq!(&payload[..count], b"direct");
    let requests = observed.join().unwrap();
    // The inbound acknowledges CONNECT before upstream dialing completes;
    // upstream arrival order is not the order of local acknowledgements.
    assert!(requests
        .iter()
        .any(|request| request.contains("CONNECT unknown.invalid:443")));
    assert!(requests
        .iter()
        .any(|request| request.contains("CONNECT 192.0.2.1:443")));
    direct.join().unwrap();
}

// Prediction verifies the pinned Windows executable; this capability is Windows-only.
#[cfg(windows)]
#[path = "sing_box_china_retry_tests.rs"]
mod china_retry_tests;
