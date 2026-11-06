#!/usr/bin/env bash
set -e

# 1. Pastikan dijalankan sebagai root (sudo)
if [ "$EUID" -ne 0 ]; then
    echo "Error: uninstall.sh must be run as root (e.g. sudo bash packaging/uninstall.sh)" >&2
    exit 1
fi

echo "Uninstalling ncharge..."

# 2. Stop dan disable systemd service
if systemctl is-enabled --quiet ncharge-apply.service 2>/dev/null; then
    systemctl disable ncharge-apply.service 2>/dev/null || true
fi
rm -f /etc/systemd/system/ncharge-apply.service
systemctl daemon-reload
echo "✓ Removed systemd service"

# 3. Hapus sleep hook
rm -f /usr/lib/systemd/system-sleep/ncharge
echo "✓ Removed sleep hook"

# 4. Hapus udev rule jika ada
if [ -f /etc/udev/rules.d/99-ncharge.rules ]; then
    rm -f /etc/udev/rules.d/99-ncharge.rules
    udevadm control --reload-rules 2>/dev/null || true
    echo "✓ Removed udev rule"
fi

# 5. Hapus binaries
rm -f /usr/local/bin/ncharge
rm -f /usr/local/bin/ncharge-apply
echo "✓ Removed /usr/local/bin/ncharge and ncharge-apply"

# 6. Hapus desktop entry & icon
rm -f /usr/share/applications/io.github.ncharge.desktop
rm -f /usr/share/icons/hicolor/scalable/apps/io.github.ncharge.svg

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
echo "✓ Removed desktop entry and icon"

# 7. Hapus temporary override
rm -f /run/ncharge.override

# 8. Konfirmasi penghapusan konfigurasi /etc/ncharge.conf
if [ -f /etc/ncharge.conf ]; then
    echo ""
    read -p "Do you also want to remove /etc/ncharge.conf and reset hardware limit to 100%? (y/N): " -r CONFIRM
    if [[ "$CONFIRM" =~ ^[Yy]$ ]]; then
        rm -f /etc/ncharge.conf
        # Reset limit baterai ke 100 (default EC)
        for f in /sys/class/power_supply/BAT*/charge_control_end_threshold; do
            if [ -f "$f" ]; then
                echo 100 > "$f" 2>/dev/null || true
            fi
        done
        echo "✓ Removed /etc/ncharge.conf and reset battery threshold to 100%"
    else
        echo "ℹ Preserved /etc/ncharge.conf"
    fi
fi

echo ""
echo "ncharge has been completely uninstalled."

