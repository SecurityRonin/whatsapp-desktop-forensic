//! WhatsApp Web `contact` object-store records — the contact roster.

use crate::schema;
use crate::value::str_field;
use chromium_storage_indexeddb::{IndexedDbRecord, RecordValue, V8Value};
use serde::Serialize;

/// One decoded contact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Contact {
    /// Contact id (JID, also the primary key).
    pub id: String,
    /// `name` — address-book name.
    pub name: Option<String>,
    /// `pushname` — server-provided push name.
    pub push_name: Option<String>,
    /// `notifyName` — notification display name.
    pub notify_name: Option<String>,
    /// `shortName` — short display name.
    pub short_name: Option<String>,
    /// `true` if recovered from a deletion tombstone.
    pub deleted: bool,
    /// LevelDB sequence number of the source record.
    pub seq: u64,
}

impl Contact {
    /// Build a [`Contact`] from a decoded IndexedDB `contact`-store record, or
    /// `None` when the value is not a decodable V8 object.
    #[must_use]
    pub fn from_record(record: &IndexedDbRecord) -> Option<Contact> {
        match &record.value {
            RecordValue::V8(v) => Some(Contact::from_v8(v, record.deleted, record.seq)),
            RecordValue::Undecoded { .. } => None,
        }
    }

    #[must_use]
    pub(crate) fn from_v8(v: &V8Value, deleted: bool, seq: u64) -> Contact {
        Contact {
            id: str_field(v, schema::F_ID).unwrap_or_default(),
            name: str_field(v, schema::F_NAME),
            push_name: str_field(v, schema::F_PUSHNAME),
            notify_name: str_field(v, schema::F_NOTIFY_NAME),
            short_name: str_field(v, schema::F_SHORT_NAME),
            deleted,
            seq,
        }
    }
}
