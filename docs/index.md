# whatsapp-desktop-forensic

Forensic reader + analyzer for **WhatsApp Desktop (Electron)** — interprets the
Chromium IndexedDB-over-LevelDB `model-storage` store into typed chats, messages,
contacts, and media references, plus a merged timeline, and audits them for
encrypted bodies, recovered deleted messages, and media references.

- **`whatsapp-desktop-core`** — the reader: typed records + timeline + an audited
  AES-CBC body-decryption primitive.
- **`whatsapp-desktop-forensic`** — the analyzer: normalized
  `forensicnomicon::report` findings.

```rust
use whatsapp_desktop_forensic::audit_path;

let findings = audit_path("Default/IndexedDB/…model-storage.indexeddb.leveldb".as_ref())?;
for f in &findings {
    println!("[{:?}] {} — {}", f.severity, f.code, f.note);
}
# Ok::<(), whatsapp_desktop_forensic::WaError>(())
```

The message body is AES-CBC-encrypted at rest; this tool surfaces the encrypted
envelope and **never fabricates plaintext**. See [Validation](validation.md) for
the evidence and the crypto boundary.

- [Validation](validation.md) · [Product Requirements](PRD.md)
- [Privacy Policy](privacy.md) · [Terms of Service](terms.md)
