use super::*;

#[test]
fn china_preset_keeps_domain_exit_on_retry_and_routes_literal_ipv6() {
    let Some(binary) = crate::test_core::binary() else {
        return;
    };
    let binary = Path::new(&binary);
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(binary).unwrap(), &mut hasher).unwrap();
    let checksum = hex::encode(hasher.finalize());
    let ipv6 = TcpListener::bind(("::1", 0))
        .expect("Windows real-core IPv6 scenario requires an IPv6 loopback listener");
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
    config.runtime_mode = crate::models::RuntimeMode::Rules {
        use_china_direct: true,
        default_action: crate::models::RuleAction::Proxy,
    };
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
    let (received_tx, received_rx) = std::sync::mpsc::channel();
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
            let candidate = (index < 2).then_some(candidates[index.min(1)]);
            requests.push((String::from_utf8_lossy(&request).into_owned(), candidate));
            // The upstream simulates a mixed A/AAAA answer and retries with
            // the other family. The core must forward the original domain.
            let response = if index == 0 || index == 2 {
                b"HTTP/1.1 502 Bad Gateway\r\n\r\n".as_slice()
            } else {
                b"HTTP/1.1 200 Connection Established\r\n\r\n".as_slice()
            };
            stream.write_all(response).unwrap();
            received_tx.send(()).unwrap();
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
        config.runtime_mode,
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
        // Local inbound acknowledgement does not mean upstream has received
        // the request. Keep the client alive and wait before sending the next.
        if !target.starts_with("[::1]") {
            received_rx.recv_timeout(Duration::from_secs(10)).unwrap();
        }
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
