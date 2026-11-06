# PRD — ncharge

**Product:** ncharge — Battery Charge Limit Manager untuk Linux
**Versi PRD:** 1.1 (desain final disepakati, pra-implementasi)
**Tanggal:** 2026-11-06
**Status:** Desain UI dan koreksi spesifikasi dikunci; implementasi siap dimulai
**Target utama:** Laptop pribadi (ASUS VivoBook X415EA, lihat Lampiran A) dengan arsitektur yang tetap universal untuk laptop lain yang mengekspos sysfs charge threshold

---

## 1. Ringkasan (Elevator Pitch)

ncharge adalah aplikasi GUI ringan, open-source, untuk Linux: **"MyASUS Battery Health Charging" versi universal**, tapi fokus satu fungsi saja: **membatasi maksimal pengisian daya laptop** (misalnya berhenti di 80%) agar kesehatan baterai terjaga, tanpa memasang daemon berat (TLP, asusctl) atau extension GNOME.

> **Filsafat produk:** satu fungsi, dikerjakan sempurna. Dibuka → atur → Save → close. Nol daemon di background.

---

## 2. Latar Belakang & Masalah

### 2.1 Masalah

- Baterai Li-ion yang terus disimpan di 100% sambil dicolok menua lebih cepat; membatasi ke 80% memperlambatnya (Battery University, BU-808).
- Laptop ASUS (dan sebagian besar vendor lain) mengekspos kontrol limit pengisian ke kernel Linux via:

  ```
  /sys/class/power_supply/BAT0/charge_control_end_threshold
  ```

- Akan tetapi **EC (Embedded Controller) ASUS mengreset nilai ini ke 100 pada setiap power cycle**, jadi untuk dipakai konsisten, wajib ada mekanisme re-apply saat boot dan resume (bukti: riset di Lampiran B).
- Tool Linux yang umum masing-masing kurang pas untuk use case ini:

  | Tool | Kelemahan untuk use case ini |
  |---|---|
  | **TLP** | Power tuning menyeluruh; harus mask `power-profiles-daemon`; banyak moving part |
  | **asusctl** | Daemon D-Bus berat; target utamanya ROG/TUF; kompatibilitas di VivoBook belum pasti |
  | **GNOME ext "Battery Health Charging"** | GNOME-only; tidak portable ke DE lain |
  | **Skrip manual / cron** | Berhasil, tapi tanpa GUI, tidak nyaman diubah-ubah |

### 2.2 Peluang

Kontrol hardware-nya **sudah ada di kernel**; yang kurang hanya **(a)** UI nyaman dan **(b)** jaminan nilai selalu diterapkan kembali setelah reset EC. ncharge mengisi dua hal itu dengan footprint minimal.

### 2.3 Nol Konflik

ncharge **tidak menyentuh** apa pun di luar file targetnya. TLP/asusctl/PPD tetap bekerja sendiri, ncharge hanya menulis `charge_control_end_threshold` dan membaca file lainnya secara read-only. (Catatan README nanti: jangan jalankan dua tool yang sama-sama mengatur charge threshold secara otomatis.)

---

## 3. Tujuan & Non-Tujuan

### 3.1 Goals

| Kode | Tujuan | Metrik keberhasilan |
|---|---|---|
| **G1** | Limit pengisian bisa diatur via GUI → baterai berhenti charge di nilai terpilih | Set 80% → saat mencapai 80%, `status` = `Not charging` |
| **G2** | Nilai terpilih bertahan setelah reboot dan resume (meski EC ter-reset) | Setelah boot / hibernate-resume → sysfs = pilihan user |
| **G3** | Aplikasi ringan & menarik: tampilan native Adwaita + animasi spring ("jelly") | Target RSS GUI ≤ 60 MB (batas atas 100 MB, diukur di M2); animasi ada |
| **G4** | Universal: jalan di distro Linux modern selama sysfs file tersedia | Teruji di Ubuntu target; code tidak hardcode vendor |
| **G5** | Open-source sejak hari pertama: repo bersih, README mantap | Repo publik + README sebelum tag 1.0 |

### 3.2 Non-Goals (eksplisit)

