#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
binary_dir="$HOME/.local/bin"
data_dir="${XDG_DATA_HOME:-$HOME/.local/share}"
applications_dir="$data_dir/applications"
icons_root="$data_dir/icons/hicolor"
desktop_dir="$(xdg-user-dir DESKTOP 2>/dev/null || printf '%s/Desktop' "$HOME")"

cd "$project_dir"
cargo build --release --locked -j 6

install -Dm755 target/release/openvpn-gpui "$binary_dir/openvpn-gpui"
mkdir -p "$applications_dir" "$desktop_dir"

rm -f "$icons_root/scalable/apps/dev.kxviel.OpenVPN.svg"
for icon_size in 16 32 48 64 128 256 512; do
    install -Dm644 \
        "assets/icons/app-icon-${icon_size}.png" \
        "$icons_root/${icon_size}x${icon_size}/apps/dev.kxviel.OpenVPN.png"
done

cat > "$applications_dir/dev.kxviel.OpenVPN.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=OpenVPN
Comment=Connect to your work VPN with saved profiles
Exec="$binary_dir/openvpn-gpui"
Icon=dev.kxviel.OpenVPN
Terminal=false
Categories=Network;RemoteAccess;
Keywords=VPN;OpenVPN;DFKI;
StartupNotify=true
StartupWMClass=dev.kxviel.OpenVPN
EOF

install -m755 "$applications_dir/dev.kxviel.OpenVPN.desktop" "$desktop_dir/OpenVPN.desktop"
if command -v gio >/dev/null 2>&1; then
    gio set "$desktop_dir/OpenVPN.desktop" metadata::trusted true >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$applications_dir"
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$icons_root" >/dev/null 2>&1 || true
fi
if command -v kbuildsycoca6 >/dev/null 2>&1; then
    kbuildsycoca6 >/dev/null 2>&1 || true
fi

printf 'Installed OpenVPN to %s\nDesktop launcher: %s\n' "$binary_dir/openvpn-gpui" "$desktop_dir/OpenVPN.desktop"
