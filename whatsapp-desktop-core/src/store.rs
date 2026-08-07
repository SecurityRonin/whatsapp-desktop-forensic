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
use chromium_storage_indexeddb::{IndexedDbRecord, RecordValue};
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

/// A canonical, stable, **injective** identity for one object-store record.
///
/// An IndexedDB primary key is unique only within one object store of one
/// database, and a single `<origin>.indexeddb.leveldb` directory holds every
/// database of that origin — the `KeyPrefix`'s `(database id, object store id)` is
/// what separates them. Both coordinates therefore belong in the identity:
/// grouping on the resolved store *name* alone would merge two databases' key
/// spaces, and would also merge a retired object store with a recreated one of the
/// same name (`deleteObjectStore` + `createObjectStore` yields a new id).
///
/// The key itself is rendered with `Debug` for **every** variant, so the rendering
/// is variant-tagged and cannot collide: passing `IdbKey::String` through verbatim
/// would make the string key `Number(1.0)` and the numeric key `1.0` one identity.
fn record_identity(r: &IndexedDbRecord) -> (u64, u64, String) {
    (r.database_id, r.object_store_id, format!("{:?}", r.key))
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
    let mut groups: BTreeMap<(u64, u64, String), Vec<&IndexedDbRecord>> = BTreeMap::new();
    for r in records {
        if r.object_store.as_deref() == Some(store) {
            groups.entry(record_identity(r)).or_default().push(r);
        }
    }

    let mut out = Vec::new();
    for group in groups.values() {
        let Some(latest) = group.iter().max_by_key(|r| r.seq) else {
            // Kept as a total-function guard: `Vec` non-emptiness is not expressible
            // in the type here, so a future refactor that groups differently degrades
            // to skipping the key rather than panicking.
            continue; // cov:unreachable: every group is created by `groups.entry(..).or_default().push(r)` above, which pushes before the entry is ever read, so no group is empty and `max_by_key` on a non-empty iterator always returns `Some`
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
