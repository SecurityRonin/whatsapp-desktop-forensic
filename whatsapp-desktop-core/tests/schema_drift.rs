//! Schema-drift and undecodable-record behaviour of the readers.
//!
//! WhatsApp's IndexedDB schema is undocumented and versions over time, and a
//! LevelDB stream carries records whose value failed to deserialize at all. These
//! tests pin the documented degradation: a wrong-typed or absent field yields
//! `None` (never a coerced or fabricated value), an undecodable value yields no
//! record, and a key whose content was fully compacted away is skipped rather
//! than recovered as an empty shell.
//!
//! Tier 3 (self-authored fixtures): the subject under test is this crate's own
//! accessor semantics — the *rule* is the specification — not a byte-decode that
//! an external oracle could adjudicate. The byte-decode layer is validated
//! against `ccl_chromium_reader` in `differential_ccl.rs` (tier 1).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use whatsapp_desktop_core::{
    parse_records, schema, Chat, Contact, IdbKey, IndexedDbRecord, Message, MessageBody,
    RecordValue, V8Value,
};

/// A V8 object from `(name, value)` pairs, in insertion order.
fn obj(fields: Vec<(&str, V8Value)>) -> V8Value {
    V8Value::Object(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn text(s: &str) -> V8Value {
    V8Value::String(s.to_string())
}

fn record(
    store: &str,
    key: IdbKey,
    value: RecordValue,
    seq: u64,
    deleted: bool,
) -> IndexedDbRecord {
    IndexedDbRecord {
        database_id: 1,
        object_store_id: 1,
        database: Some(schema::DB_MODEL_STORAGE.to_string()),
        object_store: Some(store.to_string()),
        key,
        value,
        seq,
        deleted,
    }
}

/// A value the deserializer could not decode — raw bytes + error retained.
fn undecoded() -> RecordValue {
    RecordValue::Undecoded {
        raw: vec![0xff, 0x0d, 0x99],
        error: "unknown V8 tag 0x99".to_string(),
    }
}

fn v8(value: V8Value) -> RecordValue {
    RecordValue::V8(value)
}

// ── undecodable values yield no typed record ─────────────────────────────────

#[test]
fn undecodable_value_yields_no_message() {
    let r = record(
        schema::STORE_MESSAGE,
        IdbKey::String("m1".into()),
        undecoded(),
        7,
        false,
    );
    assert_eq!(Message::from_record(&r), None);
}

#[test]
fn undecodable_value_yields_no_chat() {
    let r = record(
        schema::STORE_CHAT,
        IdbKey::String("c1".into()),
        undecoded(),
        7,
        false,
    );
    assert_eq!(Chat::from_record(&r), None);
}

#[test]
fn undecodable_value_yields_no_contact() {
    let r = record(
        schema::STORE_CONTACT,
        IdbKey::String("p1".into()),
        undecoded(),
        7,
        false,
    );
    assert_eq!(Contact::from_record(&r), None);
}

// ── field accessors over a drifted value graph ───────────────────────────────

#[test]
fn non_object_value_yields_an_empty_message_not_a_panic() {
    // A record whose value decoded to a bare number: no property can be read, so
    // every field degrades to absent rather than crashing the reader.
    let r = record(
        schema::STORE_MESSAGE,
        IdbKey::String("m1".into()),
        v8(V8Value::Int(7)),
        3,
        false,
    );
    let m = Message::from_record(&r).expect("a decodable value still yields a record");
    assert!(m.id.is_empty(), "no `id` property to read");
    assert_eq!(m.timestamp_secs, None);
    assert_eq!(m.from, None);
    assert_eq!(m.body, MessageBody::None);
    assert_eq!(m.media, None);
    assert_eq!(m.seq, 3);
}

#[test]
fn wrong_typed_string_field_is_absent_never_coerced() {
    // `name` present but numeric — reported absent, not rendered as "5".
    let r = record(
        schema::STORE_CHAT,
        IdbKey::String("15551239999@c.us".into()),
        v8(obj(vec![
            (schema::F_ID, text("15551239999@c.us")),
            (schema::F_NAME, V8Value::Int(5)),
        ])),
        1,
        false,
    );
    let c = Chat::from_record(&r).unwrap();
    assert_eq!(c.id, "15551239999@c.us");
    assert_eq!(c.name, None);
}

#[test]
fn wrong_typed_int_field_is_absent_never_truncated() {
    // `t` is a string and `unreadCount` a fractional double: both absent rather
    // than parsed-from-text or silently truncated to 1.
    let r = record(
        schema::STORE_CHAT,
        IdbKey::String("15551239999@c.us".into()),
        v8(obj(vec![
            (schema::F_ID, text("15551239999@c.us")),
            (schema::F_T, text("1596233500")),
            (schema::F_UNREAD_COUNT, V8Value::Double(1.5)),
        ])),
        1,
        false,
    );
    let c = Chat::from_record(&r).unwrap();
    assert_eq!(c.timestamp_secs, None);
    assert_eq!(c.unread_count, None);
}

#[test]
fn partial_or_wrong_typed_opaque_envelope_yields_no_body() {
    // The envelope must carry both `iv` and `_data` as ArrayBuffers. A wrong-typed
    // IV, or a missing ciphertext, is reported as *no body* — never a bogus
    // zero-length or half-decrypted one.
    let wrong_typed_iv = record(
        schema::STORE_MESSAGE,
        IdbKey::String("m1".into()),
        v8(obj(vec![
            (schema::F_ID, text("m1")),
            (
                schema::F_MSG_ROW_OPAQUE_DATA,
                obj(vec![
                    (schema::F_OPAQUE_IV, text("not-an-array-buffer")),
                    (schema::F_OPAQUE_DATA, V8Value::ArrayBuffer(vec![1, 2, 3])),
                ]),
            ),
        ])),
        1,
        false,
    );
    assert_eq!(
        Message::from_record(&wrong_typed_iv).unwrap().body,
        MessageBody::None
    );

    let ciphertext_absent = record(
        schema::STORE_MESSAGE,
        IdbKey::String("m2".into()),
        v8(obj(vec![
            (schema::F_ID, text("m2")),
            (
                schema::F_MSG_ROW_OPAQUE_DATA,
                obj(vec![(
                    schema::F_OPAQUE_IV,
                    V8Value::ArrayBuffer(vec![0; 16]),
                )]),
            ),
        ])),
        2,
        false,
    );
    assert_eq!(
        Message::from_record(&ciphertext_absent).unwrap().body,
        MessageBody::None
    );
}

// ── aggregate store: key identity and fully-compacted keys ───────────────────

#[test]
fn non_string_primary_keys_are_distinguished() {
    // WhatsApp keys messages by string id, but the store must not collapse (or
    // panic on) records keyed by any other IDBKey type. Same numeric key ⇒ one
    // record (newest wins); different numeric keys ⇒ two.
    let msg = |id: &str| v8(obj(vec![(schema::F_ID, text(id))]));

    let same_key = parse_records(&[
        record(
            schema::STORE_MESSAGE,
            IdbKey::Number(1.0),
            msg("old"),
            1,
            false,
        ),
        record(
            schema::STORE_MESSAGE,
            IdbKey::Number(1.0),
            msg("new"),
            2,
            false,
        ),
    ]);
    assert_eq!(same_key.messages.len(), 1);
    assert_eq!(same_key.messages[0].id, "new");

    let distinct_keys = parse_records(&[
        record(
            schema::STORE_MESSAGE,
            IdbKey::Number(1.0),
            msg("one"),
            1,
            false,
        ),
        record(
            schema::STORE_MESSAGE,
            IdbKey::Number(2.0),
            msg("two"),
            2,
            false,
        ),
        record(
            schema::STORE_MESSAGE,
            IdbKey::Binary(vec![0xde, 0xad]),
            msg("three"),
            3,
            false,
        ),
    ]);
    let mut ids: Vec<_> = distinct_keys
        .messages
        .iter()
        .map(|m| m.id.clone())
        .collect();
    ids.sort();
    assert_eq!(ids, ["one", "three", "two"]);
}

#[test]
fn key_with_no_surviving_content_is_skipped_not_fabricated() {
    // A key whose every record is undecodable (content fully compacted away) has
    // nothing to recover: it is dropped, and neighbouring keys still parse.
    let store = parse_records(&[
        record(
            schema::STORE_MESSAGE,
            IdbKey::String("gone".into()),
            undecoded(),
            1,
            false,
        ),
        record(
            schema::STORE_MESSAGE,
            IdbKey::String("gone".into()),
            undecoded(),
            2,
            true,
        ),
        record(
            schema::STORE_MESSAGE,
            IdbKey::String("kept".into()),
            v8(obj(vec![(schema::F_ID, text("kept"))])),
            3,
            false,
        ),
    ]);
    assert_eq!(store.messages.len(), 1, "the content-less key is skipped");
    assert_eq!(store.messages[0].id, "kept");
    assert!(!store.messages[0].deleted);
}
