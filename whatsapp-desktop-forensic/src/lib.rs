//! WhatsApp Desktop (Electron) **analyzer** — audits a parsed
//! [`whatsapp_desktop_core`] store and emits normalized
//! [`forensicnomicon_core::report`] findings.
//!
//! Findings are **observations, never verdicts**: a recovered deleted message is
//! reported as residue "consistent with" indicator removal, an encrypted body is
//! reported as present (the analyst supplies the key), a media message is
//! reported as an off-store reference. The examiner draws the conclusions.
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use forensicnomicon_core::report::{Category, Evidence, Finding, Observation, Severity, Source};
use std::path::Path;
use whatsapp_desktop_core::{MessageBody, WhatsAppStore};

pub use whatsapp_desktop_core::WaError;

/// Stable finding code: a message body is AES-CBC-encrypted at rest.
pub const CODE_ENCRYPTED_BODY: &str = "WA-MSG-ENCRYPTED-BODY";
/// Stable finding code: a deleted message recovered from a LevelDB tombstone.
pub const CODE_DELETED_RECOVERED: &str = "WA-MSG-DELETED-RECOVERED";
/// Stable finding code: a media message referencing an off-store blob.
pub const CODE_MEDIA_REF: &str = "WA-MSG-MEDIA-REF";

/// Rendered `when` evidence for a message that carries no plausible send time.
const WHEN_UNKNOWN: &str = "unknown";

/// A typed WhatsApp Desktop anomaly. Implements [`Observation`] so the canonical
/// [`Finding`] assembly lives in the shared report model.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WaAnomaly {
    /// A message whose text body is AES-CBC-encrypted (`msgRowOpaqueData`).
    EncryptedMessageBody {
        /// Message id.
        id: String,
        /// `_keyId` naming the derived key needed to decrypt.
        key_id: Option<String>,
        /// Ciphertext length in bytes.
        ciphertext_len: usize,
    },
    /// A deleted message whose content was recovered from a tombstoned put.
    DeletedMessageRecovered {
        /// Message id.
        id: String,
        /// Sender JID, if recovered.
        from: Option<String>,
        /// Recipient JID, if recovered.
        to: Option<String>,
        /// Send time (RFC 3339 UTC), if datable.
        when: Option<String>,
    },
    /// A media message referencing an off-store (CDN) blob.
    MediaReference {
        /// Message id.
        id: String,
        /// Media MIME type.
        mimetype: Option<String>,
        /// CDN direct path to the encrypted blob.
        direct_path: Option<String>,
    },
}

impl Observation for WaAnomaly {
    fn severity(&self) -> Option<Severity> {
        Some(match self {
            WaAnomaly::EncryptedMessageBody { .. } | WaAnomaly::MediaReference { .. } => {
                Severity::Info
            }
            WaAnomaly::DeletedMessageRecovered { .. } => Severity::Medium,
        })
    }

