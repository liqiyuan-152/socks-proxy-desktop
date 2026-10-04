use super::*;
use std::{fs, path::Path};

#[test]
fn captures_matches_rejects_failures_and_reaps_timed_out_processes() {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join(if cfg!(windows) {
        "fixture.exe"
    } else {
        "fixture"
    });
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-rule-match.rs");
    assert!(Command::new("rustc")
        .arg(fixture)
        .arg("-o")
        .arg(&binary)
        .status()
        .unwrap()
        .success());
    let timeout = Duration::from_secs(2);
    assert!(run(Command::new(&binary).arg("match"), timeout)
        .unwrap()
        .starts_with("match rules."));
    assert!(run(Command::new(&binary).arg("fail"), timeout).is_err());
    assert!(run(Command::new(&binary).arg("flood"), timeout)
        .unwrap_err()
        .message
        .contains("超过限制"));
    let marker = directory.path().join("marker");
    let start = Instant::now();
    assert!(run(
        Command::new(&binary).arg("idle").arg(&marker),
        Duration::from_millis(200)
    )
    .unwrap_err()
    .message
    .contains("超时"));
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(fs::read_to_string(&marker).unwrap(), "started");
    thread::sleep(Duration::from_millis(550));
    assert_eq!(fs::read_to_string(marker).unwrap(), "started");
}
