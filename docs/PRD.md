# Purpose & Scope — whatsapp-desktop-forensic

A library repo (PARSER tier). This is the lighter **Purpose & Scope** form of the
gated doc (ADR-0015), not a product PRD.

## Purpose

Interpret a **WhatsApp Desktop (Electron/PWA)** installation's Chromium
IndexedDB-over-LevelDB `model-storage` store into typed forensic records — chats,
messages, contacts, media references — plus a merged timeline, and audit them for
encrypted bodies, recovered deleted messages, and media references. It sits on top
of the Wave-2 Chromium storage readers and reuses forensicnomicon's IndexedDB
key-coding; it never re-implements LevelDB or V8 decoding.

## Audience

DFIR analysts and the Issen orchestrator. The `-forensic` analyzer is the
analyst-facing entry point (`audit_path` → graded findings); the `-core` reader is
for tools that want the typed records directly.

## Scope

**In scope**

- Parse `message`/`chat`/`contact` object stores into typed records
  (`whatsapp-desktop-core`).
- Extract media reference metadata from media messages.
- Deduplicate by primary key and recover deleted messages from LevelDB tombstones.
- Build a time-ordered message timeline.
- Surface the AES-CBC-encrypted message body as a typed envelope, never fabricated;
  decrypt it (`decrypt_body`) only when the caller supplies the derived key.
- Emit normalized `forensicnomicon::report` findings (`whatsapp-desktop-forensic`).

**Out of scope**

- The **native** WhatsApp Desktop client (Windows UWP/WebView2, macOS Catalyst),
  which uses SEE/DPAPI-encrypted SQLite — a different artifact (see
  `forensicnomicon_core::messenger_desktop`), a separate parser.
- Deriving the message encryption key (HKDF over unavailable key material) — see
  `docs/validation.md`.
- LevelDB / IndexedDB / V8 decoding (owned by the Wave-2 reader crates).
- Media blob download/decryption.

## Crate structure

- `whatsapp-desktop-core` — the reader: typed records + timeline + the AES-CBC
  decryption primitive. No findings.
- `whatsapp-desktop-forensic` — the analyzer: `forensicnomicon::report::Finding`s
  via `impl Observation`.

See `docs/decisions/` for the load-bearing decisions and `docs/validation.md` for
the evidence.