- ❌ Bukan pengganti TLP/asusctl; tidak menyentuh profil daya, fan, LED, GPU.
- ❌ Tidak mengontrol **discharge**: baterai yang saat ini di atas limit tidak akan "diturunkan"; threshold hanya menghentikan charging (batasan hardware, dinyatakan jujur di UI/README).
- ❌ Tidak ada daemon yang jalan terus, hanya oneshot service + hooks.
- ❌ Tidak mendukung start threshold kecuali runtime menemukan filenya (laptop target tidak mengeksposnya).
- ❌ Tidak ada sinkronisasi dengan MyASUS di Windows (dual-boot: nilai EC terakhir menang; didokumentasikan).
- ❌ Hanya satu binary helper internal; CLI publik didefer.
- ❌ Gratis & open-source (license MIT), tidak dijual.

---

## 4. Target Pengguna

1. **Persona utama:** Pemilik laptop Linux (khususnya ASUS VivoBook X415EA) yang charger-nya nyaris selalu terpasang seharian (coding/study) dan ingin umur baterai panjang.
2. **Persona sekunder:** Pengguna laptop merek lain (ThinkPad/LG/ASUS lain) yang butuh charge-limit GUI ringan tanpa memasang stack besar.
3. **Anti-persona:** Pengguna yang mau power tuning lengkap → arahkan ke asusctl/TLP di README.

---

## 5. Positioning

- **v1.0:** "Aplikasi open-source satu fungsi: limit pengisian baterai, ringan, bergaya GNOME/Adwaita."
- Nanti (v1.1 / v2+, di luar PRD ini): kesehatan baterai (v1.1), polkit passwordless opt-in, custom threshold (slider), start threshold bila didukung hardware, lokalisasi.

---

## 6. Persyaratan Fungsional (FR)

### FR-1 Probe Hardware (tanpa root)

- Saat start, app scan `/sys/class/power_supply/BAT*/` (iterasi semua baterai).
- Baca (read-only, tanpa root):
  - `capacity`, `status` (`Charging` / `Discharging` / `Full` / `Not charging`)
  - `charge_control_end_threshold` → limit aktif di EC
  - (v1.1) `energy_full`, `energy_full_design` (fallback ke `charge_full`, `charge_full_design` bila `energy_*` tidak ada) → kesehatan baterai = `full / full_design × 100`
- Jika file threshold tidak ada → masuk state "tidak didukung" (FR-9).
- Path baterai **dinamis**, tidak hardcode `BAT0` di logika; `BAT0` hanya nilai default untuk laptop target.

### FR-2 Panel Status

Panel status di jendela utama menampilkan:
- Persentase baterai saat ini (dari `capacity`).
- Status pengisian (`Charging` / `Discharging` / `Not charging` / `Full`).
- Bar level baterai dengan garis penanda (tick) di posisi limit aktif.
- **Active limit** hasil pembacaan sysfs (*bukan* dari config), biar user tahu nilai nyata di EC. Bila override sementara (`/run/ncharge.override`) aktif, tampilkan sublabel `Until next restart, then N%`.
- Bila `capacity > active_limit`, tampilkan banner di dalam kartu: `Above the limit. The battery drops to N% with normal use.`
- Update live: polling ringan (2 detik) di v1; upgrade ke UPower D-Bus signal nanti bisa.

### FR-3 Pilihan Limit (preset)

- Tiga preset chip: **60 / 80 / 100** dengan label:
  - 60 → `Max lifespan` (untuk yang nyaris selalu di charger)
  - 80 → `Everyday` (default; setara MyASUS "Balanced")
  - 100 → `Full capacity` (mau bepergian lepas charger, berlaku sementara sampai reboot)
- Chip berperilaku radio (satu terpilih); perpindahan pilihan memicu **spring animation**.
- Memilih 100 menampilkan penjelasan: `Charges to 100% until the next restart. Then your saved limit of N% comes back.`
- Pilihan belum di-commit sampai user menekan **Save**; selama belum disimpan, tampilkan baris: `Not saved. The battery still stops at N%.` Menutup window tanpa Save = pilihan tidak diterapkan (_unsaved changes_ dibuang).
- Tombol **Save** nonaktif (insensitive) bila pilihan sama dengan limit aktif dan tidak ada perubahan mode override.
- Default app baru dibuka: nilai limit aktif saat ini (atau **80** bila belum pernah diset).

