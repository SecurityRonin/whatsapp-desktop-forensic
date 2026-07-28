//! WhatsApp Web `message` object-store records.
//!
//! A message's metadata (`id`, `t`, `from`, `to`, `type`, …) is plaintext, but
//! the **text body is AES-CBC-encrypted** inside `msgRowOpaqueData` — this reader
//! surfaces that envelope as [`MessageBody::Encrypted`] and **never** fabricates
//! plaintext. Decryption (given the derived key) lives in [`crate::decrypt_body`].

use crate::media::MediaMeta;
use crate::schema;
use crate::value::{bytes_field, field, int_field, str_field};
use chromium_storage_indexeddb::{IndexedDbRecord, RecordValue, V8Value};
use serde::Serialize;

/// The AES-CBC-encrypted body envelope (`msgRowOpaqueData`). The ciphertext is
/// carried verbatim; it is decrypted only on request via [`crate::decrypt_body`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EncryptedBody {
    /// `_keyId` — identifies the derived key that decrypts this body.
    pub key_id: Option<String>,
    /// `iv` — the AES-CBC initialization vector.
    pub iv: Vec<u8>,
    /// `_data` — the ciphertext.
    pub ciphertext: Vec<u8>,
}

/// A message body as it exists on disk — never speculatively decrypted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum MessageBody {
    /// An encrypted `msgRowOpaqueData` envelope (the normal case for text). The
    /// analyst must supply the derived key to read it; see [`crate::decrypt_body`].
    Encrypted(EncryptedBody),
    /// No body field on this record (media/system messages, or schema drift).
    /// The absence is reported as-is, never filled with invented text.
    None,
}

/// One decoded WhatsApp Web message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Message {
    /// Message id (`id`, also the primary key).
    pub id: String,
    /// `t` — send time, seconds since the Unix epoch.
    pub timestamp_secs: Option<i64>,
    /// `from` — sender JID.
    pub from: Option<String>,
    /// `to` — recipient JID.
    pub to: Option<String>,
    /// `type` — `chat`, `image`, `video`, `audio`, `ptt`, `document`, …
    pub kind: Option<String>,
    /// `ack` — delivery/read acknowledgement level.
    pub ack: Option<i64>,
    /// `notifyName` — sender push name at send time.
    pub notify_name: Option<String>,
    /// The on-disk body (encrypted or absent).
    pub body: MessageBody,
    /// Media reference, on media-type messages.
    pub media: Option<MediaMeta>,
    /// `true` if this message was recovered from a deletion tombstone.
    pub deleted: bool,
    /// LevelDB sequence number of the source record.
    pub seq: u64,
}

impl Message {
    /// Build a [`Message`] from a decoded IndexedDB `message`-store record.
    ///
    /// Returns `None` when the record's value is not a decodable V8 object (e.g.
    /// an empty deletion tombstone — recovery of such is handled by the
    /// aggregate parser).
    #[must_use]
    pub fn from_record(record: &IndexedDbRecord) -> Option<Message> {
        let v = match &record.value {
            RecordValue::V8(v) => v,
            RecordValue::Undecoded { .. } => return None,
        };
        Some(Message::from_v8(v, record.deleted, record.seq))
    }

    /// Build a [`Message`] from an already-decoded V8 object.
    #[must_use]
    pub(crate) fn from_v8(v: &V8Value, deleted: bool, seq: u64) -> Message {
        let id = str_field(v, schema::F_ID).unwrap_or_default();
        Message {
            id,
            timestamp_secs: int_field(v, schema::F_T),
            from: str_field(v, schema::F_FROM),
            to: str_field(v, schema::F_TO),
            kind: str_field(v, schema::F_TYPE),
            ack: int_field(v, schema::F_ACK),
            notify_name: str_field(v, schema::F_NOTIFY_NAME),
            body: decode_body(v),
            media: MediaMeta::from_v8(v),
            deleted,
            seq,
        }
    }
}

/// Extract the encrypted body envelope, or [`MessageBody::None`] when absent.
fn decode_body(v: &V8Value) -> MessageBody {
    let Some(opaque) = field(v, schema::F_MSG_ROW_OPAQUE_DATA) else {
        return MessageBody::None;
    };
    // The envelope must carry both the IV and the ciphertext to be meaningful;
    // a partial envelope is reported as absent rather than a bogus zero-length body.
    match (
        bytes_field(opaque, schema::F_OPAQUE_IV),
        bytes_field(opaque, schema::F_OPAQUE_DATA),
    ) {
        (Some(iv), Some(ciphertext)) => MessageBody::Encrypted(EncryptedBody {
            key_id: str_field(opaque, schema::F_OPAQUE_KEY_ID),
            iv,
            ciphertext,
        }),
        _ => MessageBody::None,
    }
}
