//! The aggregate WhatsApp Desktop store: typed records + merged timeline,
//! parsed from the raw IndexedDB record stream with deduplication and
//! deleted-record recovery.
//!
//! The LevelDB record stream surfaces every version of a key, including
//! superseded puts and deletion tombstones. This layer collapses each primary
//! key to its latest state: a key whose newest record is a tombstone is reported
//! **deleted**, with its content recovered from the newest surviving put (never
//! fabricated); otherwise the newest put wins.

use crate::chat::Chat;
use crate::contact::Contact;
use crate::error::WaError;
use crate::message::Message;
use crate::schema;
use crate::timeline::TimelineEntry;
use chromium_storage_indexeddb::{IdbKey, IndexedDbRecord, RecordValue};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Everything parsed from a WhatsApp Desktop `model-storage` IndexedDB store.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct WhatsAppStore {
    /// Deduplicated messages (deleted ones recovered + flagged).
    pub messages: Vec<Message>,
    /// Deduplicated chats.
    pub chats: Vec<Chat>,
    /// Deduplicated contacts.
    pub contacts: Vec<Contact>,
    /// Merged, time-ordered message timeline.
    pub timeline: Vec<TimelineEntry>,
}

/// Open a `*.indexeddb.leveldb` directory and parse it into a [`WhatsAppStore`].
///
/// # Errors
///
/// Returns [`WaError::LevelDb`] if the LevelDB store cannot be read. A read that
/// succeeds but contains no WhatsApp records yields an empty store (a genuine
/// clean result), distinct from the loud read failure.
pub fn open(dir: &Path) -> Result<WhatsAppStore, WaError> {
    let records = chromium_storage_indexeddb::read_dir(dir)?;
    Ok(parse_records(&records))
}

/// Parse already-decoded IndexedDB records into a [`WhatsAppStore`].
#[must_use]
pub fn parse_records(records: &[IndexedDbRecord]) -> WhatsAppStore {
    let messages: Vec<Message> = resolve(records, schema::STORE_MESSAGE, |r, deleted| {
        Message::from_record(r).map(|mut m| {
            m.deleted = deleted;
            m
        })
    });
    let chats: Vec<Chat> = resolve(records, schema::STORE_CHAT, |r, deleted| {
        Chat::from_record(r).map(|mut c| {
            c.deleted = deleted;
            c
        })
    });
    let contacts: Vec<Contact> = resolve(records, schema::STORE_CONTACT, |r, deleted| {
        Contact::from_record(r).map(|mut c| {
            c.deleted = deleted;
            c
        })
    });
    let timeline = TimelineEntry::build_timeline(&messages);
    WhatsAppStore {
        messages,
        chats,
        contacts,
        timeline,
    }
}

/// A canonical, stable string identity for a record's primary key.
fn key_identity(key: &IdbKey) -> String {
    match key {
        IdbKey::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

/// Collapse each primary key in `store` to one built record.
///
/// `build(content_record, deleted)` constructs the typed record from the newest
/// surviving *content* record (a record with a decodable value), with `deleted`
/// set when the newest record for that key is a deletion tombstone. Keys whose
/// content was fully compacted away (only a value-less tombstone remains) are
/// skipped — there is nothing to recover.
fn resolve<T, F>(records: &[IndexedDbRecord], store: &str, build: F) -> Vec<T>
where
    F: Fn(&IndexedDbRecord, bool) -> Option<T>,
{
    // BTreeMap keyed by identity keeps output deterministic; each entry keeps the
    // records for that key in arrival order (LevelDB yields ascending sequence).
    let mut groups: BTreeMap<String, Vec<&IndexedDbRecord>> = BTreeMap::new();
    for r in records {
        if r.object_store.as_deref() == Some(store) {
            groups.entry(key_identity(&r.key)).or_default().push(r);
        }
    }

    let mut out = Vec::new();
    for group in groups.values() {
        let Some(latest) = group.iter().max_by_key(|r| r.seq) else {
            continue;
        };
        let deleted = latest.deleted;
        // Newest surviving content record (highest seq with a decodable value).
        let content = group
            .iter()
            .filter(|r| matches!(r.value, RecordValue::V8(_)))
            .max_by_key(|r| r.seq);
        if let Some(c) = content {
            if let Some(built) = build(c, deleted) {
                out.push(built);
            }
        }
    }
    out
}