### FR-4 Save (tulis, via pkexec)

Tombol `Save`:
1. Menulis ke sysfs via `pkexec` (prompt password standar polkit):
   - Untuk preset **60** atau **80** (permanen):
     ```
     pkexec /usr/local/bin/ncharge-apply apply <limit>
     ```
   - Untuk preset **100** (sementara sampai reboot):
     ```
     pkexec /usr/local/bin/ncharge-apply apply 100 --once
     ```
   Alasan pakai helper binary yang well-defined (bukan `sh -c 'echo...'`): bersih di log polkit, validasi input terpusat, tidak ada shell injection surface.
2. Helper `ncharge-apply`:
   - Validasi argumen (integer 1–100).
   - Tulis ke semua `/sys/class/power_supply/BAT*/charge_control_end_threshold` yang ada.
   - Tanpa `--once`: tulis nilai secara atomik ke `/etc/ncharge.conf` (mode `0644`) dan hapus `/run/ncharge.override` bila ada.
   - Dengan `--once`: tulis `limit = 100` ke `/run/ncharge.override` (mode `0644`) tanpa menyentuh `/etc/ncharge.conf`.
   - Exit code yang rapi (lihat FR-8 & FR-12).
3. Feedback UI:
   - Sukses 60/80 → toast `Charge limit set to N%` + panel Active limit ikut berubah + spring animation.
   - Sukses 100 (`--once`) → toast `Charge limit set to 100% until restart` + sublabel `Until next restart, then N%`.
   - Gagal / password dibatalkan → toast sesuai mapping FR-8.

Prompt password muncul tiap kali Save (keputusan terkunci). Opsi "passwordless via polkit rule" = tombol opt-in di v2.

### FR-5 Persistensi (boot & resume)

Tiga lapis, sesuai hasil riset (Lampiran B):

#### FR-5.1 (utama): systemd oneshot + retry

Unit `/etc/systemd/system/ncharge-apply.service`:

```ini
[Unit]
Description=Apply ncharge battery charge limit
After=multi-user.target
StartLimitIntervalSec=120
StartLimitBurst=10

[Service]
Type=oneshot
ExecStart=/usr/local/bin/ncharge-apply restore
Restart=on-failure
RestartSec=5
RestartPreventExitStatus=1

[Install]
WantedBy=multi-user.target
```

- `restore` = baca `/run/ncharge.override` bila ada, kalau tidak baca `/etc/ncharge.conf` → tulis ke sysfs (tidak menulis config lagi). Karena `/run` adalah tmpfs, override otomatis hilang saat reboot dan kembali ke nilai `/etc/ncharge.conf`.
- Di-enable saat install; di-disable saat uninstall (FR-11).
- Alasan `After=multi-user.target` + retry terukur (maks 10x tiap 5 detik dalam 120 detik): file sysfs baru ada setelah modul `asus_nb_wmi` load yang kadang telat; retry menutupi race ini tanpa loop tanpa henti bila hardware memang tidak didukung. Exit 1 (argumen/config invalid) tidak di-retry. Catatan: `Restart=` untuk `Type=oneshot` butuh systemd ≥ 244 (Ubuntu 24+ sudah memenuhi).

#### FR-5.2 (lapis 2): sleep hook (resume/hibernate)

- File `/usr/lib/systemd/system-sleep/ncharge` (script kecil): pada fase `post` (resumed), jalankan `ncharge-apply restore` sekali lagi.
- Alasan: nilai EC ter-reset setelah **hibernate-resume** (ArchWiki; Lampiran B). Karena `/run/ncharge.override` ikut tersimpan di image RAM saat hibernate, mode `--once` tetap bertahan melewati hibernate dan baru hilang saat reboot penuh.

#### FR-5.3 (lapis 3, opsional, khusus ASUS): udev rule

