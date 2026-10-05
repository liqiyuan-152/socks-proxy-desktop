#!/usr/bin/env bash
set -euo pipefail

# Tauri skips Finder layout in CI unless TAURI_BUNDLER_DMG_IGNORE_CI=true.
# Refuse a DMG that contains artwork but cannot display its installation guide.
bundle_dir="${1:?Usage: verify-macos-dmg.sh <dmg-directory>}"
mount_dir="$(mktemp -d "${TMPDIR:-/tmp}/socks-dmg-check.XXXXXX")"
mounted=0
cleanup() {
  if [[ "$mounted" == 1 ]]; then
    hdiutil detach "$mount_dir" >/dev/null
  fi
  rmdir "$mount_dir"
}
trap cleanup EXIT

shopt -s nullglob
packages=("$bundle_dir"/*.dmg)
[[ ${#packages[@]} == 1 ]] || { echo 'Expected exactly one DMG' >&2; exit 1; }
hdiutil attach "${packages[0]}" -readonly -nobrowse -mountpoint "$mount_dir" >/dev/null
mounted=1
[[ -s "$mount_dir/.DS_Store" ]] || { echo 'DMG Finder layout is missing' >&2; exit 1; }
[[ -L "$mount_dir/Applications" ]] || { echo 'Applications link is missing' >&2; exit 1; }
[[ "$(readlink "$mount_dir/Applications")" == '/Applications' ]] || exit 1
[[ -d "$mount_dir/Socks Proxy.app" ]] || exit 1
node --input-type=module - "$mount_dir" <<'JS'
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
const root = process.argv[2];
const layout = readFileSync(join(root, '.DS_Store'));
for (const record of ['icvp', 'Iloc']) {
  if (!layout.includes(Buffer.from(record))) throw new Error(`Missing Finder ${record} record`);
}
const backgrounds = readdirSync(join(root, '.background'));
if (backgrounds.length === 0) throw new Error('Missing DMG background');
console.log('DMG Finder layout, icon locations, application and background resources verified');
JS
