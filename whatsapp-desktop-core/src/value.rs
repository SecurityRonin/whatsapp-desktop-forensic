//! Small, total accessors over a decoded [`V8Value`] object graph.
//!
//! Every accessor returns `Option` and never panics — a missing field, a wrong
//! type, or a hole yields `None`, so a malformed or schema-drifted record
//! degrades gracefully rather than crashing (Paranoid-Gatekeeper: distrust the
//! spec, distrust the data).

use chromium_storage_indexeddb::V8Value;

/// The value of object property `name`, if `v` is an object that has it.
#[must_use]
pub(crate) fn field<'a>(v: &'a V8Value, name: &str) -> Option<&'a V8Value> {
    match v {
        V8Value::Object(kv) => kv.iter().find(|(k, _)| k == name).map(|(_, val)| val),
        _ => None,
    }
}

/// `name` as a string, if present and string-typed.
#[must_use]
pub(crate) fn str_field(v: &V8Value, name: &str) -> Option<String> {
    match field(v, name)? {
        V8Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// `name` as an `i64`, accepting both the integer and double V8 encodings.
///
/// A `Double` is accepted only when it is finite and integral (WhatsApp stores
/// `t`/`size`/dimensions as whole numbers); a fractional double yields `None`
/// rather than a silently-truncated integer.
#[must_use]
pub(crate) fn int_field(v: &V8Value, name: &str) -> Option<i64> {
    match field(v, name)? {
        V8Value::Int(i) => Some(*i),
        V8Value::Double(d) if d.is_finite() && d.fract() == 0.0 => Some(*d as i64),
        _ => None,
    }
}

/// `name` as raw bytes, if present and an `ArrayBuffer`.
#[must_use]
pub(crate) fn bytes_field(v: &V8Value, name: &str) -> Option<Vec<u8>> {
    match field(v, name)? {
        V8Value::ArrayBuffer(b) => Some(b.clone()),
        _ => None,
    }
}