- File `/etc/udev/rules.d/99-ncharge.rules`, rule di device `asus-nb-wmi` (bukan di `BAT0`):
  ```
  ACTION=="add", SUBSYSTEM=="platform", KERNEL=="asus-nb-wmi", RUN+="/usr/local/bin/ncharge-apply restore"
  ```
- **Catatan riset penting:** udev rule yang menarget device `power_supply`/`BAT0` terbukti sering gagal karena atribut threshold belum dibuat saat rule jalan (Lampiran B). Rule ini khusus ASUS dan hanya dipasang lewat flag `--with-udev` pada `install.sh`; desain wajib tetap valid tanpa lapis ini agar G4 (universal) tetap terpenuhi.

### FR-6 Config Persist & Temporary Override

- Path permanen: `/etc/ncharge.conf`; format mini key=value (diparse sederhana, tidak butuh parser INI penuh):
  ```
  # ncharge: batas pengisian (ditulis oleh helper root)
  limit = 80
  ```
- Path sementara (`--once`): `/run/ncharge.override` dengan format yang sama (`limit = 100`).
- Helper (root) menulis file secara atomik (tulis ke file sementara di direktori yang sama lalu `rename`) dengan mode `0644` agar GUI user biasa selalu bisa membaca.
- File `/etc/ncharge.conf` hilang/corrupt saat `restore` → helper pakai default 80, tulis ulang config dengan mode `0644`, log warning; tidak crash.

### FR-7 UI & Polish

- Window Adwaita satu halaman (gaya settings): dibuka → pilih → Save → close. Lebar 400px, tidak resizable. Referensi visual: `design/ncharge.figcraft` dan `design/ncharge-screens.png` (6 layar).
- Struktur: battery status card di atas (persentase, status, bar + tick limit, Active limit), kelompok `Charge limit` (preset chips + penjelasan), tombol `Save` (suggested) di header bar.
- Spring/jelly animation:
  - Chip terpilih: bounce scale (0.95 → 1.05 → 1.0) via `AdwSpringAnimation` (damping ratio ±0.55).
  - Perubahan angka/label: cross-fade + slide kecil.
  - Save sukses: toast + update nilai.
- Ikuti dark/light mode sistem (Adwaita default). Warna aksen biru tombol dan seleksi menggunakan `#1C71D8` di light mode (agar kontras teks putih ≥ 4.5:1 sesuai WCAG AA) dan `#78AEED` di dark mode.
- Bahasa UI v1: **Inggris** (skeleton gettext `.po` siap; lokal `id` di v1.1).

Tabel string UI v1:

| Kunci | Teks |
|---|---|
| `title` | `ncharge` |
| `save` | `Save` |
| `active_limit` | `Active limit` |
| `group` | `Charge limit` |
| `p60` | `60%` · `Max lifespan` · `Charging stops at 60%. Best when the charger stays plugged in all day.` |
| `p80` | `80%` · `Everyday` · `Charging stops at 80%, so the battery does not sit at full voltage while the charger stays plugged in.` |
| `p100` | `100%` · `Full capacity` · `Charges to 100% until the next restart. Then your saved limit of N% comes back.` |
| `unsaved` | `Not saved. The battery still stops at N%.` |
| `until_restart` | `Until next restart, then N%` |
| `above_limit` | `Above the limit. The battery drops to N% with normal use.` |
| `toast_ok` | `Charge limit set to N%` |
| `toast_once` | `Charge limit set to 100% until restart` |
| `toast_cancel` | `Cancelled. Nothing was changed.` |
| `toast_denied` | `Not authorized` |
| `toast_fail` | `Could not set the limit. See journalctl -xeu ncharge-apply` |
| `unsupported_title` | `Charge limit not available` |
| `unsupported_body` | `This laptop does not give Linux a charge threshold file. It comes from the vendor driver, such as asus_wmi on ASUS laptops. To check, run:` |

### FR-8 Mapping Error

