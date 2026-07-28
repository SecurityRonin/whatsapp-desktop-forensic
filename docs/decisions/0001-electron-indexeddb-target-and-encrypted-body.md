# 0001 — Target the Electron IndexedDB store; surface (never fabricate) the encrypted body

Status: accepted

## Context

"WhatsApp Desktop" is two different products with two different on-disk shapes:

1. the **native** client (Windows UWP/WebView2, macOS Catalyst) — SEE/DPAPI-
   encrypted SQLite, which `forensicnomicon_core::messenger_desktop` already models
   (`AppKind::Native`, `genericStorageDB`/`ChatStorage.sqlite`); and
2. the **Electron/PWA** client — the WhatsApp Web bundle in a Chromium wrapper,
   whose chats live in a Chromium **IndexedDB-over-LevelDB** database
   `model-storage`.

This repo targets **(2)**. The two are not interchangeable: on the host used to
build this, the installed app was the *native* macOS client (Core Data SQLite),
which carries none of the IndexedDB artifacts this parser reads.

The message **body is encrypted**: WhatsApp Web AES-CBC-encrypts a text message's
body into `msgRowOpaqueData:{_keyId, iv, _data}`; only the metadata (`id`, `t`,
`from`, `to`, `type`, media fields) is plaintext (Mazzoli; MDPI Future Internet
12(11):184).

## Decision

- **Sit on top of the Wave-2 Chromium storage readers.** `whatsapp-desktop-core`
  consumes `chromium_storage_indexeddb` (which walks the LevelDB key-coding — reused
  from `forensicnomicon_core::chromium_indexeddb` — and decodes each value's V8
  graph). This crate never re-implements LevelDB or V8, and never re-hardcodes a
  forensicnomicon constant.
- **Own the WhatsApp Web *schema* names here.** The object-store and field names
  (`model-storage`, `message`/`chat`/`contact`, `msgRowOpaqueData`, media fields)
  are this parser's domain knowledge, cited in `src/schema.rs`. forensicnomicon
  models WhatsApp as native SQLite and does not carry the Web IndexedDB schema, so
  this is new knowledge, not duplication.
- **Surface the encrypted body verbatim; never fabricate plaintext.** A message's
  body is a typed `MessageBody::Encrypted{ key_id, iv, ciphertext }` or
  `MessageBody::None` — never invented text. Decryption (`crypto::decrypt_body`) is
  audited RustCrypto AES-CBC, KAT-validated, and fails loud on a wrong/missing key.
- **Defer key *derivation*.** Deriving the AES key (HKDF-SHA256 over the
  `wawc_db_enc` master key + the `WebEncKeySalt` localStorage salt) is out of scope
  until real key material is available to validate it against an oracle; the caller
  supplies the derived key. (See `docs/validation.md`.)
- **Recover deleted messages from LevelDB tombstones.** The raw record stream keeps
  superseded puts and delete tombstones; the aggregate parser collapses each key to
  its latest state, flags it deleted when the newest record is a tombstone, and
  recovers the content from the newest surviving put.

## Consequences

- The parser is medium-agnostic (PARSER-tier rule): it accepts a `Path`/records and
  never learns where the bytes came from.
- Correctness of *extraction* is bounded at tier-2 (a real Chrome-minted store in
  the documented schema) until a real corpus is available; crypto and the timestamp
  conversion are tier-1 (KAT). See `docs/validation.md`.
- If a real message corpus later surfaces, only the schema field set and the
  (currently deferred) key-derivation glue need revisiting — the reader/analyzer
  split and the encrypted-body handling do not.
