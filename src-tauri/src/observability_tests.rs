use super::*;
use serde_json::json;

fn connection() -> Value {
    json!({
        "id": "8d4f3431-3912-4d4f-a715-553cfc213ecc",
        "start": "2026-09-23T10:23:12.001Z",
        "metadata": { "host": "example.com", "destinationPort": "443" },
        "rule": "domain=example.com port=443 => route(selected-proxy)",
        "chains": ["selected-proxy"],
        "upload": 100, "download": 200,
        "headers": { "Authorization": "Bearer never-expose" },
        "url": "https://example.com/?password=never-expose",
    })
}

#[test]
fn parses_only_verified_active_fields_and_copies_without_secrets() {
    let raw = json!({ "connections": [connection()], "downloadTotal": 123 });
    let snapshot = parse_connections(&raw).unwrap();
    assert_eq!(snapshot.status, ObservationStatus::Available);
    assert_eq!(snapshot.active_count, Some(1));
    assert!(!snapshot.history_available);
    let details = snapshot
        .copy_detail("8d4f3431-3912-4d4f-a715-553cfc213ecc")
        .unwrap();
    assert!(details.contains("example.com:443"));
    assert!(details.contains("route(selected-proxy)"));
    assert!(!details.contains("never-expose"));
    assert!(!serde_json::to_string(&snapshot)
        .unwrap()
        .contains("never-expose"));
}

#[test]
fn rejects_unverifiable_or_unsafe_fields_instead_of_guessing() {
    let mut raw = connection();
    raw["metadata"]["host"] = json!("https://user:password@example.com/secret?token=123");
    assert!(parse_connections(&json!({"connections": [raw]})).is_err());
    let mut raw = connection();
    raw["rule"] = json!("password=abc => route(selected-proxy)");
    assert!(parse_connections(&json!({"connections": [raw]})).is_err());
    let mut raw = connection();
    raw["chains"] = json!(["unknown-user-tag"]);
    assert!(parse_connections(&json!({"connections": [raw]})).is_err());
}

struct MissingInterface;
impl ActiveConnectionSource for MissingInterface {
    fn connections_json(&self) -> Result<Value, AppError> {
        Err(AppError::unavailable(
            "secret source error: Bearer never-expose",
        ))
    }
}

#[test]
fn unavailable_interface_degrades_without_leaking_errors_or_inventing_history() {
    let snapshot = read_connections(&MissingInterface);
    assert_eq!(snapshot.status, ObservationStatus::Degraded);
    assert_eq!(snapshot.active_count, None);
    assert!(snapshot.recent.is_empty());
    assert!(!snapshot.history_available);
    assert!(!serde_json::to_string(&snapshot)
        .unwrap()
        .contains("never-expose"));
}

#[test]
fn limits_recent_items_without_claiming_historical_results() {
    let mut items = Vec::new();
    for i in 0..25 {
        let mut item = connection();
        item["id"] = json!(format!("8d4f3431-3912-4d4f-a715-{:012x}", i));
        items.push(item);
    }
    let snapshot = parse_connections(&json!({ "connections": items })).unwrap();
    assert_eq!(snapshot.active_count, Some(25));
    assert_eq!(snapshot.recent.len(), 20);
}
