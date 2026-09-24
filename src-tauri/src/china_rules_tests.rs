use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::PathBuf, process::Command, thread, time::Duration};

#[test]
fn bundled_china_rules_check_and_load_offline_with_pinned_core() {
    let Ok(core) = std::env::var("SING_BOX_TEST_BIN") else {
        return;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/china-rules");
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["core_version"], "1.14.1");
    for (name, expected) in manifest["hashes"].as_object().unwrap() {
        let contents = fs::read(root.join(name)).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(contents)),
            expected.as_str().unwrap()
        );
    }
    for (name, source) in [
        ("LICENSE.domain-list-community", "domain"),
        ("LICENSE.china-operator-ip", "ip"),
    ] {
        let contents = fs::read(root.join(name)).unwrap();
        assert_eq!(
            hex::encode(Sha256::digest(contents)),
            manifest["sources"][source]["license_sha256"]
                .as_str()
                .unwrap()
        );
    }

    for (name, target) in [
        ("china-domains.srs", "baidu.com"),
        ("china-ipv4.srs", "1.0.1.1"),
        ("china-ipv6.srs", "240e::1"),
    ] {
        let result = Command::new(&core)
            .args(["rule-set", "match"])
            .arg(root.join(name))
            .args([target, "-f", "binary"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(String::from_utf8_lossy(&result.stderr).contains("match rules."));
    }
    let negative = Command::new(&core)
        .args(["rule-set", "match"])
        .arg(root.join("china-domains.srs"))
        .args(["example.invalid", "-f", "binary"])
        .output()
        .unwrap();
    assert!(negative.status.success());
    assert!(!String::from_utf8_lossy(&negative.stderr).contains("match rules."));

    let rule_set = |name: &str, tag: &str| {
        json!({
            "type": "local", "tag": tag, "format": "binary", "path": root.join(name)
        })
    };
    let candidate = json!({
        "log": { "level": "warn" },
        "dns": { "servers": [{ "type": "udp", "tag": "bootstrap", "server": "223.5.5.5" }], "final": "bootstrap", "strategy": "prefer_ipv4" },
        "inbounds": [{ "type": "mixed", "listen": "127.0.0.1", "listen_port": 0 }],
        "outbounds": [
            { "type": "direct", "tag": "direct" },
            { "type": "socks", "tag": "proxy", "server": "127.0.0.1", "server_port": 19999 }
        ],
        "route": {
            "rule_set": [
                rule_set("china-domains.srs", "cn-domain"),
                rule_set("china-ipv4.srs", "cn-v4"),
                rule_set("china-ipv6.srs", "cn-v6")
            ],
            "rules": [
                { "rule_set": ["cn-domain"], "action": "route", "outbound": "direct" },
                { "action": "resolve", "server": "bootstrap" },
                { "rule_set": ["cn-v4", "cn-v6"], "action": "route", "outbound": "direct" }
            ],
            "final": "proxy"
        }
    });
    let mut config = tempfile::NamedTempFile::new().unwrap();
    config.write_all(candidate.to_string().as_bytes()).unwrap();
    let check = Command::new(&core)
        .arg("check")
        .arg("-c")
        .arg(config.path())
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let mut run = Command::new(&core)
        .arg("run")
        .arg("-c")
        .arg(config.path())
        .spawn()
        .unwrap();
    thread::sleep(Duration::from_millis(600));
    let status = run.try_wait().unwrap();
    if status.is_none() {
        run.kill().unwrap();
        run.wait().unwrap();
    }
    assert!(
        status.is_none(),
        "the core exited before offline rule-set load"
    );
}