| Kondisi | Deteksi | Perilaku |
|---|---|---|
| Dialog password pkexec ditutup/dibatalkan user | pkexec exit code `126` | Toast `Cancelled. Nothing was changed.`; pilihan chip kembali ke nilai sebelumnya |
| User tidak diizinkan / polkit error | pkexec exit code `127` | Toast `Not authorized`; pilihan chip kembali ke nilai sebelumnya |
| Sysfs threshold tidak ada | FR-1 probe / helper exit `2` | State "tidak didukung" (FR-9); tombol Save disabled |
| Write gagal | helper exit `3` (atau selain 0/126/127) | Toast `Could not set the limit. See journalctl -xeu ncharge-apply` |
| Limit aktif < level baterai sekarang | GUI baca `capacity > active_limit` | Tetap sukses; banner di kartu status: `Above the limit. The battery drops to N% with normal use.` |
| Config hilang/corrupt saat restore | helper | Apply 80 + tulis config baru (`0644`) + log warning |
| Multiple baterai | iterasi `BAT*` | Apply ke semua yang punya file threshold; UI menampilkan baterai pertama |
| EC ter-reset unplugged (rare) | tidak terdeteksi | Bahas di README; opsi `--with-udev` |
| Baterai sangat rendah (critical) | — | Bukan scope app (UPower yang handle) |

### FR-9 State "Tidak Didukung"

- Judul: `Charge limit not available`
- Body: `This laptop does not give Linux a charge threshold file. It comes from the vendor driver, such as asus_wmi on ASUS laptops. To check, run:` diikuti blok kode `ls /sys/class/power_supply/BAT0/`. Tombol Save nonaktif; app tidak crash (lihat layar 4 di `design/ncharge-screens.png`).

### FR-10 Desktop Integration

- `ncharge.desktop` (Name, Comment, Icon, `Categories=Utility;Settings;`) → muncul di app drawer GNOME.
- Icon SVG sederhana (baterai + batas), placeholder boleh di M2, final di M5.

### FR-11 Install / Uninstall (`install.sh` & `uninstall.sh`, idempotent)

Build dilakukan sebagai user biasa (karena `rustup`/`cargo` terpasang di home user, bukan root), lalu instalasi dijalankan dengan `sudo`:
```bash
cargo build --release
sudo bash packaging/install.sh
```

`install.sh`:
1. Cek `$EUID -eq 0` (kalau bukan root, keluar dengan pesan jelas).
2. Cek keberadaan `target/release/ncharge` dan `target/release/ncharge-apply`. Jika belum ada, tampilkan `Run cargo build --release first` dan `exit 1` (script tidak menjalankan `cargo build` sebagai root).
3. Copy `ncharge` (GUI) dan `ncharge-apply` ke `/usr/local/bin/` (mode `0755`).
4. Pasang unit systemd (FR-5.1) → `systemctl daemon-reload` → `systemctl enable ncharge-apply.service`.
5. Pasang sleep hook (FR-5.2, mode `0755`).
6. Pasang `.desktop` + icon (FR-10).
7. Pasang udev rule (FR-5.3) **hanya dengan flag `--with-udev`**.
8. Bila `/etc/ncharge.conf` belum ada, buat dengan `limit = 80` (mode `0644`), lalu jalankan `/usr/local/bin/ncharge-apply restore`.
9. Cetak ringkasan status akhir.

`uninstall.sh`:
- Cek `$EUID -eq 0`, hapus semua artefak pemasangan secara idempotent, stop/disable service dulu, hapus `/run/ncharge.override` bila ada.
- **Tidak menghapus `/etc/ncharge.conf` tanpa konfirmasi**: kalau config dihapus dan app di-uninstall, limit direset ke 100 (default EC). Prompt ke user untuk memilih: hapus config (dan kembalikan sysfs ke 100) atau simpan.

### FR-12 CLI Helper (`ncharge-apply`: `apply`/`restore` harus root, `status` tidak)

| Subcommand | Fungsi | Pemanggil | Butuh root |
|---|---|---|---|
| `apply <N> [--once]` | Tanpa `--once`: tulis sysfs + tulis `/etc/ncharge.conf` + hapus `/run/ncharge.override`. Dengan `--once`: tulis sysfs + tulis `/run/ncharge.override` | GUI via pkexec | Ya |
| `restore` | Baca `/run/ncharge.override` (jika ada) atau `/etc/ncharge.conf` → tulis sysfs | systemd / sleep hook / udev | Ya |
| `status` | Cetak limit aktif (sysfs), nilai config, dan override (jika ada) | user / debug | Tidak |

