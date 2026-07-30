//! A merged, time-ordered message timeline.

use crate::message::Message;
use crate::value::is_datable;
use serde::Serialize;

/// One event in the message timeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TimelineEntry {
    /// Event time, seconds since the Unix epoch (`t`).
    pub timestamp_secs: i64,
    /// `timestamp_secs` rendered as RFC 3339 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
    pub rfc3339: String,
    /// The message id.
    pub message_id: String,
    /// Sender JID.
    pub from: Option<String>,
    /// Recipient JID.
    pub to: Option<String>,
    /// Message type.
    pub kind: Option<String>,
    /// `true` if the message was recovered from a deletion tombstone.
    pub deleted: bool,
}

impl TimelineEntry {
    /// Build a timeline from messages, ascending by time then id. Messages with
    /// no datable `t` are omitted: they cannot be placed on a timeline. A `t`
    /// outside the plausible range — the zero unset sentinel, a negative value, an
    /// absurd magnitude — is not datable either (see `crate::value::is_datable`).
    #[must_use]
    pub fn build_timeline(messages: &[Message]) -> Vec<TimelineEntry> {
        let mut entries: Vec<TimelineEntry> = messages
            .iter()
            .filter_map(|m| {
                let t = m.timestamp_secs.filter(|&t| is_datable(t))?;
                Some(TimelineEntry {
                    timestamp_secs: t,
                    rfc3339: epoch_secs_to_rfc3339(t),
                    message_id: m.id.clone(),
                    from: m.from.clone(),
                    to: m.to.clone(),
                    kind: m.kind.clone(),
                    deleted: m.deleted,
                })
            })
            .collect();
        entries.sort_by(|a, b| {
            a.timestamp_secs
                .cmp(&b.timestamp_secs)
                .then_with(|| a.message_id.cmp(&b.message_id))
        });
        entries
    }
}

/// Render Unix epoch **seconds** as RFC 3339 UTC (`YYYY-MM-DDTHH:MM:SSZ`).
///
/// Pure calendar arithmetic (no dependency), correct for negative (pre-1970)
/// epochs via Euclidean division. Uses Howard Hinnant's `civil_from_days`
/// algorithm (public domain): <https://howardhinnant.github.io/date_algorithms.html>.
#[must_use]
pub fn epoch_secs_to_rfc3339(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (hh, mm, ss) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// Convert a count of days since 1970-01-01 to a `(year, month, day)` civil date.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}
