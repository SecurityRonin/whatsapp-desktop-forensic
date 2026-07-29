//! Tier-1 **differential** validation against the independent Python oracle
//! [`cclgroupltd/ccl_chromium_reader`](https://github.com/cclgroupltd/ccl_chromium_reader).
//!
//! Where `store.rs` asserts our typed extraction against the *documented* writes
//! (construction-derived ground truth, tier 2), this test reconciles the raw
//! Chromium IndexedDB-over-LevelDB decode our parser consumes
//! (`whatsapp_desktop_core::read_dir` → [`IndexedDbRecord`]s of decoded V8
//! values) against a **separate, third-party re-implementation** reading the
//! *same* on-disk bytes. Two independent decoders agreeing on real Chrome/V8
//! output — down to the full V8 object graph, including the encrypted
//! `msgRowOpaqueData` byte blobs — is tier-1 evidence: the answer key is
//! authored by someone else, not by us.
//!
//! ## Gating (skips cleanly unless the oracle is present)
//! * `CCL_WHATSAPP_ORACLE` — path to a Python interpreter that can
//!   `import ccl_chromium_reader` (set `PYTHONPATH` to the checkout). Unset ⇒ skip.
//! * `CCL_WHATSAPP_DIR` — optional override pointing at a *different* real
//!   WhatsApp `*.indexeddb.leveldb` directory. Defaults to the committed minted
//!   store, so the differential runs against real Chromium bytes with zero setup
//!   once the oracle is available.
//!
//! Absence of `CCL_WHATSAPP_ORACLE` is the only clean-skip path; once it is set
//! we trust that assertion and fail loud on any oracle error (a broken
//! interpreter is a bootstrap failure, never a silent skip).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;

use whatsapp_desktop_core::{read_dir, IdbKey, IndexedDbRecord, RecordValue, V8Value};

/// The committed minted store (workspace-root relative).
fn default_fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/data/indexeddb/http_127.0.0.1_8731.indexeddb.leveldb")
}

/// The bundled Python oracle driver (this test's private companion).
fn oracle_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ccl_oracle.py")
}

fn unhex(s: &str) -> Vec<u8> {
    assert!(
        s.len().is_multiple_of(2),
        "odd-length hex from oracle: {s:?}"
    );
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).expect("valid hex from oracle"))
        .collect()
}

fn unhex_str(s: &str) -> String {
    String::from_utf8(unhex(s)).expect("oracle emits UTF-8 hex")
}