- Exit codes: `0` sukses; `1` argumen invalid; `2` sysfs tidak tersedia; `3` write gagal (tidak bentrok dengan exit code pkexec `126`/`127`).
- CLI parsing pakai `clap` atau parsing manual minimal (ditentukan saat M1); **tanpa shell-out** ke `/bin/bash`.

---

## 7. Persyaratan Non-Fungsional (NFR)

| Kode | Persyaratan | Target |
|---|---|---|
| NFR-1 | RAM runtime GUI | Target ≤ 60 MB RSS, batas atas 100 MB (diukur di M2; angka final dikunci setelah pengukuran) |
| NFR-2 | RAM background | 0 MB: oneshot service, tidak ada daemon |
| NFR-3 | Disk footprint | ≤ 15 MB terpasang (2 binary + config + unit + hook + desktop/icon) |
| NFR-4 | Privilege | Tulis ke sysfs hanya via pkexec + helper; helper tanpa shell-out |
| NFR-5 | Waktu startup app | < 1 detik (baca sysfs langsung) |
| NFR-6 | Kompatibilitas | Distro systemd modern (systemd ≥ 244, Ubuntu 24/26-base); Rust stable; gtk4-rs + libadwaita-rs versi stable |
| NFR-7 | Keamanan | Validasi input di helper; tidak pakai `sh -c 'echo > ...'`; tulis config atomik mode `0644` |
| NFR-8 | License | MIT |
| NFR-9 | Aksesibilitas | Keyboard-navigable, a11y native GTK (label & focus), kontras teks memenuhi WCAG AA (≥ 4.5:1) |

---

## 8. Arsitektur Teknis

```
┌────────────────────────────┐   pkexec (password)   ┌──────────────────────────────────┐
│  ncharge (GUI, Rust)       │ ────────────────────▶ │ ncharge-apply (helper, root)     │
│  gtk4-rs + libadwaita-rs   │  apply <N> [--once]   │ tulis sysfs + /etc/ncharge.conf  │
└────────────────────────────┘                       │ (atau /run/ncharge.override)     │
        │ baca (read-only, probe & status)           └──────────────────────────────────┘
        ▼                                                             ▲
 /sys/class/power_supply/BAT0/*                      • ncharge-apply.service (boot)
        │                                            • /usr/lib/systemd/system-sleep/ncharge (resume)
        ▼                                            • /etc/udev/rules.d/99-ncharge.rules (opsional)
 EC — via driver kernel: asus_wmi / asus_nb_wmi
```

### 8.1 Struktur repo

```
ncharge/
├── PRD.md
├── goal.md
├── task.md
├── README.md                 (diisi M5)
├── Cargo.toml                [workspace]
├── design/
│   ├── ncharge.figcraft      file sumber desain FigCraft
│   └── ncharge-screens.png   ekspor 6 layar referensi UI
├── crates/
│   ├── gui/                  ncharge (bin) — gtk4-rs, libadwaita-rs
│   └── apply/                ncharge-apply (bin + lib) — logika sysfs & config (tertest)
├── packaging/
│   ├── install.sh / uninstall.sh
│   ├── systemd/ncharge-apply.service
│   ├── sleep/ncharge
│   ├── udev/99-ncharge.rules      (opsional, khusus ASUS)
│   └── desktop/ncharge.desktop + icons/
└── docs/                     (nanti: hasil pengujian & benchmark)
```

### 8.2 Detail kecil yang sudah dikunci

- Config key: `limit` (integer 1–100, satuan persen) di `/etc/ncharge.conf`; override sementara di `/run/ncharge.override`.
- Alur helper: `probe → validate → write_sysfs → write_config_or_override (atomic 0644) → report`.
- Animasi: `AdwSpringAnimation` (damping ≈ 0.55, stiffness ≈ 200).

---

## 9. Desain UI (Referensi Visual)

File desain lengkap tersimpan di `design/ncharge.figcraft` dan diekspor ke `design/ncharge-screens.png`. Enam layar yang menjadi acuan implementasi M2 dan M4:

