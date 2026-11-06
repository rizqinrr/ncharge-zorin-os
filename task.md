# TASKS — ncharge

Urutan eksekusi berdasarkan PRD + GOAL. Setiap task punya: **siapa**, **aksi**, dan **cek sukses** (kalau ceknya gagal → stop, lapor, diskusikan; jangan meneruskan di atas error).

Penanda:
- **[A]** = asisten lakukan sendiri, non-destruktif.
- **[U]** = user lakukan sendiri (butuh interaksi terminal / sudo / konfirmasi fisik).
- **[A→U]** = asisten siapkan command/text, user jalankan.

Aturan keamanan yang berlaku sepanjang proyek:
1. **Jangan pernah menulis ke `/sys` real battery, `/etc`, `/usr`, `/var` tanpa bilang user dulu**: perubahan yang mengubah perilaku charge hardware butuh konfirmasi.
2. Kegagalan verifikasi = stop, lapor, diskusikan; jangan menumpuk perubahan di atas yang belum terbukti.
3. Setiap fase diakhiri commit git dengan pesan yang jelas.

---

## Fase 0 — Environment Readiness (`~15–30 menit, ikut user`)

- [ ] **T0.1 [A]** Konfirmasi keadaan environment (sudah dicek 2026-11-06): Rust ✗, GTK4-dev ✗, libadwaita-dev ✗, git ✓, pkexec ✓, session GNOME/Wayland ✓. Re-cek kalau sudah lama.
- [ ] **T0.2 [A→U]** Install Rust via rustup (user pilih default-stable):
      ```bash
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
      source "$HOME/.cargo/env"
      ```
      Cek: `rustc --version` ≥ stable terkini; `cargo --version` OK.
- [ ] **T0.3 [A→U]** Install build deps GTK (butuh sudo):
      ```bash
      sudo apt update
      sudo apt install -y libgtk-4-dev libadwaita-1-dev
      ```
      Cek: `pkg-config --exists gtk4 libadwaita-1 && echo GTK-deps-ready`.
- [ ] **T0.4 [A]** Smoke-test compile GTK: crate sementara di `/tmp` yang hanya memanggil `adw::Application::new()` + build → pastikan toolchain bisa compile aplikasi GTK. Hapus setelah pass.

## Fase 1 — Scaffold repo (M0)

- [ ] **T1.1 [A]** `git init` di `/media/rizqi/DATA/01_Coding/nProjects/ncharge`; bikin `.gitignore` (`target/`, `dist/`), `LICENSE` (MIT).
- [ ] **T1.2 [A]** Commit `PRD.md`, `goal.md`, `task.md`, dan `design/` (commit "docs: PRD + goal + task plan + UI design").
- [ ] **T1.3 [A]** Rust workspace `Cargo.toml` + `crates/gui/`, `crates/apply/` (keduanya skeleton `fn main()`), satu dependency shared; `cargo check` pass tanpa error.
- [ ] **T1.4 [A]** (opsional, ringan) CI skeleton di `.github/workflows/ci.yml`: `cargo build` + `cargo test` (tanpa GUI runtime).
- [ ] **T1.5 [A]** Commit: "feat: workspace scaffold".

## Fase 2 — Core logic `ncharge-apply` (M1)

- [ ] **T2.1 [A]** Modul `crates/apply/src/`:
      - `config.rs`: parse + atomic write `/etc/ncharge.conf` dan `/run/ncharge.override` (format `limit = N`, mode `0644`); `restore` membaca override lebih dulu, baru config; unit test valid/corrupt/missing/override.
      - `power.rs`: scan `/sys/class/power_supply/BAT*/`, isi struct (capacity, status, threshold), cek keberadaan file attr; unit test dengan fixture dir (env var override root paths untuk test).
      - `apply.rs`: `apply(limit, once)` (tanpa `--once`: tulis sysfs + config + hapus override; dengan `--once`: tulis sysfs + override tanpa menyentuh config); `restore` (override/config → sysfs, default 80 on missing); exit codes 0/1/2/3.
- [ ] **T2.2 [A]** Binary CLI (clap atau parsing manual minimal): `apply <N> [--once]` / `restore` / `status` (`status` tidak butuh root); tanpa shell-out; validasi strict (`1..=100`).
- [ ] **T2.3 [A]** `cargo test` pass; `cargo clippy` clean.
- [ ] **T2.4 [A→U]** Manual smoke root (di mesin, **dengan persetujuan user karena menyentuh hardware charge**):
      ```bash
      ./target/release/ncharge-apply status
      sudo ./target/release/ncharge-apply apply 80          # tulis 80 ke sysfs + /etc/ncharge.conf
      cat /sys/class/power_supply/BAT0/charge_control_end_threshold
      sudo ./target/release/ncharge-apply apply 100 --once  # tulis 100 ke sysfs + /run/ncharge.override
      cat /etc/ncharge.conf                                 # pastikan tetap limit = 80
      sudo ./target/release/ncharge-apply apply 100         # kembalikan kalau user belum siap persist
      ```
      Cek: nilai sysfs, config, dan override benar sesuai aksi; exit codes benar untuk `apply 999` (→1).
