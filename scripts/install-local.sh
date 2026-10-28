#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX="${HOME}/.local"
APP_ID="io.github.SakibShahariar.CoprHub"

cd "$ROOT"
cargo build --release

install -Dm755 "target/release/copr-hub" "$PREFIX/bin/copr-hub"
install -Dm644 "data/${APP_ID}.desktop" "$PREFIX/share/applications/${APP_ID}.desktop"
install -Dm644 "data/${APP_ID}.metainfo.xml" "$PREFIX/share/metainfo/${APP_ID}.metainfo.xml"
install -Dm644 "data/icons/hicolor/scalable/apps/${APP_ID}.svg" "$PREFIX/share/icons/hicolor/scalable/apps/${APP_ID}.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$PREFIX/share/applications" >/dev/null 2>&1 || true
fi

printf 'Copr Hub installed for %s. Ensure %s/bin is on PATH.\n' "$USER" "$PREFIX"