1. **Saved state (light):** baterai 74% `Charging`, bar dengan penanda di 80%, `Active limit: 80%`, chip `80% Everyday` terpilih, tombol `Save` nonaktif (insensitive).
2. **New preset picked, not saved:** user memilih `100% Full capacity`, tombol `Save` aktif, muncul teks `Charges to 100% until the next restart. Then your saved limit of 80% comes back.` dan catatan tebal `Not saved. The battery still stops at 80%.`
3. **After Save, toast (dark):** setelah Save 100%, `Active limit` menjadi `100%` dengan sublabel `Until next restart, then 80%`, tombol `Save` kembali nonaktif, muncul toast `Charge limit set to 100% until restart`.
4. **No threshold file (FR-9):** tampilan saat `charge_control_end_threshold` tidak tersedia, judul `Charge limit not available` + perintah cek `ls /sys/class/power_supply/BAT0/`, tombol `Save` nonaktif.
5. **Password dialog cancelled:** ketika dialog pkexec ditutup (exit `126`), pilihan chip kembali ke `80%` dan muncul toast `Cancelled. Nothing was changed.`
6. **Battery above the limit:** saat baterai `92%` (`Not charging`) sementara limit aktif `80%`, kartu status menampilkan banner `Above the limit. The battery drops to 80% with normal use.`

---

## 10. Rencana Pengujian

### 10.1 Manual (di laptop target, akhir M3)

| # | Skenario | Ekspektasi |
|---|---|---|
| T1 | Set 80 saat baterai 74% (charger pasang) | Masuk `Not charging` di ±80%; sysfs terbaca 80 |
| T2 | Reboot | sysfs = 80; service `ncharge-apply.service` sukses di journal |
| T3 | Suspend-to-RAM → resume | sysfs masih 80 |
| T4 | Hibernate → resume (apabila dipakai) | sysfs kembali ke nilai aktif (sleep hook) |
| T5 | Set 100 → Save (`--once`, mau full-charge) | Charge berjalan ke 100%, `/run/ncharge.override` = 100, `/etc/ncharge.conf` tetap 80; setelah reboot kembali 80 |
| T6 | Cancel pkexec prompt (exit 126) | Toast `Cancelled. Nothing was changed.`; nilai chip kembali; tanpa write |
| T7 | Uninstall (dengan confirm hapus config) | EC kembali default 100 |
| T8 | Kesehatan baterai *(ditunda ke v1.1)* | % = `energy_full / energy_full_design` (atau fallback `charge_full*`) |
| T9 | Resource | Target RSS GUI ≤ 60 MB (batas atas 100 MB); tanpa proses background setelah service `oneshot` selesai |
| T10 | App drawer | ikon + nama `ncharge` muncul |
| T11 | Limit aktif di bawah level baterai saat ini | Banner `Above the limit...` tampil di kartu status |

### 10.2 Unit test (cargo, workspace)

- Parser & writer config: valid / corrupt / hilang / izin `0644` / prioritas `/run/ncharge.override` di atas `/etc/ncharge.conf`.
- Helper arg validation: non-int, out-of-range (`1..=100`), flag `--once`.
- Probe: matrix `BAT0`, `BAT1`, none, multi-battery.

---

## 11. Roadmap

| Milestone | Isi | DoD |
|---|---|---|
| **M0 — Scaffold** | `git init`, workspace Rust, PRD + desain committed, CI skeleton (build+test) | Repo jalan |
| **M1 — Core** | `ncharge-apply` (probe/apply/apply --once/restore/status) + unit test | Pemanggilan langsung (sudo manual) sukses |
| **M2 — GUI dasar** | Window + status + bar + chips + Save via pkexec + ukur RSS awal | T1, T6, T11 |
| **M3 — Persist** | systemd unit + sleep hook + install.sh (+ udev ops) | T2, T3, T4, T5 |
| **M4 — Polish** | Spring animasi, toast, dark/light, a11y, verifikasi 6 layar desain | T9 + sign-off visual user |
| **M5 — Packaging** | `.desktop`, icon final, README (EN) dengan install + FAQ | G5 |
| **M6 — Release** | Push ke repo publik, tag v1.0.0 | — |

