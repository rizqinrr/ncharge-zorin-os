#!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# 1. Pastikan dijalankan sebagai root (sudo)
if [ "$EUID" -ne 0 ]; then
    echo "Error: install.sh must be run as root (e.g. sudo bash packaging/install.sh)" >&2
    exit 1
fi

# 2. Cek apakah binary release sudah dibuild
GUI_BIN="$PROJECT_ROOT/target/release/ncharge"
HELPER_BIN="$PROJECT_ROOT/target/release/ncharge-apply"

if [ ! -f "$GUI_BIN" ] || [ ! -f "$HELPER_BIN" ]; then
    echo "Error: Release binaries not found." >&2
    echo "Please build the project first as normal user:" >&2
    echo "  cargo build --release" >&2
    exit 1
fi

echo "Installing ncharge..."

# 3. Salin executable binaries
install -m 0755 "$GUI_BIN" /usr/local/bin/ncharge
install -m 0755 "$HELPER_BIN" /usr/local/bin/ncharge-apply
echo "✓ Installed binaries to /usr/local/bin/"

# 4. Pasang systemd oneshot unit
install -m 0644 "$PROJECT_ROOT/packaging/systemd/ncharge-apply.service" /etc/systemd/system/ncharge-apply.service
systemctl daemon-reload
systemctl enable ncharge-apply.service
echo "✓ Enabled systemd boot persistence (ncharge-apply.service)"

# 5. Pasang sleep hook (resume/hibernate)
mkdir -p /usr/lib/systemd/system-sleep
install -m 0755 "$PROJECT_ROOT/packaging/sleep/ncharge" /usr/lib/systemd/system-sleep/ncharge
echo "✓ Installed sleep/resume hook (/usr/lib/systemd/system-sleep/ncharge)"

# 6. Pasang desktop entry & icon
mkdir -p /usr/share/applications
install -m 0644 "$PROJECT_ROOT/packaging/desktop/io.github.ncharge.desktop" /usr/share/applications/io.github.ncharge.desktop

mkdir -p /usr/share/icons/hicolor/scalable/apps
install -m 0644 "$PROJECT_ROOT/packaging/desktop/icons/io.github.ncharge.svg" /usr/share/icons/hicolor/scalable/apps/io.github.ncharge.svg

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications >/dev/null 2>&1 || true
fi
echo "✓ Installed desktop entry and application icon"

# 7. Pasang udev rule (hanya jika flag --with-udev diberikan)
if [ "$1" = "--with-udev" ]; then
    install -m 0644 "$PROJECT_ROOT/packaging/udev/99-ncharge.rules" /etc/udev/rules.d/99-ncharge.rules
    udevadm control --reload-rules || true
    echo "✓ Installed optional udev rule (99-ncharge.rules)"
fi

# 8. Inisialisasi config default (80) jika belum ada dan restore
if [ ! -f /etc/ncharge.conf ]; then
    echo -e "# ncharge configuration\nlimit = 80" > /etc/ncharge.conf
    chmod 0644 /etc/ncharge.conf
    echo "✓ Created default /etc/ncharge.conf (limit = 80)"
fi

/usr/local/bin/ncharge-apply restore >/dev/null 2>&1 || true

echo ""
echo "ncharge successfully installed!"
/usr/local/bin/ncharge-apply status

