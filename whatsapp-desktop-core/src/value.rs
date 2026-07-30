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

/// `name` as a Unix-epoch **seconds** event time, or `None` when it is absent,
/// wrong-typed, or the zero sentinel.
///
/// See [`is_datable`] for why zero is not a time.
#[must_use]
pub(crate) fn epoch_secs_field(v: &V8Value, name: &str) -> Option<i64> {
    int_field(v, name).filter(|&t| is_datable(t))
}

/// `true` when `t` can be placed on a timeline.
///
/// Zero is WhatsApp's absent/unset time, not 1970-01-01T00:00:00Z: no message was
/// ever sent at the Unix epoch, so reading the sentinel as a real time fabricates
/// a 1970 event. Every timestamp path applies this — the reader ([`epoch_secs_field`])
/// and [`crate::TimelineEntry::build_timeline`], so a caller who builds a
/// [`crate::Message`] by hand cannot reintroduce the 1970 row either.
#[must_use]
pub(crate) fn is_datable(t: i64) -> bool {
    t != 0
}

/// `name` as raw bytes, if present and an `ArrayBuffer`.
#[must_use]
pub(crate) fn bytes_field(v: &V8Value, name: &str) -> Option<Vec<u8>> {
    match field(v, name)? {
        V8Value::ArrayBuffer(b) => Some(b.clone()),
        _ => None,
    }
}
