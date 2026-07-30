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
/// wrong-typed, or outside the plausible range.
///
/// See [`is_datable`] for the range and why values outside it are not times.
#[must_use]
pub(crate) fn epoch_secs_field(v: &V8Value, name: &str) -> Option<i64> {
    int_field(v, name).filter(|&t| is_datable(t))
}

/// Earliest plausible WhatsApp event time — 2009-01-01T00:00:00Z.
///
/// WhatsApp Inc. was founded in February 2009, so no genuine `t` predates 2009;
/// the year boundary is the conservative round floor just below the earliest
/// message the application could have produced.
const EARLIEST_PLAUSIBLE_SECS: i64 = 1_230_768_000;

/// Latest plausible WhatsApp event time — 2100-01-01T00:00:00Z.
///
/// Deliberately far beyond "now": a device whose clock is skewed or set forward
/// produces a genuine record with a future `t`, and that record is evidence — the
/// ceiling must not discard it. It rejects only the absurd, most usefully a
/// millisecond value read as seconds (1.6e12 s lands in year 52560), which would
/// otherwise sort to the far end of every timeline as a fabricated event.
const LATEST_PLAUSIBLE_SECS: i64 = 4_102_444_800;

/// `true` when `t` can be placed on a timeline.
///
/// A time outside `[2009-01-01, 2100-01-01)` is not a WhatsApp send time — most
/// often the `0` unset sentinel, but equally a negative value (which renders as a
/// pre-1970 date) or a magnitude from a mis-scaled/mis-parsed field. Reading any
/// of them as a real time fabricates an event on a forensic timeline, so they
/// yield **no** timestamp rather than a wrong one. A rejected value is not lost:
/// it stays in the record as read, it merely does not become a date.
///
/// Every timestamp path applies this — the reader ([`epoch_secs_field`]) and
/// [`crate::TimelineEntry::build_timeline`], so a caller who builds a
/// [`crate::Message`] by hand cannot reintroduce the fabricated row either.
#[must_use]
pub(crate) fn is_datable(t: i64) -> bool {
    (EARLIEST_PLAUSIBLE_SECS..LATEST_PLAUSIBLE_SECS).contains(&t)
}

/// `name` as raw bytes, if present and an `ArrayBuffer`.
#[must_use]
pub(crate) fn bytes_field(v: &V8Value, name: &str) -> Option<Vec<u8>> {
    match field(v, name)? {
        V8Value::ArrayBuffer(b) => Some(b.clone()),
        _ => None,
    }
}
