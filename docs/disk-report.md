# Laporan Penggunaan dan Pembersihan Disk VPS

Tanggal: 2026-11-02
Hostname: VPS (Debian GNU/Linux 12 / Linux 6.1.0-52-cloud-amd64)

## 1. Pengukuran Awal (Sebelum Pembersihan)

### Filesystem & Inodes
```
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        20G   18G  809M  96% /

Filesystem      Inodes  IUsed   IFree IUse% Mounted on
/dev/sda1      1302528 263741 1038787   21% /
```

### Penelusuran Direktori Terbesar (`du -xh --max-depth=2 /`)
- `/home` (11G)
  - `/home/saryu` (11G)
- `/usr` (2.8G)
  - `/usr/lib` (1.8G)
  - `/usr/share` (688M)
  - `/usr/bin` (238M)
- `/var` (365M)
  - `/var/lib` (183M)
  - `/var/cache` (119M)
  - `/var/log` (61M)
- `/boot` (69M)
- `/tmp` (11M)

### Rincian `/home/saryu`
- `/home/saryu/horizon-android` (4.6G)
  - `/home/saryu/horizon-android/target`: **4.1G** (build artifact debug lama)
  - `/home/saryu/horizon-android/.git`: 216M
- `/home/saryu/.hermes`: 2.7G (aplikasi/tools eksternal - jangan disentuh)
- `/home/saryu/.local`: 1.2G
- `/home/saryu/.rustup`: 614M
- `/home/saryu/.cargo`: 396M
  - `/home/saryu/.cargo/registry/src`: 308M
  - `/home/saryu/.cargo/registry/cache`: 29M
  - `/home/saryu/.cargo/registry/index`: 39M
- `/home/saryu/.nvm`: 340M
- `/home/saryu/.gradle`: 166M

### Penyebab Penumpukan Utama
1. `/home/saryu/horizon-android/target` menghabiskan **4.1G** untuk file build debug lama.
2. `~/.cargo/registry/src` menghabiskan **308M** unpack cache crate.
3. `~/.cargo/registry/cache` menghabiskan **29M** download crate cache.

---

## 2. Eksekusi Pembersihan

Waktu: 2026-11-02

| Path / Item | Ukuran Sebelum | Aksi | Hasil / Status |
|---|---|---|---|
| `/home/saryu/horizon-android/target` | 4.3 GiB | `cargo clean` | Berhasil dihapus (5421 file) |
| `~/.cargo/registry/cache/*` | 29 MB | `rm -rf` | Berhasil dibersihkan |
| `~/.cargo/registry/src/*` | 308 MB | `rm -rf` | Berhasil dibersihkan |

---

## 3. Pengukuran Akhir (Setelah Pembersihan)

```
Filesystem      Size  Used Avail Use% Mounted on
/dev/sda1        20G   14G  5.3G  72% /

Filesystem      Inodes  IUsed   IFree IUse% Mounted on
/dev/sda1      1302528 233279 1069249   18% /
```

- Total ruang bebas saat ini: **5.3 GB** (memenuhi syarat minimal >= 5 GB bebas).
- Inode bebas: 1.069.249 (tersedia 82%).
