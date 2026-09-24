use super::*;
use crate::models::{ProxyProfile, ProxyProtocol, RoutingRule, RuleAction, RuleMatcher};
use std::collections::HashMap;
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
        proxy_profile_id: Some("upstream".into()),
        enabled: true,
    });
    let directory = tempfile::tempdir().unwrap();
    let mut process = SingBoxProcess::start(
        binary,
        &checksum,
        directory.path(),
        &config,
        RuntimeMode::Rules,
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

#[test]
fn china_preset_routes_unlisted_domains_to_proxy_and_literal_private_ip_direct() {
    let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") else {
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
    config.china_direct_enabled = true;
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
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0u8; 1024];
            let count = stream.read(&mut request).unwrap();
            targets.push(String::from_utf8_lossy(&request[..count]).into_owned());
            stream
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .unwrap();
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
        RuntimeMode::Rules,
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
    drop(connect("unknown.invalid:443"));
    drop(connect("192.0.2.1:443"));
    let mut client = connect(&format!("127.0.0.1:{destination_port}"));
    let mut payload = [0u8; 64];
    let count = client.read(&mut payload).unwrap();
    assert_eq!(&payload[..count], b"direct");
    let requests = observed.join().unwrap();
    assert!(requests[0].contains("CONNECT unknown.invalid:443"));
    assert!(requests[1].contains("CONNECT 192.0.2.1:443"));
    direct.join().unwrap();
}

#[test]
fn china_preset_keeps_domain_exit_on_retry_and_routes_literal_ipv6() {
    let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") else {
        return;
    };
    let binary = Path::new(&binary);
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(binary).unwrap(), &mut hasher).unwrap();
    let checksum = hex::encode(hasher.finalize());
    let Ok(ipv6) = TcpListener::bind(("::1", 0)) else {
        return;
    };
    let direct_port = ipv6.local_addr().unwrap().port();
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
    config.china_direct_enabled = true;
    let rule_root = crate::china_rules::resource_root(binary).unwrap();
    for (target, action) in [
        ("mixed.invalid", RuleAction::Proxy),
        ("unresolvable.invalid", RuleAction::Proxy),
        ("1.0.1.1", RuleAction::Direct),
        ("2001:db8::1", RuleAction::Proxy),
        ("::1", RuleAction::Direct),
    ] {
        assert_eq!(
            crate::route_test::evaluate(&config, Some(&rule_root), target, 443)
                .unwrap()
                .action,
            action,
            "{target}"
        );
    }
    let observed = thread::spawn(move || {
        upstream.set_nonblocking(true).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut requests = Vec::new();
        let candidates = [
            "1.0.1.1".parse::<std::net::IpAddr>().unwrap(),
            "2001:db8::1".parse::<std::net::IpAddr>().unwrap(),
        ];
        for index in 0..4 {
            let (mut stream, _) = loop {
                match upstream.accept() {
                    Ok(connection) => break connection,
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(20));
                    }
                    Err(error) => panic!("expected proxy request {index}: {error}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0u8; 1024];
            let count = stream.read(&mut request).unwrap();
            let candidate = (index < 2).then_some(candidates[index.min(1)]);
            requests.push((
                String::from_utf8_lossy(&request[..count]).into_owned(),
                candidate,
            ));
            // The upstream simulates a mixed A/AAAA answer and retries with
            // the other family. The core must forward the original domain.
            let response = if index == 0 || index == 2 {
                b"HTTP/1.1 502 Bad Gateway\r\n\r\n".as_slice()
            } else {
                b"HTTP/1.1 200 Connection Established\r\n\r\n".as_slice()
            };
            stream.write_all(response).unwrap();
        }
        requests
    });
    let direct = thread::spawn(move || {
        let (mut stream, _) = ipv6.accept().unwrap();
        stream.write_all(b"ipv6-direct").unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let process = SingBoxProcess::start(
        binary,
        &checksum,
        directory.path(),
        &config,
        RuntimeMode::Rules,
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
        (
            client,
            String::from_utf8_lossy(&response[..count]).into_owned(),
        )
    };
    // CONNECT may be acknowledged by the local inbound before an upstream
    // failure; the observed upstream request is the routing assertion.
    let _ = connect("mixed.invalid:443");
    assert!(connect("mixed.invalid:443").1.contains("200"));
    let _ = connect("unresolvable.invalid:443");
    assert!(connect("[2001:db8::1]:443").1.contains("200"));
    let (mut client, response) = connect(&format!("[::1]:{direct_port}"));
    assert!(response.contains("200"));
    let mut payload = [0u8; 64];
    let count = client.read(&mut payload).unwrap();
    assert_eq!(&payload[..count], b"ipv6-direct");
    let requests = observed.join().unwrap();
    assert!(requests[0].0.contains("CONNECT mixed.invalid:443"));
    assert!(requests[0].1.unwrap().is_ipv4());
    assert!(requests[1].0.contains("CONNECT mixed.invalid:443"));
    assert!(requests[1].1.unwrap().is_ipv6());
    assert!(requests[2].0.contains("CONNECT unresolvable.invalid:443"));
    assert!(requests[3].0.contains("CONNECT [2001:db8::1]:443"));
    direct.join().unwrap();
}