- [ ] **T2.5 [A]** Commit: "feat: ncharge-apply core logic + tests".

## Fase 3 — GUI dasar `ncharge` (M2)

- [ ] **T3.1 [A]** `crates/gui`: `adw::Application`, Window Adwaita 400px, non-resizable, layout mengacu ke `design/ncharge-screens.png` (PRD §9).
- [ ] **T3.2 [A]** Battery status card: baca sysfs langsung (`fs::read_to_string`, tanpa subprocess); loop update 2 detik via `glib::timeout_add_seconds_local`; tampilkan persentase, status, bar level dengan garis penanda limit, `Active limit` (+ sublabel `Until next restart, then N%` bila override ada), serta banner `Above the limit...` bila `capacity > active_limit`.
- [ ] **T3.3 [A]** Preset chips 60 / 80 / 100 dengan label Inggris (`Max lifespan` / `Everyday` / `Full capacity`), teks penjelasan sesuai tabel FR-7, baris `Not saved. The battery still stops at N%.` saat pilihan belum disimpan, dan tombol `Save` nonaktif bila pilihan sama dengan state aktif.
- [ ] **T3.4 [A]** Save: spawn `pkexec /usr/local/bin/ncharge-apply apply <N>` (atau `apply 100 --once` untuk preset 100; selama dev: path binary bisa lewat env var); tangani exit code pkexec `126` (dibatalkan user → toast cancel, chip kembali) dan `127` (tidak diizinkan → toast not authorized, chip kembali).
- [ ] **T3.5 [A]** Toast: success / once / cancel / denied / error (libadwaita `Toast`), persis tabel FR-7 & FR-8.
- [ ] **T3.6 [A→U]** Manual run GUI di sesi GNOME (user yang jalankan, GUI butuh session display): `cargo run -p ncharge-gui`, user menekan chips & Save.
      Cek: toast muncul; sysfs ter-update setelah Save; cancel (exit 126) aman.
- [ ] **T3.7 [A]** State "tidak didukung" (FR-9, layar 4 desain): window fallback `Charge limit not available` saat sysfs absent; Save disabled, di-test dengan env var fake path kosong.
- [ ] **T3.8 [A]** Commit: "feat: GUI dasar + Save via pkexec".
- [ ] **T3.9 [A→U]** Ukur RSS GUI awal di M2 (`/usr/bin/time -v` atau `ps -o rss= -p <PID>`), catat di `docs/bench.md` untuk memastikan di dalam target NFR-1 sebelum lanjut ke M3/M4.

## Fase 4 — Persistensi & packaging plumbing (M3)

- [ ] **T4.1 [A]** `packaging/systemd/ncharge-apply.service` (struktur persis PRD FR-5.1, lengkap dengan `[Install] WantedBy=multi-user.target`, `RestartSec=5`, `StartLimitBurst=10`, `RestartPreventExitStatus=1`); `packaging/sleep/ncharge` (hook pada fase `post` → `ncharge-apply restore`).
- [ ] **T4.2 [A]** `packaging/udev/99-ncharge.rules` (FR-5.3, `SUBSYSTEM=="platform"`, hanya dipasang dengan flag `--with-udev`).
- [ ] **T4.3 [A]** `install.sh` (idempotent, langkah sesuai PRD FR-11; **tidak** menjalankan `cargo build` sebagai root, melainkan cek `$EUID -eq 0` dan cek binary di `target/release/`):
      copy 2 binary → pasang unit + enable → sleep hook → `.desktop` + icon → tulis config default 80 bila belum ada (`0644`) → `ncharge-apply restore` → print status akhir.
- [ ] **T4.4 [A]** `uninstall.sh` (idempotent; stop/disable + remove unit/hook/rule/desktop/override; prompt konfirmasi HAPUS config dengan klarifikasi konsekuensi nilai kembali ke 100).
- [ ] **T4.5 [A]** **Dry-run safety check**, TANPA memasang apa pun di mesin: semua path dicek lagi, `bash -n packaging/install.sh packaging/uninstall.sh` (syntax check); unit + hook + rule direview mata; nama file tidak typo.
- [ ] **T4.6 [A→U]** Build & pasang bersama (user menjalankan):
      ```bash
      cargo build --release
      sudo bash packaging/install.sh
      systemctl status ncharge-apply.service          # oneshot passed/exited
      cat /sys/class/power_supply/BAT0/charge_control_end_threshold
      ```
      Cek: sysfs = 80 (atau config); service enabled, exit sukses; icon muncul di app drawer (H1).