---

## 12. Risiko & Mitigasi

| Risiko | Level | Mitigasi |
|---|---|---|
| Timing modul `asus_nb_wmi` load → sysfs belum ada saat service jalan | Sedang | `After=multi-user.target` + `Restart=on-failure` (`RestartSec=5`, `StartLimitBurst=10`) + (ops) early module load via initramfs (didokumentasikan) |
| RSS GTK4 dengan renderer GL melewati 60 MB | Sedang | Ukur lebih awal di akhir M2 (T3.9); batas atas 100 MB; uji dampak `GSK_RENDERER` bila perlu |
| EC reset saat baterai di-plug ulang secara mekanis | Rendah | README + flag `--with-udev` |
| Prompt password tiap Save terasa ribet | Rendah | Opt-in polkit passwordless di v2 |
| Nama file sysfs berubah di kernel lain | Rendah | Helper melakukan scan `charge_control_end_threshold` di semua `BAT*` |
| Nilai berubah oleh MyASUS di Windows (dual-boot) | Info | Didokumentasikan: EC menyimpan satu nilai global; MyASUS menimpanya saat di Windows |

---

## 13. Pertanyaan Terbuka (bukan blocker M0)

1. Desain ikon app final (placeholder dulu di M2, final di M5).
2. Kesehatan baterai: **dikunci ditunda ke v1.1** agar v1.0 fokus inti bersih.
3. Polkit passwordless opt-in dimasukkan di installer v1 atau v2? (sekarang: v2).
4. Lokal Indonesia (`id.po`) ikut M5 atau setelahnya? (sekarang: setelahnya, v1.1).

---

## Lampiran A — Fakta Hardware dari Laptop Target (tercek langsung, Nov 2026)

- Mesin: `ASUSTeK / VivoBook_ASUSLaptop X415EA_A1400EA` (model `X415EA`).
- Kernel: `7.0.0-42-generic`; modul `asus_wmi`, `asus_nb_wmi`, `asus_armoury` aktif.
- Battery sysfs ada di `/sys/class/power_supply/BAT0` berisi: `charge_control_end_threshold` (nilai sekarang: **100**, tanpa limit), `capacity`, `status` (`Charging`), `charge_type`, `cycle_count`, `energy_full`, `energy_full_design`.
- **Tidak ada** `charge_control_start_threshold`, hanya end threshold yang diekspos.
- Power supply AC: `AC0`.
- Rentang valid `charge_control_end_threshold` di kernel power-supply class: `1..100` (100 = off/full).

## Lampiran B — Ringkasan Riset Persistensi (snapshot Nov 2026)

Sumber: ArchWiki "Laptop/ASUS" + AskUbuntu (thread 1006778 & 1529927) + repo evemarco/battery-charge-limit + forum Q4OS (KDE PowerDevil tidak persist) + TLP docs (Context7: `/websites/linrunner_de_tlp`).

| Fakta | Konsekuensi desain |
|---|---|
| EC ASUS mengreset threshold ke 100 per **power cycle** | Persist wajib: systemd oneshot = lapis 1 |
| udev rule yang menarget device `power_supply`/`BAT0` sering **gagal** karena atribut dibuat belakangan oleh `asus_nb_wmi` | udev rule bukan lapis utama; kalau dipakai target `asus-nb-wmi` dengan `SUBSYSTEM=="platform"` (lapis 3, ops, khusus ASUS) |
| systemd oneshot + `After=multi-user.target` + retry terbukti paling awet (laporan multi-tahun) | Diadopsi sebagai lapis 1 |
| Setelah **hibernate-resume** nilai ter-reset; suspend-to-RAM aman | Lapis 2: sleep hook pada resume |
| TLP/asusctl/ext-GNOME masing-masing menutup kebutuhan ini dengan cara berbeda tapi berat/khusus | ncharge = wrapper universal tipis: 3 lapis + GUI |

---

*End of PRD — ncharge v1.1. Langkah berikutnya: Fase 0 & M0 scaffold.*
