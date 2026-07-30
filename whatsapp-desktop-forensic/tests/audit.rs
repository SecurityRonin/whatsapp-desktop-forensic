//! Analyzer findings over the real minted store (tier-2 oracle).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use forensicnomicon_core::report::{Category, Finding, Severity};
use std::path::PathBuf;
use whatsapp_desktop_core::{parse_records, IdbKey, IndexedDbRecord, RecordValue, V8Value};
use whatsapp_desktop_forensic::{audit, audit_path};

fn data_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb"
    ))
}

fn by_code<'a>(fs: &'a [Finding], code: &str) -> Vec<&'a Finding> {
    fs.iter().filter(|f| f.code.as_ref() == code).collect()
}

#[test]
fn flags_recovered_deleted_message() {
    let findings = audit_path(&data_dir()).expect("audit");
    let deleted = by_code(&findings, "WA-MSG-DELETED-RECOVERED");
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted[0].severity, Some(Severity::Medium));
    assert_eq!(deleted[0].category, Category::Residue);
    assert!(
        deleted[0]
            .context
            .external_refs
            .iter()
            .any(|r| r.id == "T1070"),
        "deleted-message recovery is consistent with T1070 Indicator Removal"
    );
}

#[test]
fn flags_encrypted_message_body_as_info() {
    let findings = audit_path(&data_dir()).expect("audit");
    let enc = by_code(&findings, "WA-MSG-ENCRYPTED-BODY");
    assert_eq!(enc.len(), 1, "one encrypted-body message minted");
    assert_eq!(enc[0].severity, Some(Severity::Info));
}

#[test]
fn flags_media_reference() {
    let findings = audit_path(&data_dir()).expect("audit");
    let media = by_code(&findings, "WA-MSG-MEDIA-REF");
    assert_eq!(media.len(), 1);
    assert_eq!(media[0].severity, Some(Severity::Info));
}

#[test]
fn a_deleted_message_with_a_zero_timestamp_is_not_dated_to_1970() {
    // The `when` evidence on a recovered-deletion finding is the message's send
    // time. A zero `t` is absent, so `when` must be empty — never the fabricated
    // "1970-01-01T00:00:00Z" that reads as a real send time in a report.
    let store = parse_records(&[
        IndexedDbRecord {
            database_id: 1,
            object_store_id: 1,
            database: Some("model-storage".to_string()),
            object_store: Some("message".to_string()),
            key: IdbKey::String("false_15551239999@c.us_ZEROTIME00000001".into()),
            value: RecordValue::V8(V8Value::Object(vec![
                (
                    "id".to_string(),
                    V8Value::String("false_15551239999@c.us_ZEROTIME00000001".to_string()),
                ),
                ("t".to_string(), V8Value::Int(0)),
            ])),
            seq: 1,
            deleted: false,
        },
        IndexedDbRecord {
            database_id: 1,
            object_store_id: 1,
            database: Some("model-storage".to_string()),
            object_store: Some("message".to_string()),
            key: IdbKey::String("false_15551239999@c.us_ZEROTIME00000001".into()),
            value: RecordValue::Undecoded {
                raw: vec![],
                error: "deletion tombstone".to_string(),
            },
            seq: 2,
            deleted: true,
        },
    ]);
    let findings = audit(&store);
    let deleted = by_code(&findings, "WA-MSG-DELETED-RECOVERED");
    assert_eq!(deleted.len(), 1);
    let when = deleted[0]
        .evidence
        .iter()
        .find(|e| e.field == "when")
        .expect("a `when` evidence row");
    assert_eq!(
        when.value, "",
        "a zero `t` is no send time — never rendered as 1970-01-01T00:00:00Z"
    );
}
