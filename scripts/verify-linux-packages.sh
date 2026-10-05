#!/usr/bin/env bash
set -euo pipefail

bundle_dir=$(realpath "${1:?Expected Linux bundle directory}")
previous_packages=${2:-}
previous_revision=${3:-}
script_dir=$(cd -- "$(dirname -- "$0")" && pwd)
verification_root=$(mktemp -d)
trap 'rm -rf "$verification_root"' EXIT
deb_package=$(rg --files "$bundle_dir/deb" -g '*.deb')
appimage_package=$(rg --files "$bundle_dir/appimage" -g '*.AppImage')
test -f "$deb_package"
test -f "$appimage_package"

dpkg-deb --extract "$deb_package" "$verification_root/deb"
desktop_path="$verification_root/deb/usr/share/applications/Socks Proxy.desktop"
desktop-file-validate "$desktop_path"
appstreamcli validate --no-net "$verification_root/deb/usr/share/metainfo/com.socksproxy.desktop.metainfo.xml"
rg -q '^Name\[zh_CN\]=Socks Proxy 代理管理工具$' "$desktop_path"
rg -q '^Categories=Network;Settings;$' "$desktop_path"
rg -q '^Exec=socks-proxy$' "$desktop_path"
test -f "$verification_root/deb/usr/share/icons/hicolor/512x512/apps/socks-proxy.png"

export XDG_DATA_HOME="$verification_root/user-data"
export XDG_CONFIG_HOME="$verification_root/user-config"
export XDG_CACHE_HOME="$verification_root/user-cache"
mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"

verify_startup() {
  local label=$1
  shift
  local status=0
  # Each launch must produce its own acknowledgment, including upgrade launches.
  rm -rf "$XDG_DATA_HOME/com.socksproxy.desktop/logs"
  timeout --signal=TERM 20s dbus-run-session -- xvfb-run -a "$@" > "$verification_root/$label.log" 2>&1 || status=$?
  if [ "$status" -ne 124 ]; then
    cat "$verification_root/$label.log"
    echo "$label exited before the startup observation completed (status $status)" >&2
    exit 1
  fi
  rg -q 'startup_frontend_ready' "$XDG_DATA_HOME" "$verification_root/$label.log"
  echo "$label: native frontend readiness confirmed"
}

configuration="$XDG_DATA_HOME/com.socksproxy.desktop/config.sqlite3"
if [ -n "$previous_packages" ]; then
  test "$(cat "$previous_packages/SOURCE_REVISION.txt" | tr -d '\r\357\273\277')" = "$previous_revision"
  (cd "$previous_packages" && sha256sum --check SHA256SUMS.txt)
  previous_deb=$(rg --files "$previous_packages" -g '*.deb')
  old_version=$(dpkg-deb --field "$previous_deb" Version)
  test "$(cat "$previous_packages/VERSION.txt" | tr -d '\r\357\273\277')" = "$old_version"
  new_version=$(dpkg-deb --field "$deb_package" Version)
  dpkg --compare-versions "$new_version" gt "$old_version"
  sudo dpkg --install "$previous_deb"
  verify_startup previous-deb socks-proxy
  node "$script_dir/linux-package-state.mjs" seed "$configuration" "$verification_root/previous-state.json"
  verify_startup previous-deb-with-configuration socks-proxy
  node "$script_dir/linux-package-state.mjs" verify "$configuration" "$verification_root/previous-state.json"
fi

sudo dpkg --install "$deb_package"
desktop-file-validate '/usr/share/applications/Socks Proxy.desktop'
command -v socks-proxy
verify_startup deb socks-proxy
test -f "$configuration"
if [ -n "$previous_packages" ]; then
  node "$script_dir/linux-package-state.mjs" verify "$configuration" "$verification_root/previous-state.json"
  echo "DEB upgrade $old_version -> $new_version preserves nonempty configuration and selected mode"
fi
configuration_hash=$(sha256sum "$configuration" | cut -d ' ' -f 1)
sudo dpkg --remove socks-proxy
test ! -f '/usr/share/applications/Socks Proxy.desktop'
test "$configuration_hash" = "$(sha256sum "$configuration" | cut -d ' ' -f 1)"
echo 'DEB uninstall removes its desktop entry and preserves the user configuration'

# AppImage starts with fresh state so a previous DEB acknowledgment cannot pass it.
export XDG_DATA_HOME="$verification_root/appimage-data"
export XDG_CONFIG_HOME="$verification_root/appimage-config"
export XDG_CACHE_HOME="$verification_root/appimage-cache"
mkdir -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"
chmod +x "$appimage_package"
verify_startup appimage "$appimage_package"
echo 'Linux package metadata, installation, native startup and DEB uninstall checks passed'
