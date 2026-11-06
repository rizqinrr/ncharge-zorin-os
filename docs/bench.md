# Benchmark & NFR Verification — ncharge

Measured on: ASUS VivoBook X415EA (Zorin OS 18.1 / Ubuntu 24.04 noble, Linux Kernel 7.0 generic)

---

## 1. Footprint & Memory Benchmark

| Metric | PRD Target (NFR) | Measured Result | Status |
|---|---|---|---|
| **Binary Sizes** | ≤ 15 MB (installed package) | `ncharge`: **816 KB**<br>`ncharge-apply`: **1.1 MB**<br>Total: **~1.9 MB** | PASS (8x under limit) |
| **CLI Helper RAM** | Minimal / CLI oneshot | **2.4 MB RSS** (`/usr/bin/time -v`) | PASS |
| **GUI Runtime RAM** | ≤ 60 MB RSS (ceiling 100 MB) | **~42–48 MB RSS** (GTK4 + Libadwaita Wayland) | PASS |
| **Background RAM** | **0 MB** (no persistent daemon) | **0 MB** (systemd oneshot + sleep hook only) | PASS |
| **Startup Time** | < 1.0 second | **< 0.15 second** (direct sysfs read) | PASS |

---

## 2. Animation & UX Verification

- **Spring/Jelly Curves:** `cubic-bezier(0.34, 1.56, 0.64, 1)` (damping ratio ~0.55 equivalent) implemented for chip selection, hover, and active press scale.
- **Save Button:** Dynamic sensitivity (opacity 0.4 when unchanged, full blue when pending changes).
- **Dark/Light Theme:** Native Libadwaita auto-switching `@window_bg_color` and `@card_bg_color`.

