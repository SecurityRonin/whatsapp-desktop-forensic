//! Media-reference metadata carried on media-type message records.
//!
//! WhatsApp does not store media blobs in IndexedDB — a media message carries a
//! *reference*: the MIME type, the plaintext SHA-256 (`filehash`), the media
//! decryption key (`mediaKey`), the CDN `directPath`, and dimensions. The blob
//! itself is fetched/decrypted out of band; this is the on-disk pointer to it.

use crate::schema;
use crate::value::{int_field, str_field};
use chromium_storage_indexeddb::V8Value;
use serde::Serialize;

/// The media reference on a media-type message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MediaMeta {
    /// `mimetype` — e.g. `image/jpeg`, `video/mp4`, `audio/ogg`.
    pub mimetype: Option<String>,
    /// `filehash` — base64 SHA-256 of the plaintext media.
    pub filehash: Option<String>,
    /// `mediaKey` — base64 media decryption key.
    pub media_key: Option<String>,
    /// `directPath` — CDN path to the encrypted media blob.
    pub direct_path: Option<String>,
    /// `size` — encrypted media size in bytes.
    pub size: Option<i64>,
    /// `width` — media width in pixels.
    pub width: Option<i64>,
    /// `height` — media height in pixels.
    pub height: Option<i64>,
}

impl MediaMeta {
    /// Extract a media reference from a message value, or `None` when the record
    /// carries no media field at all (a non-media message).
    #[must_use]
    pub(crate) fn from_v8(v: &V8Value) -> Option<MediaMeta> {
        let meta = MediaMeta {
            mimetype: str_field(v, schema::F_MIMETYPE),
            filehash: str_field(v, schema::F_FILEHASH),
            media_key: str_field(v, schema::F_MEDIA_KEY),
            direct_path: str_field(v, schema::F_DIRECT_PATH),
            size: int_field(v, schema::F_SIZE),
            width: int_field(v, schema::F_WIDTH),
            height: int_field(v, schema::F_HEIGHT),
        };
        if meta.is_empty() {
            None
        } else {
            Some(meta)
        }
    }

    /// `true` when no media field was present.
    fn is_empty(&self) -> bool {
        self.mimetype.is_none()
            && self.filehash.is_none()
            && self.media_key.is_none()
            && self.direct_path.is_none()
            && self.size.is_none()
            && self.width.is_none()
            && self.height.is_none()
    }
}
