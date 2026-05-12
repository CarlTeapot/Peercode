use crate::session::guest_commands::parse_join_url;

#[test]
fn parses_default_ws_endpoint() {
    let info = parse_join_url("wss://example.com/ws?room=abc".to_string()).unwrap();
    assert_eq!(info.server_url, "wss://example.com");
    assert_eq!(info.room_id, "abc");
}

#[test]
fn parses_custom_path_endpoint() {
    let info = parse_join_url("wss://example.com/custom/ws?room=abc".to_string()).unwrap();
    assert_eq!(info.server_url, "wss://example.com/custom");
    assert_eq!(info.room_id, "abc");
}

#[test]
fn rejects_empty_host() {
    let result = parse_join_url("ws://?room=abc".to_string());
    assert!(
        matches!(result, Err(err) if err.contains("missing host") || err.contains("Invalid URL"))
    );
}

#[test]
fn rejects_missing_room_param() {
    let result = parse_join_url("wss://example.com/ws?foo=abc".to_string());
    assert!(matches!(result, Err(err) if err.contains("missing the ?room=")));
}
