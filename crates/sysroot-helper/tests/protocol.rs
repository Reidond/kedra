use sysroot_helper::protocol::{MAX_REQUEST, decode};

#[test]
fn requests_cannot_choose_a_key_path_scope_or_arbitrary_command() {
    for bytes in [
        r#"{"schema_version":1,"request":{"operation":"status","verified":true}}"#,
        r#"{"schema_version":1,"request":{"operation":"status","public_key":"/tmp/key"}}"#,
        r#"{"schema_version":1,"request":{"operation":"exec","command":"sh"}}"#,
        r#"{"schema_version":2,"request":{"operation":"status"}}"#,
        r#"{"schema_version":2,"schema_version":1,"request":{"operation":"status"}}"#,
        r#"{"schema_version":1,"request":{"operation":"status"},"scope":"other"}"#,
    ] {
        assert!(decode(bytes.as_bytes()).is_err(), "{bytes}");
    }
    assert!(decode(&vec![b' '; MAX_REQUEST + 1]).is_err());
    assert!(decode(br#"{"schema_version":1,"request":{"operation":"status"}}"#).is_ok());
}
