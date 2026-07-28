#!/usr/bin/env bash
# Mint a real Chromium IndexedDB `model-storage` store (WhatsApp Web schema) by
# driving headless Google Chrome against scripts/mint/index.html, then copy the
# LevelDB directory into tests/data/. Real Chrome/V8 output = tier-2 oracle.
#
# Usage: scripts/mint/mint.sh
# Re-runnable: it uses a throwaway profile under a temp dir.
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
PORT=8731
ORIGIN_DIR="http_127.0.0.1_${PORT}.indexeddb.leveldb"
DEST="$REPO/tests/data/indexeddb/$ORIGIN_DIR"

PROFILE="$(mktemp -d /tmp/wa-mint-profile.XXXXXX)"
trap 'kill "${SRV_PID:-0}" 2>/dev/null || true; rm -rf "$PROFILE"' EXIT

# Serve index.html on a fixed port so the IndexedDB origin is stable.
( cd "$HERE" && python3 -m http.server "$PORT" >/dev/null 2>&1 ) &
SRV_PID=$!
sleep 1

"$CHROME" --headless=new --disable-gpu --no-first-run \
  --user-data-dir="$PROFILE" --disable-background-networking \
  --disable-component-update --disable-sync --disable-default-apps \
  --disable-domain-reliability --metrics-recording-only \
  "http://127.0.0.1:${PORT}/index.html" >/dev/null 2>&1 &
CHROME_PID=$!

# IndexedDB commits synchronously to the LevelDB .log on transaction commit;
# give Chrome time to load, run the transaction, and flush, then terminate
# gracefully (SIGTERM) so LevelDB closes cleanly.
sleep 10
kill "$CHROME_PID" 2>/dev/null || true
wait "$CHROME_PID" 2>/dev/null || true

SRC="$PROFILE/Default/IndexedDB/$ORIGIN_DIR"
if [ ! -d "$SRC" ]; then
  echo "ERROR: minted store not found at $SRC" >&2
  echo "Chrome profile IndexedDB dir contents:" >&2
  ls -la "$PROFILE/Default/IndexedDB/" >&2 || true
  exit 1
fi

rm -rf "$DEST"
mkdir -p "$DEST"
# Copy only the readable store files (skip LOCK/LOG — not needed to read it).
for f in "$SRC"/*.log "$SRC"/*.ldb "$SRC"/CURRENT "$SRC"/MANIFEST-*; do
  [ -e "$f" ] && cp "$f" "$DEST/"
done

echo "Minted store copied to: $DEST"
ls -la "$DEST"
