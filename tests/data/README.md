# Test data provenance

Cross-referenced from the fleet catalog `ronin-issen/docs/test-data-catalog.md`
(the single machine-index); this file is the co-located human-facing detail.

## `indexeddb/http_127.0.0.1_8731.indexeddb.leveldb/`

- **Classification:** `REAL-self` (real Chromium engine output, self-driven
  scenario) — a **tier-2** oracle (Chrome/V8-authored bytes in WhatsApp Web's
  documented schema; the writes below are the ground truth). This is **not** a
  real message corpus (none was available on the host — the installed WhatsApp is
  the native macOS Core Data/SQLite client, a different artifact). See
  `docs/validation.md` for what would upgrade this to tier-1.
- **Source:** minted on this host by driving headless Google Chrome 150 against
  `scripts/mint/index.html`, then copying the profile's IndexedDB LevelDB
  directory. Generator (verbatim): **`scripts/mint/mint.sh`**.
- **Schema authority (documented WhatsApp Web IndexedDB layout):**
  - F. Mazzoli, *Backing up WhatsApp data through the multi-device web client* —
    <https://mazzo.li/posts/whatsapp-backup.html> (database `model-storage`;
    object stores `message`/`chat`/`contact`; the encrypted
    `msgRowOpaqueData:{_keyId, iv, _data}` message body; media fields).
  - F. Paligo et al., *Browser Forensic Investigations of WhatsApp Web Utilizing
    IndexedDB Persistent Storage*, Future Internet 12(11):184, MDPI, 2020 —
    <https://www.mdpi.com/1999-5903/12/11/184>.
- **Ground-truth writes** (IndexedDB database `model-storage`):
  - `message` store: a text message (id `false_…_3EB0A1B2C3D4E5F6`, `t`
    1596233451) whose body is AES-CBC-encrypted inside `msgRowOpaqueData`
    (`iv` = 16 bytes `00..0F`, `_data` = 32 bytes); an image message (id
    `true_…_ABCDEF0123456789`, `t` 1596233500) with `mimetype`/`filehash`/
    `mediaKey`/`directPath`/`size`/`width`/`height`; and a message
    (id `false_…_DELETEDMSG000001`) that is `put` then `delete`d — leaving a
    LevelDB tombstone over a recoverable content record.
  - `chat` store: `{id:"15551239999@c.us", name:"Alice", t:1596233500,
    unreadCount:2}`.
  - `contact` store: `{id:"15551239999@c.us", name:"Alice Example",
    pushname:"Alice", notifyName:"Alice", shortName:"Alice"}`.
- **Files:** `000003.log`, `CURRENT`, `MANIFEST-000001` (the `LOCK`/`LOG` files
  are excluded — not needed to read the store). MD5:
  - `000003.log` = `6fd64ebd0191eaec3d2acc75da0dbcf9`
  - `CURRENT` = `46295cac801e5d4857d09837238a6394`
  - `MANIFEST-000001` = `3fd11ff447c1ee23538dc4d9724427a3`
- **Consumed by:** `whatsapp-desktop-core/tests/store.rs` (record extraction +
  deleted-message recovery), `whatsapp-desktop-forensic/tests/` (findings), and
  `whatsapp-desktop-core/tests/differential_ccl.rs` (tier-1 differential of the
  Chromium IndexedDB/V8 decode against `cclgroupltd/ccl_chromium_reader`;
  env-gated on `CCL_WHATSAPP_ORACLE`).
- **Re-mint:** `bash scripts/mint/mint.sh` (overwrites this directory). The exact
  bytes differ per run (LevelDB sequence numbers, origin file names) but the
  decoded records are stable; tests assert on decoded content, not raw bytes.
