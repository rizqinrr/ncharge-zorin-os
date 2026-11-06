# GOAL — ncharge

**Referensi:** `PRD.md` (desain & spesifikasi). File ini mendefinisikan **hasil akhir**: kalau semua di bawah tercapai, proyek selesai.

---

## 1. Hasil Akhir, Digambarkan (literally, sebagai end-user di laptop target)

Di laptop ASUS VivoBook X415EA (dan laptop lain yang sejenis):

1. Di **app drawer GNOME** ada ikon `ncharge`. Klik → window Adwaita kecil terbuka (< 1 detik), tema ikut dark/light sistem.
2. Di dalam window ada **battery status card**: persentase baterai, status (Charging / Discharging / Not charging / Full), bar level dengan garis penanda limit, dan **"Active limit: N%"**, semuanya dibaca live dari sysfs.
3. Ada **3 preset chips: 60 / 80 / 100** (label: "Max lifespan" / "Everyday" / "Full capacity"). Klik chip → animasi spring/jelly. Pindah chip belum commit apa pun, baru memilih; tombol **Save** hanya aktif bila pilihan berbeda dari limit aktif.
4. Tekan **Save** → muncul dialog password polkit (pkexec) → sukses: toast + "Active limit" ikut berubah + animasi. Jika password dibatalkan (exit 126) → tidak ada write, chip kembali ke nilai lama.
5. Baterai benar-benar **berhenti diisi di angka terpilih** (mis. set 80% saat charging → status jadi `Not charging` di sekitar 80%).
6. Tenang setelah dipasang: **tidak ada proses yang hidup di background.** Hanya satu systemd oneshot yang selesai dalam detik saat boot, plus sleep hook saat resume; sisanya dua binary + satu config di disk.
7. Setelah **reboot**, sysfs otomatis kembali ke nilai config meski EC hardware ter-reset ke 100. Setelah **hibernate-resume**, begitu juga.
8. Mau full-charge besok (bepergian)? Buka app → pilih 100 → Save. Berlaku sementara sampai reboot (`--once`, override di `/run/ncharge.override`, `/etc/ncharge.conf` tetap 80). Setelah reboot berikutnya otomatis kembali ke 80 (dari config).
9. `sudo bash uninstall.sh` membersihkan semua artefak, dengan konfirmasi sebelum menghapus config (karena tanpa config, limit kembali ke 100 bawaan EC).

## 2. Acceptance Criteria (harus bisa)

| Kode | Harus bisa... | Cara bukti (cek manual) |
|---|---|---|
| **H1** | Terpasang & muncul di GNOME app drawer | ikon + nama `ncharge` muncul di app drawer |
| **H2** | Menampilkan status baterai live (persen, status, limit aktif) | cocok dengan `cat /sys/class/power_supply/BAT0/capacity` dan `charge_control_end_threshold` |
| **H3** | Limit diset lewat GUI dengan prompt password | dialog pkexec muncul; setelah sukses sysfs berubah |
| **H4** | Baterai **berhenti diisi** di nilai terpilih | set 80 saat charging → jadi `Not charging` di ±80% |
| **H5** | Nilai bertahan setelah **reboot** | setelah reboot: `cat charge_control_end_threshold` = config; journal service sukses |
| **H6** | Nilai bertahan setelah **hibernate-resume** | sama seperti H5 setelah hibernate (jika dipakai) |
| **H7** | Cancel prompt password → tidak menulis, tidak crash | toast info; chip kembali; sysfs tak berubah |
| **H8** | Menangani "hardware tidak didukung" dengan anggun | sysfs file tidak ada → pesan jelas, tanpa crash, Save disabled |
| **H9** | Ringan: target RAM GUI ≤ **60 MB RSS** (batas atas 100 MB); disk ≤ 15 MB; startup < 1 detik | diukur (`/usr/bin/time -v` atau top + `ls -l` binary) |
| **H10** | Nol proses background | `ps` tidak menemukan daemon; service `oneshot` status inactive/exited |
| **H11** | Install/uninstall idempotent | `sudo bash install.sh` dua kali aman; uninstall menghapus semua artefak |
| **H12** | Open-source rapi | repo git + README + LICENSE MIT siap publik |
| **H13** | Preset 100 sementara (`--once`) | Save 100 → sysfs = 100, `/etc/ncharge.conf` tetap 80; hibernate-resume sebelum reboot → tetap 100; reboot → sysfs kembali 80 |

### Standar kualitas umum

- **Safety:** semua penulisan ke sysfs/config hanya lewat helper root yang tervalidasi, tanpa shell-out `sh -c`.
- **Tidak konflik:** ncharge tidak menyentuh apa pun di luar `charge_control_end_threshold` dan config miliknya; TLP/asusctl/PPD tetap bekerja sendiri.
- **Portabilitas:** tidak hardcode `BAT0` eksklusif, scan `BAT*`; path target boleh di-override untuk test.
- **Performa GUI:** pembacaan status langsung dari sysfs (tanpa spawn subprocess).

## 3. Definition of Done (DoD akhir proyek)

- [ ] Semua H1–H13 terverifikasi dan dicentang.
- [ ] `cargo build` + unit tests pass tanpa warning; `cargo clippy` dan `cargo fmt --check` bersih.
- [ ] Manual test plan (PRD §10.1, dijalankan di task.md Fase 4) pass di laptop target, hasil log di `docs/testing-log.md`.
- [ ] Benchmark NFR tercatat di `docs/bench.md` (RSS, disk, startup).
- [ ] README (EN) + `.desktop` + ikon final + LICENSE MIT, repo git siap push publik dengan tag `v1.0.0`.

## 4. Yang eksplisit bukan target (agar scope tidak melebar) → v2

- Kesehatan baterai (v1.1), polkit passwordless, localisasi `.po`, slider custom-limit, installer GUI, semuanya ditunda (lihat PRD §3.2 & §13).

---

*File ini sejajar dengan PRD. Kalau ada konflik, PRD menang. Update GOAL hanya lewat kesepakatan.*
