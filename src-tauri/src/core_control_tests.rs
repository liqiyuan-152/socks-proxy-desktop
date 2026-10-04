use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Instant,
};

fn server(
    reply: impl FnOnce(TcpStream) + Send + 'static,
) -> (CoreControlClient, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let client = CoreControlClient::new(
        listener.local_addr().unwrap().port(),
        "private-control-key".into(),
    )
    .unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        let request = String::from_utf8(request).unwrap().to_lowercase();
        assert!(request.contains("authorization: bearer private-control-key\r\n"));
        reply(stream);
    });
    (client, handle)
}

#[test]
fn decodes_chunked_responses_and_accepts_exact_limit() {
    let (client, handle) = server(|mut stream| {
        stream.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n9\r\n{\"value\":\r\n2\r\n1}\r\n0\r\n\r\n").unwrap();
    });
    assert_eq!(client.connections().unwrap()["value"], 1);
    handle.join().unwrap();
    let (client, handle) = server(|mut stream| {
        let body = format!("\"{}\"", "a".repeat(RESPONSE_LIMIT - 2));
        let header = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len());
        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(body.as_bytes()).unwrap();
    });
    assert_eq!(
        client.connections().unwrap().as_str().unwrap().len(),
        RESPONSE_LIMIT - 2
    );
    handle.join().unwrap();
}

#[test]
fn rejects_oversize_authentication_redirect_and_invalid_json_without_secret_details() {
    for (status, body, expected) in [
        (
            "200 OK",
            "a".repeat(RESPONSE_LIMIT + 1),
            "control_response_limit",
        ),
        (
            "401 Unauthorized",
            "private-control-key".into(),
            "control_authentication",
        ),
        ("302 Found", "".into(), "control_status"),
        (
            "200 OK",
            "private-control-key invalid json".into(),
            "control_json",
        ),
    ] {
        let (client, handle) = server(move |mut stream| {
            let header = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nLocation: http://127.0.0.1:1/\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body.as_bytes());
        });
        let error = client.connections().unwrap_err();
        assert_eq!(error.code, expected);
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains("private-control-key"));
        assert!(!error.message.contains("127.0.0.1"));
        handle.join().unwrap();
    }
}

#[test]
fn trickled_body_stops_at_total_deadline() {
    let (client, handle) = server(|mut stream| {
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n")
            .unwrap();
        for _ in 0..30 {
            if stream.write_all(b" ").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    let started = Instant::now();
    let error = client.connections().unwrap_err();
    assert_eq!(error.code, "control_timeout");
    assert!(started.elapsed() >= Duration::from_millis(1800));
    assert!(started.elapsed() < Duration::from_millis(2300));
    handle.join().unwrap();
}
