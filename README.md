# whatsapp-desktop-forensic

[![Crates.io core](https://img.shields.io/crates/v/whatsapp-desktop-core.svg?label=whatsapp-desktop-core)](https://crates.io/crates/whatsapp-desktop-core)
[![Crates.io forensic](https://img.shields.io/crates/v/whatsapp-desktop-forensic.svg?label=whatsapp-desktop-forensic)](https://crates.io/crates/whatsapp-desktop-forensic)
[![Docs.rs](https://img.shields.io/docsrs/whatsapp-desktop-core?label=docs.rs)](https://docs.rs/whatsapp-desktop-core)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-blue.svg)](https://www.rust-lang.org)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Sponsor](https://img.shields.io/badge/sponsor-h4x0r-ea4aaa.svg)](https://github.com/sponsors/h4x0r)

[![CI](https://github.com/SecurityRonin/whatsapp-desktop-forensic/actions/workflows/ci.yml/badge.svg)](https://github.com/SecurityRonin/whatsapp-desktop-forensic/actions/workflows/ci.yml)
[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-success.svg)](https://github.com/rust-secure-code/safety-dance/)
[![fuzzed](https://img.shields.io/badge/fuzzed-cargo--fuzz-success.svg)](fuzz/)
[![Security advisories clean](https://img.shields.io/badge/advisories-clean-success.svg)](deny.toml)

**Read WhatsApp Desktop (Electron) chats out of the Chromium IndexedDB store —
typed messages, chats, contacts, media, and a timeline — with encrypted bodies
surfaced, never faked.**

WhatsApp's Electron/PWA desktop client keeps its chats in a Chromium
IndexedDB-over-LevelDB database (`model-storage`) whose values are Blink/V8
structured-clone blobs. This crate interprets that store into typed forensic
records and audits them.

## Above the fold

```rust
use whatsapp_desktop_forensic::audit_path;

// Point at the app's `…model-storage` IndexedDB LevelDB directory.
let findings = audit_path("Default/IndexedDB/…indexeddb.leveldb".as_ref())?;
for f in &findings {
    println!("[{:?}] {} — {}", f.severity, f.code, f.note);
}
// [Some(Medium)] WA-MSG-DELETED-RECOVERED — Deleted message … recovered from a tombstone …
// [Some(Info)]   WA-MSG-ENCRYPTED-BODY    — Message … has an AES-CBC-encrypted body …
# Ok::<(), whatsapp_desktop_forensic::WaError>(())
```

Need the records, not findings? Use the reader:

```rust
use whatsapp_desktop_core::open;

let store = open("…indexeddb.leveldb".as_ref())?;
println!("{} messages, {} chats, {} contacts", store.messages.len(), store.chats.len(), store.contacts.len());
for e in &store.timeline {
    println!("{}  {}  {}", e.rfc3339, if e.deleted { "[deleted]" } else { "" }, e.message_id);
}
# Ok::<(), whatsapp_desktop_core::WaError>(())
```

## The two crates

| Crate | Role |
|---|---|
| `whatsapp-desktop-core` | **Reader** — typed `Message`/`Chat`/`Contact`/`MediaMeta` + timeline, and the audited AES-CBC body-decryption primitive. No findings. |
| `whatsapp-desktop-forensic` | **Analyzer** — normalized `forensicnomicon::report::Finding`s via `impl Observation`. |

It sits on top of the fleet's Chromium storage readers (`chromium-storage-indexeddb`
→ `blob-decoder`) and reuses forensicnomicon's IndexedDB key-coding; it never
re-implements LevelDB or V8 decoding.

## What it surfaces

| Code | Severity | Meaning |
|---|---|---|
| `WA-MSG-DELETED-RECOVERED` | Medium | A deleted message recovered from a LevelDB tombstone (consistent with `T1070`). |
| `WA-MSG-ENCRYPTED-BODY` | Info | A message body is AES-CBC-encrypted (`msgRowOpaqueData`); the plaintext needs the derived key. |
| `WA-MSG-MEDIA-REF` | Info | A media message referencing an off-store (CDN) blob. |

## The message body is encrypted — and never fabricated

WhatsApp Web AES-CBC-encrypts a text body into `msgRowOpaqueData:{_keyId, iv,
_data}`. This crate reports the encrypted envelope as
`MessageBody::Encrypted{ key_id, iv, ciphertext }` verbatim. `decrypt_body` (audited
RustCrypto AES-CBC) decrypts it **only when you supply the derived key**; a wrong or
missing key returns a typed `WaError`, it does not invent plaintext. Deriving the key
itself is out of scope — see [`docs/validation.md`](docs/validation.md).

## Trust, but verify

- **Fuzzed** — `cargo-fuzz` `parse_store` (646k execs) and `decrypt_body` (1.77M
  execs) locally, zero crashes; run weekly in CI.
- **Panic-free by lint** — `#![forbid(unsafe_code)]`, `unwrap_used`/`expect_used`
  denied in production code; total accessors over the decoded V8 graph.
- **Validated against real Chromium output** — extraction over a real minted
  `model-storage` store (tier-2), AES-CBC decryption over independent openssl KAT
  vectors (tier-1), the epoch→RFC 3339 conversion against Python `datetime` (tier-1).
  Full evidence + the honest boundary (no real message corpus yet) in
  [`docs/validation.md`](docs/validation.md).

---

[Privacy Policy](https://securityronin.github.io/whatsapp-desktop-forensic/privacy/) · [Terms of Service](https://securityronin.github.io/whatsapp-desktop-forensic/terms/) · © 2026 Security Ronin Ltd