    fn code(&self) -> &'static str {
        match self {
            WaAnomaly::EncryptedMessageBody { .. } => CODE_ENCRYPTED_BODY,
            WaAnomaly::DeletedMessageRecovered { .. } => CODE_DELETED_RECOVERED,
            WaAnomaly::MediaReference { .. } => CODE_MEDIA_REF,
        }
    }

    fn category(&self) -> Category {
        match self {
            // Encryption at rest / off-store pointer are provenance/context.
            WaAnomaly::EncryptedMessageBody { .. } | WaAnomaly::MediaReference { .. } => {
                Category::Provenance
            }
            // Recovered-after-deletion is residue.
            WaAnomaly::DeletedMessageRecovered { .. } => Category::Residue,
        }
    }

    fn note(&self) -> String {
        match self {
            WaAnomaly::EncryptedMessageBody { id, .. } => format!(
                "Message {id} has an AES-CBC-encrypted body (msgRowOpaqueData); the plaintext \
                 requires the derived key and is not present on disk."
            ),
            WaAnomaly::DeletedMessageRecovered { id, .. } => format!(
                "Deleted message {id} was recovered from a LevelDB tombstone over a surviving \
                 record; consistent with indicator removal."
            ),
            WaAnomaly::MediaReference { id, mimetype, .. } => format!(
                "Message {id} references off-store media{}.",
                mimetype
                    .as_deref()
                    .map_or(String::new(), |m| format!(" ({m})"))
            ),
        }
    }

    fn evidence(&self) -> Vec<Evidence> {
        let row = |field: &str, value: String| Evidence {
            field: field.to_string(),
            value,
            location: None,
        };
        match self {
            WaAnomaly::EncryptedMessageBody {
                id,
                key_id,
                ciphertext_len,
            } => vec![
                row("message_id", id.clone()),
                row("key_id", key_id.clone().unwrap_or_default()),
                row("ciphertext_len", ciphertext_len.to_string()),
            ],
            WaAnomaly::DeletedMessageRecovered { id, from, to, when } => vec![
                row("message_id", id.clone()),
                row("from", from.clone().unwrap_or_default()),
                row("to", to.clone().unwrap_or_default()),
                // A send time the record does not plausibly carry is NAMED, not
                // left blank: an empty cell reads as "nothing was there to read",
                // while the recovered record did carry a `t` that is no date.
                row(
                    "when",
                    when.clone().unwrap_or_else(|| WHEN_UNKNOWN.to_string()),
                ),
            ],
            WaAnomaly::MediaReference {
                id,
                mimetype,
                direct_path,
            } => vec![
                row("message_id", id.clone()),
                row("mimetype", mimetype.clone().unwrap_or_default()),
                row("direct_path", direct_path.clone().unwrap_or_default()),
            ],
        }
    }

    fn mitre(&self) -> &'static [&'static str] {
        match self {
            // Message deletion / indicator removal on host.
            WaAnomaly::DeletedMessageRecovered { .. } => &["T1070"],
            _ => &[],
        }
    }
}

/// The [`Source`] stamped on every finding this analyzer emits.
fn source() -> Source {
    Source {
        analyzer: "whatsapp-desktop-forensic".to_string(),
        scope: whatsapp_desktop_core::schema::DB_MODEL_STORAGE.to_string(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    }
}

/// Audit a parsed store, returning normalized findings.
#[must_use]
pub fn audit(store: &WhatsAppStore) -> Vec<Finding> {
    let src = source();
    let mut findings = Vec::new();
    for m in &store.messages {
        if let MessageBody::Encrypted(body) = &m.body {
            findings.push(
                WaAnomaly::EncryptedMessageBody {
                    id: m.id.clone(),
                    key_id: body.key_id.clone(),
                    ciphertext_len: body.ciphertext.len(),
                }
                .to_finding(src.clone()),
            );
        }
        if m.deleted {
            findings.push(
                WaAnomaly::DeletedMessageRecovered {
                    id: m.id.clone(),
                    from: m.from.clone(),
                    to: m.to.clone(),
                    when: m
                        .timestamp_secs
                        .map(whatsapp_desktop_core::epoch_secs_to_rfc3339),
                }
                .to_finding(src.clone()),
            );
        }
        if let Some(media) = &m.media {
            findings.push(
                WaAnomaly::MediaReference {
                    id: m.id.clone(),
                    mimetype: media.mimetype.clone(),
                    direct_path: media.direct_path.clone(),
                }
                .to_finding(src.clone()),
            );
        }
    }
    findings
}

/// Open a `*.indexeddb.leveldb` directory and audit it.
///
/// # Errors
///
/// Returns [`WaError`] if the underlying LevelDB store cannot be read.
pub fn audit_path(dir: &Path) -> Result<Vec<Finding>, WaError> {
    let store = whatsapp_desktop_core::open(dir)?;
    Ok(audit(&store))
}
