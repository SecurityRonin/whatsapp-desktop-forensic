//! WhatsApp Desktop (Electron) reader — scaffold.
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub use chromium_storage_indexeddb::{
    decode_records, read_dir, IdbKey, IndexedDbRecord, RecordValue, V8Value,
};
