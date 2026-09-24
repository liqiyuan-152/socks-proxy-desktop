use super::*;
use crate::models::{ProxyProfile, ProxyProtocol, RoutingRule, RuleAction, RuleMatcher};
use std::{
    net::{TcpListener, UdpSocket},
    thread,
};

#[test]
fn fixed_core_exposes_verified_active_connection_fields_on_loopback() {
    let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") else {
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
        enabled: true,
    });
    let directory = tempfile::tempdir().unwrap();
    let mut process = SingBoxProcess::start(
        binary,
        &checksum,
        directory.path(),
        &config,
        RuntimeMode::Rules,
        None,
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
        "domain=example.com port=443 => route(selected-proxy)"
    );
    assert!(connection["chains"]
        .as_array()
        .is_some_and(|chain| chain.iter().any(|tag| tag == "selected-proxy")));
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
        .any(|tag| tag == "selected-proxy"));

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
