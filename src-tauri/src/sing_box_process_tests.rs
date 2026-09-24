use super::*;
use crate::models::{ProxyProfile, ProxyProtocol};
use std::collections::HashMap;

fn configuration() -> PersistedConfiguration {
    let mut configuration = PersistedConfiguration::default();
    configuration.profiles.push(ProxyProfile {
        id: "test-proxy".into(),
        name: "Test".into(),
        protocol: ProxyProtocol::Socks5,
        host: "127.0.0.1".into(),
        port: 1080,
        authentication_enabled: false,
        credential_ref: None,
        enabled: true,
    });
    configuration.active_profile_id = Some("test-proxy".into());
    configuration
}

#[test]
fn refuses_unverified_binary_without_launching_it() {
    let directory = tempfile::tempdir().unwrap();
    let fake = directory.path().join("sing-box.exe");
    std::fs::write(&fake, b"not the pinned binary").unwrap();
    let error = SingBoxProcess::start(
        &fake,
        WINDOWS_AMD64_EXE_SHA256,
        directory.path(),
        &configuration(),
        RuntimeMode::Global,
        &HashMap::new(),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, "runtime_error");
    assert!(std::fs::read_dir(directory.path()).unwrap().count() == 1);
}

#[test]
fn real_core_starts_with_private_config_and_cleans_up_when_available() {
    let Ok(binary) = std::env::var("SING_BOX_TEST_BIN") else {
        return;
    };
    let binary = Path::new(&binary);
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(binary).unwrap(), &mut hasher).unwrap();
    let expected = hex::encode(hasher.finalize());
    let directory = tempfile::tempdir().unwrap();
    let runtime_dir = directory.path().join("private-runtime");
    let mut process = SingBoxProcess::start(
        binary,
        &expected,
        &runtime_dir,
        &configuration(),
        RuntimeMode::Global,
        &HashMap::new(),
    )
    .unwrap();
    assert!(process.is_running().unwrap());
    assert!(process.process_id() > 0);
    let config_path = process.config_path();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&config_path)
                .unwrap()
                .permissions()
                .mode()
                & 0o077,
            0
        );
    }
    let config_text = std::fs::read_to_string(&config_path).unwrap();
    assert!(config_text.contains("127.0.0.1"));
    let connections = process.connections_json().unwrap();
    assert!(connections["connections"].is_array());
    drop(process);
    assert!(!config_path.exists());
    assert!(!config_path.parent().unwrap().exists());
}

#[test]
fn fake_core_exit_and_cleanup_only_affect_the_owned_process() {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory.path().join(if cfg!(windows) {
        "fake-sing-box.exe"
    } else {
        "fake-sing-box"
    });
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-sing-box.rs");
    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg(&fixture)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut hasher = Sha256::new();
    std::io::copy(&mut File::open(&binary).unwrap(), &mut hasher).unwrap();
    let checksum = hex::encode(hasher.finalize());
    let mut unrelated = Command::new(&binary).arg("idle").spawn().unwrap();
    let mut process = SingBoxProcess::start(
        &binary,
        &checksum,
        directory.path(),
        &configuration(),
        RuntimeMode::Global,
        &HashMap::new(),
    )
    .unwrap();
    let owned_pid = process.process_id();
    assert_ne!(owned_pid, unrelated.id());
    assert!(process.connections_json().unwrap()["connections"].is_array());
    let config_path = process.config_path();
    process.child.kill().unwrap();
    process.child.wait().unwrap();
    assert!(!process.is_running().unwrap());
    drop(process);
    assert!(!config_path.exists());
    assert!(unrelated.try_wait().unwrap().is_none());
    unrelated.kill().unwrap();
    unrelated.wait().unwrap();
}

#[cfg(windows)]
#[test]
fn startup_cleanup_only_removes_owned_runtime_directories() {
    let root = tempfile::tempdir().unwrap();
    let stale = root.path().join(format!("run-{}", uuid::Uuid::new_v4()));
    let unrelated = root.path().join("unrelated");
    std::fs::create_dir(&stale).unwrap();
    std::fs::write(stale.join("sing-box.json"), "secret").unwrap();
    std::fs::write(stale.join("job-owned"), "").unwrap();
    std::fs::create_dir(&unrelated).unwrap();
    let legacy = root.path().join(format!("run-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&legacy).unwrap();
    cleanup_stale_runtime_dirs(root.path()).unwrap();
    assert!(!stale.exists());
    assert!(unrelated.exists());
    assert!(legacy.exists());
}