- [ ] **T4.7 [A→U]** Runbook test (manual test plan PRD §10.1, user eksekusi, asisten membantu memverifikasi output):
      1. Set 80 charging → verify `Not charging` at 80 (**H4**, T1)
      2. `reboot` → verify sysfs dan journal (**H5**, T2)
      3. Suspend → resume → verify (**T3**)
      4. Hibernate → resume → verify sleep hook (**H6**, T4)
      5. Set 100 → Save (`--once`) → verify `/etc/ncharge.conf` tetap 80 & `/run/ncharge.override` = 100; reboot → balik 80 (**H13**, T5)
      6. Cancel pkexec → verify no-write (**H7**, T6)
      User checklist centang; asisten menyimpan hasil di `docs/testing-log.md`.
- [ ] **T4.8 [A]** Commit: "feat: persistence + packaging (install/uninstall/unit/hook/udev)".

## Fase 5 — Polish & animasi (M4)

- [ ] **T5.1 [A]** Spring/jelly animation: `AdwSpringAnimation` di chips (scale bounce, damping 0.55), label cross-fade; cocockan detail spasi, warna `#1C71D8` / `#78AEED`, dan 6 state dengan `design/ncharge-screens.png`.
- [ ] **T5.2 [A]** Dark/light check: window render rapi di dua mode (user screenshot konfirmasi).
- [ ] **T5.3 [A]** A11y pass: keyboard navigation chips/Save; accessible labels; kontras teks ≥ 4.5:1.
- [ ] **T5.4 [A]** Benchmark final (update `docs/bench.md`): RSS GUI, startup wall-time, ukuran disk binary. Target: H9/H10.
- [ ] **T5.5 [A→U]** UX sign-off: user pakai app 1 hari; feedback bila ada yang mengganjal; fine-tune kecil.
- [ ] **T5.6 [A]** Commit: "feat: polish, animations, a11y; bench results".

## Fase 6 — README & open-source readiness (M5)

- [ ] **T6.1 [A]** Ikon final SVG + `.desktop` name/comment FIX (FR-10).
- [ ] **T6.2 [A]** README: tagline, **kenapa ncharge dibuat**, tabel banding TLP/asusctl/ext (dari PRD), requirement (sysfs exposure, systemd ≥ 244), install/uninstall, FAQ (EC reset, hibernate, preset 100 sementara, dual-boot MyASUS, "harus pakai TLP?"), screenshot, LICENSE MIT.
- [ ] **T6.3 [A]** Final audit repo: no secret, no binary besar; `cargo fmt --check`, clippy clean; semua H1–H13 re-check kembali.
- [ ] **T6.4 [A→U]** Bersihkan git history (bila perlu compact); siap remote publik (GitHub), user login GitHub interaktif.
- [ ] **T6.5 [A]** Tag: commit "chore: release v1.0.0" → `git tag v1.0.0`.

## Fase 7 — Release (M6)

- [ ] **T7.1 [A→U]** Push ke remote (user jalankan / beri akses). Verify tag.
- [ ] **T7.2 [A]** Post-release note: tulis ringkasan hasil + testing-log final + cara feedback ke README/wiki.

---

## Milestone → DoD ringkas

| Fase | Milestone (PRD) | DoD fase |
|---|---|---|
| 0–1 | M0 | toolchain OK, workspace build, PRD/goal/task/design masuk git |
| 2 | M1 | helper (`apply`, `apply --once`, `restore`, `status`) unit-tested + smoke root OK |
| 3 | M2 | GUI jalan + Save via pkexec OK + RSS awal terukur |
| 4 | M3 | persist terpasang; T1–T6 pass (runbook user) |
| 5 | M4 | animasi + polish sesuai 6 layar desain; benchmark pass |
| 6–7 | M5–M6 | README/repo/tag ✓ — DONE akhir proyek |

## Aturan kalau mentok (contingency)

- GUI GTK build gagal di fase 0/3 → cek versi lib via `pkg-config --modversion gtk4 libadwaita-1`; kalau versi distro tersendat terlalu lama → diskusi pivot toolkit (mis. relm4 yang tetap GTK4, atau alternatif lain) SEBELUM lanjut.
- Persist gagal pada T4.7 (sysfs kembali 100 setelah reboot) → cek `journalctl -b -u ncharge-apply.service`; kalau race load modul yang telat → pasang `--with-udev` atau dokumentasikan early module load initramfs (PRD §12).
- pkexec dialog tidak muncul di session Wayland → cek polkit agent (`systemctl status polkitd`); jangan langsung mutasi kode, debug dulu lewat terminal (`pkexec true`) untuk isolasi masalahnya.
