//! Message extraction over the real minted Chromium `model-storage` store
//! (tier-2 oracle — see `tests/data/README.md`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use whatsapp_desktop_core::{read_dir, Message, MessageBody};

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

fn live_messages() -> Vec<Message> {
    read_dir(&data_dir())
        .unwrap()
        .iter()
        .filter(|r| r.object_store.as_deref() == Some("message") && !r.deleted)
        .filter_map(Message::from_record)
        .collect()
}

#[test]
fn extracts_text_message_metadata() {
    let m = live_messages()
        .into_iter()
        .find(|m| m.id.ends_with("3EB0A1B2C3D4E5F6"))
        .expect("text message present");
    assert_eq!(m.timestamp_secs, Some(1_596_233_451));
    assert_eq!(m.from.as_deref(), Some("15551230000@c.us"));
    assert_eq!(m.to.as_deref(), Some("15551239999@c.us"));
    assert_eq!(m.kind.as_deref(), Some("chat"));
    assert_eq!(m.ack, Some(3));
    assert_eq!(m.notify_name.as_deref(), Some("Alice"));
}

#[test]
fn text_body_is_surfaced_as_encrypted_never_fabricated() {
    let m = live_messages()
        .into_iter()
        .find(|m| m.id.ends_with("3EB0A1B2C3D4E5F6"))
        .expect("text message present");
    match &m.body {
        MessageBody::Encrypted(b) => {
            assert_eq!(b.key_id.as_deref(), Some("1"));
            assert_eq!(b.iv.len(), 16, "iv is the 16-byte AES-CBC IV");
            assert_eq!(b.ciphertext.len(), 32, "opaque _data ciphertext");
            assert_eq!(b.iv[..4], [0, 1, 2, 3]);
        }
        other => panic!("expected an encrypted body, got {other:?}"),
    }
}

#[test]
fn message_without_opaque_body_has_no_fabricated_plaintext() {
    let m = live_messages()
        .into_iter()
        .find(|m| m.id.ends_with("ABCDEF0123456789"))
        .expect("media message present");
    assert_eq!(m.kind.as_deref(), Some("image"));
    // No msgRowOpaqueData on this record: the body is absent, never invented.
    assert!(matches!(m.body, MessageBody::None));
}
