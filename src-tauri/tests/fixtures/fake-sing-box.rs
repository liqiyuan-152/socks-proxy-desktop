// A controlled process fixture. Compiled by sing_box_process_tests.rs on the host.
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

fn value<'a>(text: &'a str, field: &str) -> &'a str {
    text.split_once(field).expect("field in rendered config").1
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("idle") {
        thread::sleep(Duration::from_secs(30));
        return;
    }
    assert_eq!(args.get(1).map(String::as_str), Some("run"));
    assert_eq!(args.get(2).map(String::as_str), Some("-c"));
    let config = fs::read_to_string(&args[3]).unwrap();
    let port: u16 = value(&config, "\"listen_port\":")
        .chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap();
    let controller: u16 = value(&config, "\"external_controller\":\"127.0.0.1:")
        .chars().take_while(char::is_ascii_digit).collect::<String>().parse().unwrap();
    let secret = value(&config, "\"secret\":\"").split('"').next().unwrap().to_owned();
    let proxy = TcpListener::bind(("127.0.0.1", port)).unwrap();
    thread::spawn(move || {
        for connection in proxy.incoming() {
            drop(connection);
        }
    });
    let api = TcpListener::bind(("127.0.0.1", controller)).unwrap();
    for connection in api.incoming() {
        let Ok(mut stream) = connection else { continue };
        let mut request = [0u8; 4096];
        let Ok(count) = stream.read(&mut request) else { continue };
        let body = if String::from_utf8_lossy(&request[..count])
            .contains(&format!("Authorization: Bearer {secret}\r\n"))
        {
            b"{\"connections\":[]}".as_slice()
        } else {
            b"{}".as_slice()
        };
        let code = if body.len() > 2 { "200 OK" } else { "401 Unauthorized" };
        let response = format!(
            "HTTP/1.1 {code}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(body);
    }
}
