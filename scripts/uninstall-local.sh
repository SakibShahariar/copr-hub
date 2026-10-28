#!/bin/sh
set -eu

PREFIX="${HOME}/.local"
APP_ID="io.github.SakibShahariar.CoprHub"

rm -f "$PREFIX/bin/copr-hub"
rm -f "$PREFIX/share/applications/${APP_ID}.desktop"
rm -f "$PREFIX/share/metainfo/${APP_ID}.metainfo.xml"
rm -f "$PREFIX/share/icons/hicolor/scalable/apps/${APP_ID}.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$PREFIX/share/applications" >/dev/null 2>&1 || true
fi

printf 'Copr Hub launcher and application metadata removed. User data was preserved.\n'
