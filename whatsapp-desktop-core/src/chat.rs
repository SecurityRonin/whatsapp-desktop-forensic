//! WhatsApp Web `chat` object-store records — the conversation roster.

use crate::schema;
use crate::value::{int_field, str_field};
use chromium_storage_indexeddb::{IndexedDbRecord, RecordValue, V8Value};
use serde::Serialize;

/// One decoded chat / conversation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Chat {
    /// Chat id (JID, also the primary key).
    pub id: String,
    /// `name` — display name.
    pub name: Option<String>,
    /// `t` — last-activity time, seconds since the Unix epoch.
    pub timestamp_secs: Option<i64>,
    /// `unreadCount` — unread message count.
    pub unread_count: Option<i64>,
    /// `true` if recovered from a deletion tombstone.
    pub deleted: bool,
    /// LevelDB sequence number of the source record.
    pub seq: u64,
}

impl Chat {
    /// Build a [`Chat`] from a decoded IndexedDB `chat`-store record, or `None`
    /// when the value is not a decodable V8 object.
    #[must_use]
    pub fn from_record(record: &IndexedDbRecord) -> Option<Chat> {
        match &record.value {
            RecordValue::V8(v) => Some(Chat::from_v8(v, record.deleted, record.seq)),
            RecordValue::Undecoded { .. } => None,
        }
    }

    #[must_use]
    pub(crate) fn from_v8(v: &V8Value, deleted: bool, seq: u64) -> Chat {
        Chat {
            id: str_field(v, schema::F_ID).unwrap_or_default(),
            name: str_field(v, schema::F_NAME),
            timestamp_secs: int_field(v, schema::F_T),
            unread_count: int_field(v, schema::F_UNREAD_COUNT),
            deleted,
            seq,
        }
    }
}