/// Lowercase hex of `bytes`, matching Python `bytes.hex()`.
fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Stable string identity for a primary key (mirrors `store.rs::key_identity`
/// *and* the oracle's `_key_identity`).
fn key_identity(key: &IdbKey) -> String {
    match key {
        IdbKey::String(s) => s.clone(),
        other => format!("{other:?}"),
    }
}

/// Deterministic canonical encoding of a decoded V8 value; **must** match the
/// oracle's Python `_canon` byte-for-byte (see `ccl_oracle.py` for the grammar).
fn canon(v: &V8Value, out: &mut String) {
    match v {
        V8Value::Null | V8Value::Undefined => out.push('n'),
        V8Value::Bool(true) => out.push('t'),
        V8Value::Bool(false) => out.push('f'),
        V8Value::Int(i) => {
            out.push('i');
            out.push_str(&i.to_string());
            out.push(';');
        }
        V8Value::Double(d) if d.is_finite() && d.fract() == 0.0 => {
            out.push('i');
            out.push_str(&(*d as i64).to_string());
            out.push(';');
        }
        V8Value::Double(d) => {
            out.push('d');
            out.push_str(&hex(&d.to_bits().to_le_bytes()));
            out.push(';');
        }
        V8Value::String(s) => canon_str(s, out),
        V8Value::ArrayBuffer(b) => {
            out.push('b');
            out.push_str(&hex(b));
            out.push(';');
        }
        V8Value::Array(items) => {
            out.push('A');
            out.push_str(&items.len().to_string());
            out.push(':');
            for e in items {
                canon(e, out);
            }
        }
        V8Value::Object(kv) => {
            let mut items: Vec<&(String, V8Value)> = kv.iter().collect();
            items.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            out.push('O');
            out.push_str(&items.len().to_string());
            out.push(':');
            for (k, val) in items {
                canon_str(k, out);
                canon(val, out);
            }
        }
        other => panic!(
            "value kind not covered by the differential canonicaliser (extend \
             both canon() and ccl_oracle.py::_canon if real data needs it): {other:?}"
        ),
    }
}

/// Length-prefixed string term, self-delimiting (`s<utf8-byte-len>:<str>`).
fn canon_str(s: &str, out: &mut String) {
    out.push('s');
    out.push_str(&s.len().to_string());
    out.push(':');
    out.push_str(s);
}

/// The live `(object_store, key, canon-value)` set both decoders are reconciled
/// on: collapse our full tombstone-keeping stream to the highest-seq record per
/// `(store, key)`, drop deletions, canonicalise the surviving V8 value.
fn our_live_set(recs: &[IndexedDbRecord]) -> BTreeSet<(String, String, String)> {
    // (store, key) -> (seq, deleted, canon-or-none)
    let mut best: HashMap<(String, String), (u64, bool, Option<String>)> = HashMap::new();
    for r in recs {
        let Some(store) = r.object_store.clone() else {
            continue; // unresolved store name — ccl only iterates named stores
        };
        let ident = (store, key_identity(&r.key));
        let value = match &r.value {
            RecordValue::V8(v) => {
                let mut s = String::new();
                canon(v, &mut s);
                Some(s)
            }
            RecordValue::Undecoded { .. } => None,
        };
        let replace = best.get(&ident).is_none_or(|(s, _, _)| r.seq > *s);
        if replace {
            best.insert(ident, (r.seq, r.deleted, value));
        }
    }
    best.into_iter()
        .filter_map(
            |((store, key), (_, deleted, value))| match (deleted, value) {
                (false, Some(canon)) => Some((store, key, canon)),
                _ => None, // newest is a tombstone (or value-less) — not in the live view
            },
        )
        .collect()
}

/// Run the ccl oracle over `dir` and parse its hex-line output.
fn oracle_live_set(python: &str, dir: &std::path::Path) -> BTreeSet<(String, String, String)> {
    let out = Command::new(python)
        .arg(oracle_script())
        .arg(dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to launch ccl oracle {python:?}: {e}"));
    assert!(
        out.status.success(),
        "ccl oracle exited {:?}\nstderr:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8(out.stdout).expect("oracle stdout is UTF-8");
    let mut set = BTreeSet::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let mut f = line.split('\t');
        match f.next() {
            Some("REC") => {
                let store = unhex_str(f.next().expect("REC object_store"));
                let key = unhex_str(f.next().expect("REC key"));
                let value = unhex_str(f.next().expect("REC canon value"));
                set.insert((store, key, value));
            }
            other => panic!("unrecognised oracle line tag {other:?} in {line:?}"),
        }
    }
    set
}

#[test]
fn differential_matches_ccl_chromium_reader() {
    let Ok(python) = std::env::var("CCL_WHATSAPP_ORACLE") else {
        eprintln!(
            "skipping ccl differential: set CCL_WHATSAPP_ORACLE to a Python \
             interpreter that can `import ccl_chromium_reader` (PYTHONPATH=<checkout>)"
        );
        return;
    };
    if python.trim().is_empty() {
        eprintln!("skipping ccl differential: CCL_WHATSAPP_ORACLE is empty");
        return;
    }

    let dir = std::env::var_os("CCL_WHATSAPP_DIR").map_or_else(default_fixture_dir, PathBuf::from);
    assert!(
        dir.is_dir(),
        "CCL_WHATSAPP_DIR / fixture is not a directory: {}",
        dir.display()
    );

    let ours = our_live_set(&read_dir(&dir).expect("our decoder reads the indexeddb leveldb dir"));
    let theirs = oracle_live_set(&python, &dir);

    // Real bytes must carry at least one live record, or the differential proves
    // nothing (guards against pointing at an empty/wrong directory).
    assert!(
        !theirs.is_empty(),
        "ccl oracle found no live IndexedDB records in {} — wrong directory?",
        dir.display()
    );

    assert_eq!(
        ours.len(),
        theirs.len(),
        "live record COUNT diverges from ccl_chromium_reader: ours={} theirs={}",
        ours.len(),
        theirs.len(),
    );

    assert_eq!(
        ours,
        theirs,
        "live (object_store, key, decoded-value) sets diverge from ccl_chromium_reader\n\
         ours-only:   {:#?}\n\
         theirs-only: {:#?}",
        ours.difference(&theirs).collect::<Vec<_>>(),
        theirs.difference(&ours).collect::<Vec<_>>(),
    );
}
