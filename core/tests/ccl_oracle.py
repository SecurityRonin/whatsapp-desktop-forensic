#!/usr/bin/env python3
"""Independent-oracle driver for the whatsapp-desktop ccl differential test.

Reads a Chromium ``*.indexeddb.leveldb`` directory (WhatsApp Desktop's
``model-storage`` store) with the third-party
``cclgroupltd/ccl_chromium_reader`` and emits its *live* view of the
IndexedDB-over-LevelDB records (V8 structured-clone values) on stdout for the
Rust harness (``tests/differential_ccl.rs``) to reconcile against our own
``whatsapp_desktop_core::read_dir`` decode.

Point ``CCL_WHATSAPP_ORACLE`` at a Python interpreter that can
``import ccl_chromium_reader`` (add the checkout to ``PYTHONPATH``):

    PYTHONPATH=/path/to/ccl_chromium_reader \\
    CCL_WHATSAPP_ORACLE=$(which python3) \\
        cargo test -p whatsapp-desktop-core --test differential_ccl

Output is a hex-encoded, tab-separated line stream (hex sidesteps every
delimiter/encoding hazard in real record values):

    REC\t<hex object_store>\t<hex key>\t<hex canon-value>

one per *live* (highest-seq, non-deleted) object-store record. ``canon-value``
is a deterministic, dependency-free canonical encoding of the decoded V8 value
graph, byte-for-byte identical to the Rust side's ``canon`` (so the two decoders
are reconciled on their full decoded values, not just record identity):

    n                       null / undefined
    t / f                   bool
    i<dec>;                 integer (integral numbers, from int OR whole float)
    d<f64-le-bits-hex>;     non-integral double
    s<utf8-byte-len>:<str>  string (length-prefixed => self-delimiting)
    b<hex>;                 bytes / ArrayBuffer
    A<count>:<elems...>     array
    O<count>:<k v k v...>   object, keys sorted by UTF-8 bytes; k encoded as a string
"""

import math
import pathlib
import struct
import sys

from ccl_chromium_reader import ccl_chromium_indexeddb as idb


def _canon(v) -> str:
    """Deterministic canonical encoding; must match the Rust `canon` exactly."""
    if v is None:
        return "n"
    if v is True:
        return "t"
    if v is False:
        return "f"
    if isinstance(v, int):  # bool already handled above
        return f"i{v};"
    if isinstance(v, float):
        if math.isfinite(v) and v == int(v):
            return f"i{int(v)};"
        return "d" + struct.pack("<d", v).hex() + ";"
    if isinstance(v, str):
        return f"s{len(v.encode('utf-8'))}:{v}"
    if isinstance(v, (bytes, bytearray)):
        return "b" + bytes(v).hex() + ";"
    if isinstance(v, (list, tuple)):
        return f"A{len(v)}:" + "".join(_canon(e) for e in v)
    if isinstance(v, dict):
        items = sorted(v.items(), key=lambda kv: kv[0].encode("utf-8"))
        return f"O{len(items)}:" + "".join(_canon(k) + _canon(val) for k, val in items)
    raise TypeError(f"uncanonicalizable {type(v).__name__}: {v!r}")


def _key_identity(key: idb.IdbKey) -> str:
    """Stable string identity for a primary key (string keys => the string)."""
    if isinstance(key.value, str):
        return key.value
    # Non-string keys are not present in the WhatsApp model-storage schema; if
    # one appears, surface it verbatim so the differential mismatches loudly
    # rather than silently coercing.
    return repr(key.value)


def _hex(s: str) -> str:
    return s.encode("utf-8").hex()


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        sys.stderr.write("usage: ccl_oracle.py <indexeddb-leveldb-dir>\n")
        return 2
    leveldb_dir = pathlib.Path(argv[1])
    if not leveldb_dir.is_dir():
        sys.stderr.write(f"not a directory: {leveldb_dir}\n")
        return 2

    wrapper = idb.WrappedIndexDB(leveldb_dir)

    # Collapse every (object_store, key) to its highest-sequence record — the
    # live view. A key whose newest record is a deletion (value is None) is
    # dropped, matching how the Rust side collapses its full tombstone-keeping
    # stream.
    best: dict[tuple[str, str], tuple[int, object]] = {}
    for db_id in wrapper.database_ids:
        db = wrapper[db_id.dbid_no]
        for store in db:
            store_name = store.name
            for rec in store.iterate_records(live_only=False):
                ident = (store_name, _key_identity(rec.key))
                prev = best.get(ident)
                if prev is None or rec.ldb_seq_no > prev[0]:
                    best[ident] = (rec.ldb_seq_no, rec.value)

    try:
        out = []
        for (store_name, key), (_seq, value) in best.items():
            if value is None:
                continue  # newest record is a deletion tombstone
            out.append(
                "REC\t"
                f"{_hex(store_name)}\t{_hex(key)}\t{_hex(_canon(value))}"
            )
    finally:
        wrapper.close()

    sys.stdout.write("\n".join(out))
    if out:
        sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
