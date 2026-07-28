//! Fuzz the whole WhatsApp Desktop pipeline on hostile LevelDB records.
//!
//! Exercises the IndexedDB key/V8 decode plus every record builder
//! (message/chat/contact/media), the dedup + deleted-record recovery, and the
//! timeline. Must never panic on lying prefixes, huge counts, malformed V8
//! streams, or partial `msgRowOpaqueData` envelopes.
#![no_main]
use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;

use leveldb_core::Record;
use whatsapp_desktop_core::{decode_records, parse_records};

fuzz_target!(|input: Vec<(Vec<u8>, Vec<u8>, u64, bool)>| {
    let records: Vec<Record> = input
        .into_iter()
        .map(|(key, value, seq, deleted)| Record {
            key,
            value,
            seq,
            deleted,
            origin_file: PathBuf::from("fuzz"),
        })
        .collect();
    let decoded = decode_records(&records);
    let store = parse_records(&decoded);
    // Touch the derived outputs so nothing is optimized away.
    let _ = (
        store.messages.len(),
        store.chats.len(),
        store.contacts.len(),
        store.timeline.len(),
    );
});
