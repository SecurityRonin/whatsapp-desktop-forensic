//! WhatsApp Web IndexedDB schema names — this parser's own domain knowledge.
//!
//! These are the object-store and field names WhatsApp Web writes into its
//! Chromium `model-storage` IndexedDB database. They are **not** re-hardcoded
//! forensicnomicon constants (forensicnomicon models WhatsApp *Desktop* as the
//! native SQLite client; the Chromium IndexedDB key-coding it *does* own is
//! consumed via [`chromium_storage_indexeddb`]). The schema below is grounded in:
//!
//! - F. Mazzoli, *Backing up WhatsApp data through the multi-device web client*:
//!   <https://mazzo.li/posts/whatsapp-backup.html>
//! - F. Paligo et al., *Browser Forensic Investigations of WhatsApp Web Utilizing
//!   IndexedDB Persistent Storage*, Future Internet 12(11):184, MDPI, 2020:
//!   <https://www.mdpi.com/1999-5903/12/11/184>
//!
//! WhatsApp's internal schema is undocumented and versions over time; treat every
//! field as optional and degrade gracefully when one is absent.

/// The IndexedDB database holding chats/messages/contacts.
pub const DB_MODEL_STORAGE: &str = "model-storage";

/// Object store: message records.
pub const STORE_MESSAGE: &str = "message";
/// Object store: chat/conversation roster.
pub const STORE_CHAT: &str = "chat";
/// Object store: contact roster.
pub const STORE_CONTACT: &str = "contact";
/// Object store: group metadata.
pub const STORE_GROUP_METADATA: &str = "group-metadata";

// ── message value fields ──────────────────────────────────────────────────────
/// Message id (also the object-store keyPath).
pub const F_ID: &str = "id";
/// Message timestamp, **seconds** since the Unix epoch (WhatsApp Web's own unit
/// — not the Cocoa 2001 epoch the Apple/Core-Data client uses). Zero means unset.
pub const F_T: &str = "t";
/// Sender JID.
pub const F_FROM: &str = "from";
/// Recipient JID.
pub const F_TO: &str = "to";
/// Message type (`chat`, `image`, `video`, `audio`, `ptt`, `document`, …).
pub const F_TYPE: &str = "type";
/// Delivery/read acknowledgement level.
pub const F_ACK: &str = "ack";
/// Sender's notify (push) name at send time.
pub const F_NOTIFY_NAME: &str = "notifyName";

/// The AES-CBC-encrypted message body envelope.
pub const F_MSG_ROW_OPAQUE_DATA: &str = "msgRowOpaqueData";
/// Opaque envelope: key identifier for the derived AES-CBC key.
pub const F_OPAQUE_KEY_ID: &str = "_keyId";
/// Opaque envelope: AES-CBC initialization vector (`ArrayBuffer`).
pub const F_OPAQUE_IV: &str = "iv";
/// Opaque envelope: ciphertext (`ArrayBuffer`).
pub const F_OPAQUE_DATA: &str = "_data";

// ── media fields (present on media-type messages) ─────────────────────────────
/// Media MIME type.
pub const F_MIMETYPE: &str = "mimetype";
/// SHA-256 hash of the plaintext media (base64).
pub const F_FILEHASH: &str = "filehash";
/// Media decryption key (base64).
pub const F_MEDIA_KEY: &str = "mediaKey";
/// CDN path to the encrypted media blob.
pub const F_DIRECT_PATH: &str = "directPath";
/// Encrypted media size in bytes.
pub const F_SIZE: &str = "size";
/// Media width in pixels.
pub const F_WIDTH: &str = "width";
/// Media height in pixels.
pub const F_HEIGHT: &str = "height";

// ── chat / contact fields ─────────────────────────────────────────────────────
/// Display name (chat or contact).
pub const F_NAME: &str = "name";
/// Unread message count (chat).
pub const F_UNREAD_COUNT: &str = "unreadCount";
/// Server-provided push name (contact).
pub const F_PUSHNAME: &str = "pushname";
/// Short display name (contact).
pub const F_SHORT_NAME: &str = "shortName";
