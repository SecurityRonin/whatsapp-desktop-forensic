# Validation

How each claim in this repo is checked, and by whom — stated by tier so nothing
reads as more validated than it is (Evidence-Based Rigor).

## Summary

| Path | Tier | Oracle |
|---|---|---|
| Record extraction (message/chat/contact/media) | **T2** | Real Chromium/V8 bytes minted in WhatsApp Web's documented schema |
| Deleted-message recovery + dedup | **T2** | Same minted store (a real `put`→`delete` LevelDB tombstone) |
| Timeline `epoch → RFC 3339` | **T1** | Independent Python `datetime` (UTC) Known-Answer-Test |
| AES-CBC body decryption | **T1** | Independent openssl `enc -aes-*-cbc` PKCS7 KAT vectors |
| Panic-freedom on hostile input | fuzz | `cargo-fuzz` (`parse_store` 646k execs, `decrypt_body` 1.77M execs, 0 crashes) |

## The oracle — which honesty-rule step yielded the data

The parser was validated by the honesty procedure, stopping at the first step that
yielded usable data:

1. **App installed on this host?** Yes — but the installed WhatsApp is the
   **native macOS client** (`net.whatsapp.WhatsApp`, a Core Data / SQLite store:
   `ChatStorage.sqlite` under `~/Library/Group Containers/…WhatsApp.shared`). That
   is a **different artifact** from the Electron/Chromium IndexedDB store this
   parser targets, and it is the account owner's private data — it was **not**
   copied or used.
2. **Public DFIR/DLEAPP sample?** No public WhatsApp *Web/Electron* IndexedDB
   (`model-storage`) LevelDB sample with provenance was located.
3. **Mint a real Chromium store in the documented schema.** ← **used.** Headless
   Google Chrome 150 was driven (`scripts/mint/`) to write a real IndexedDB
   `model-storage` database with `message`/`chat`/`contact` records shaped per the
   published WhatsApp Web schema, then the LevelDB directory was copied into
   `tests/data/`. These are genuine Chrome/V8-authored bytes → **tier-2** (a
   self-constructed *scenario*, but not self-encoded values).

**No real WhatsApp message corpus was available.** The extraction path's ceiling
is therefore T2: the *scenario* (which fields, which records) was chosen here, so
it can miss real-world quirks a genuine account would show (schema drift across
WhatsApp versions, group-metadata shapes, reactions/edits, multi-device
artifacts).

### What would upgrade extraction to T1

- A real WhatsApp Web/Electron `model-storage` LevelDB from a consenting account
  or a published third-party forensic corpus (independent author + ground truth),
  reconciled field-by-field.
- A differential against an independent WhatsApp Web IndexedDB parser
  (e.g. a DLEAPP/ALEAPP module) over the same store, reconciling record counts and
  contents.

## Crypto scope and boundary

`decrypt_body` is **audited RustCrypto AES-CBC** (PKCS7), validated at **T1**
against openssl-generated KAT vectors for both AES-128 and AES-256; a wrong key,
bad key length, or bad IV length returns a typed `WaError` — it never fabricates
plaintext (`tests/crypto.rs`).

**Deliberately out of scope:** deriving the message key itself. WhatsApp Web
derives the AES-CBC key via HKDF-SHA256 over a master key in the `wawc_db_enc`
IndexedDB `keys` store plus the `WebEncKeySalt` from localStorage. No real key
material was available to validate that derivation wiring against an oracle, and
shipping unverified crypto glue is exactly the failure the discipline forbids. The
parser therefore **surfaces the encrypted envelope** (`MessageBody::Encrypted{
key_id, iv, ciphertext }`) and lets the caller supply the derived key. Adding the
derivation requires a real key + ciphertext pair to KAT against.

## Reproducing

- Tests: `cargo test --workspace` (reads the committed minted store in place).
- Re-mint the oracle: `bash scripts/mint/mint.sh` (needs Google Chrome; overwrites
  `tests/data/indexeddb/…`). Decoded records are stable across runs; raw LevelDB
  bytes (sequence numbers, file names) are not, so tests assert on decoded content.
- Fuzz: `cd fuzz && cargo +nightly fuzz run parse_store` / `decrypt_body`.

## Open scaffolding

- `supply-chain/` imports the aggregate audit sets and declares our own crates
  first-party; the per-version `[[exemptions]]` block must be generated with
  `cargo vet` before turning on a vet CI gate.
- Coverage gate (Codecov) is wired in CI; a hard floor should be set once the
  first run establishes the baseline.
