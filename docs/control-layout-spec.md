# Spesifikasi Clean Room: Sistem Kontrol Sentuh dan Editor Tata Letak (Control Layout)

## 0. Informasi Lisensi dan Kebijakan Clean Room
- **Sumber Acuan Perilaku**: ZalithLauncher2 (ZL2).
- **Lisensi Sumber**: GNU General Public License v3.0 (GPL-3.0).
- **Metode**: Clean Room Reverse Engineering. Dokumen ini hanya memuat spesifikasi fungsional, arsitektur informasi, model data interoperabilitas, dan aturan perilaku runtime. Dokumen ini **tidak** memuat salinan kode, komentar, nama variabel/fungsi internal berhak cipta, atau aset visual dari ZL2.
- **Tujuan**: Implementasi mandiri fitur kontrol sentuh, manajer tata letak, dan editor tata letak pada `horizon-android` dengan paritas perilaku dan posisi menu 1:1.

---

## 1. Alur Navigasi dan Lokasi Entri

### 1.1 Lokasi Entri di Pengaturan Launcher
Pengaturan launcher memiliki 8 kategori utama dengan urutan tetap:
1. **Renderer** (Pengaturan Grafis / Video)
2. **Game** (Pengaturan Game / Peluncuran)
3. **Control** (Pengaturan Perilaku Input: Mouse Fisik, Sensitivitas Kursor, Delay Tap/Hold, Gestur Kamera, Giroskop)
4. **Gamepad** (Pengaturan Kontroler / Gamepad Fisik)
5. **Launcher** (Pengaturan Aplikasi Peluncur)
6. **Java Manager** (Manajer Runtime Java)
7. **Control Manager** (Manajer Tata Letak Kontrol / Layout Manager) -> **Titik Masuk Utama Editor**
8. **About / Info** (Informasi Aplikasi)

Entri **Control Manager** berada persis di urutan ke-7 pada bilah navigasi/tab Pengaturan, tepat di bawah Java Manager dan di atas About.

### 1.2 Peta Alur Layar
```
[Layar Utama Launcher]
       │
       ▼ (Buka Pengaturan)
[Pengaturan: Tab 7 - Control Manager]
       │
       ├── Top Bar Aksi: [Refresh] [Import File (.json)] [Buat Layout Baru (+)]
       │
       ├── Daftar Layout Tersimpan (Radio Button: pilih aktif, Copy, Delete)
       │      │
       │      └── Kartu Detail Layout Terpilih:
       │             - Info: Nama, Pembuat, Versi, Deskripsi
       │             - [Bagikan / Ekspor]
       │             - [Edit Tata Letak] ──────────────────────────┐
       │                                                           ▼
       │                                                 [Editor Tata Letak]
       │                                                 (Layar Penuh / Activity)
       │                                                           │
       │                                           ┌───────────────┴───────────────┐
       │                                           ▼                               ▼
       │                                    [Menu Kiri]                      [Menu Kanan]
       │                             (Tambah Kontrol, Style,            (Daftar Layer,
       │                              Pratinjau, Simpan/Keluar)         Fokus, Properti Layer)
       │
       ▼ (Saat Game Berjalan)
[Layar Permainan / In-Game Overlay]
       │
       ├── Tombol GUI / Menu Launcher
       │      │
       │      ▼
       └── [Dialog Ganti Kontrol / ReplacementControlDialog]
              - Memilih layout lain secara langsung tanpa restart game
```

### 1.3 Akses dari Dalam Game
- Tersedia dialog cepat **Replacement Control Dialog** yang dapat dipanggil melalui `launcher_event` (mis. tombol GUI in-game).
- Menampilkan daftar tata letak yang tersedia beserta indikator tata letak aktif saat ini.
- Memilih tata letak langsung memuat ulang konfigurasi layer dan tombol secara asinkron tanpa memutus koneksi server atau mematikan sesi game.

---

## 2. Manajer Tata Letak (Layout Manager)

### 2.1 Operasi Tata Letak
- **Daftar Layout**: Membaca seluruh file `.json` dari direktori penyimpanan layout (`control_layouts/`).
- **Pilihan Aktif**: Radio button pada tiap item. Hanya satu layout yang aktif pada satu waktu, disimpan pada pengaturan launcher (`active_control_layout`).
- **Buat Baru (Create)**: Membuka dialog input (Nama Layout, Pembuat, Nomor/Nama Versi). Menghasilkan file layout baru dengan konfigurasi layer bawaan minimal.
- **Duplikasi (Copy)**: Menyalin file layout yang ada dengan nama baru (ditambah penanda salinan) dan menghasilkan UUID baru untuk seluruh elemen.
- **Impor**: Mendukung seleksi file tunggal maupun multi-file `.json`. Melakukan validasi struktur dan versi editor sebelum disimpan ke direktori layout.
- **Ekspor / Bagikan**: Menyalin/mengirimkan file `.json` layout terpilih ke aplikasi lain melalui mekanisme share sistem atau direktori penyimpanan dokumen.
- **Hapus (Delete)**: Menampilkan dialog konfirmasi penghapusan. Menolak penghapusan jika hanya tersisa satu layout default.
- **Penanganan Kesalahan**:
  - File JSON rusak/tidak valid: Diberi tanda status error pada daftar tanpa membuat launcher crash.
  - Versi editor di atas versi yang didukung: Menampilkan pesan peringatan format tidak didukung.

---

## 3. Model Data Tata Letak (Interoperabilitas Schema)

Format penyimpanan adalah JSON UTF-8 dengan struktur hierarkis:
`ControlLayout` -> `ControlLayer` -> `Widget` (Normal Button, Text Box, Joystick).

### 3.1 Struktur Akar: `ControlLayout`
- `editorVersion`: Bilangan bulat (versi schema saat ini: `12`).
- `info`: Objek informasi metadata.
  - `name`: `TranslatableString`
  - `author`: `TranslatableString`
  - `description`: `TranslatableString`
  - `versionCode`: Integer
  - `versionName`: String
- `layers`: Array objek `ControlLayer`.
- `styles`: Array objek `ButtonStyle`.
- `joystickStyles`: Array objek `JoystickStyle`.

### 3.2 Tipe Umum
- **`TranslatableString`**:
  ```json
  {
    "default": "Default Name",
    "matchQueue": [
      { "language_tag": "id", "value": "Nama Bawaan" },
      { "language_tag": "zh", "value": "預設" }
    ]
  }
  ```
- **`ButtonPosition`**:
  Koordinat normalisasi resolusi tinggi `0..10000` (mewakili 0.00% hingga 100.00% dimensi layar).
  - `x`: Integer (0 = tepi kiri, 10000 = tepi kanan)
  - `y`: Integer (0 = tepi atas, 10000 = tepi bawah)
- **`ButtonSize`**:
  - `type`: `"percentage"` atau `"dp"`.
  - `widthDp` / `heightDp`: Float (minimal 5.0 dp).
  - `widthPercentage` / `heightPercentage`: Integer `100..10000` (1% hingga 100%).
  - `widthReference` / `heightReference`: `"screen_width"` atau `"screen_height"`.
- **`ButtonShape`**:
  Radius sudut border (persentase kelengkungan `0.0..100.0`):
  - `topStart`, `topEnd`, `bottomEnd`, `bottomStart`.
- **Warna**: Integer 32-bit format ARGB (misal `0x80000000` = 2147483648 untuk hitam transparan 50%).

### 3.3 Struktur Layer: `ControlLayer`
- `name`: String nama layer.
- `uuid`: String UUID unik layer.
- `hide`: Boolean (apakah layer disembunyikan secara default).
- `hideWhenMouse`: Boolean (sembunyikan layer saat kursor mouse fisik aktif/bergerak, default: true).
- `hideWhenGamepad`: Boolean (sembunyikan layer saat gamepad terhubung/aktif, default: true).
- `visibilityType`:
  - `"always"`: Selalu tampak.
  - `"in_game"`: Hanya tampak saat kursor virtual terkunci (mode bermain).
  - `"in_menu"`: Hanya tampak saat kursor virtual bebas (mode menu/inventaris).
- `normalButtons`: Array objek tombol normal.
- `textBoxes`: Array objek label teks display.
- `joystickButtons`: Array objek kontrol analog joystick.

### 3.4 Tombol Normal: `NormalData`
- `uuid`: String UUID.
- `text`: `TranslatableString`.
- `position`: `ButtonPosition`.
- `buttonSize`: `ButtonSize`.
- `buttonStyle`: String UUID style tombol (opsional).
- `textAlignment`: `"left"`, `"center"`, `"right"`.
- `textBold`, `textItalic`, `textUnderline`: Boolean.
- `visibilityType`: `"always"`, `"in_game"`, `"in_menu"`.
- `clickEvents`: Array objek `ClickEvent`.
- `isSwipple`: Boolean (mendukung gestur usap / swipe berantai antar tombol).
- `isPenetrable`: Boolean (tembus sentuhan: sentuhan diteruskan ke lapisan di bawahnya/kamera).
- `isToggleable`: Boolean (mode toggle on/off alih-alih tekan-lepas).

### 3.5 Kotak Teks: `TextData`
- Elemen non-interaktif untuk menampilkan status atau label.
- Memiliki properti `uuid`, `text`, `position`, `buttonSize`, `buttonStyle`, perataan teks, dan `visibilityType`.

### 3.6 Tombol Joystick: `JoystickData`
- `uuid`: String UUID.
- `position`: `ButtonPosition`.
- `sizeType`: `"percentage"` atau `"dp"`.
- `sizeDp`: Float (default 200 dp, minimal 20 dp).
- `sizePercentage`: Integer (default 2500 = 25%, minimal 2000).
- `visibilityType`: `"always"`, `"in_game"`, `"in_menu"`.
- `joystickStyleId`: String UUID style joystick (opsional).
- `deadZoneRatio`: Float `0.0..0.9` (rasio zona mati tengah, default 0.5).
- `lockThreshold`: Float `0.0..1.0` (ambang batas geser luar untuk mengunci arah, default 0.3).
- `canLock`: Boolean (apakah fitur kunci sprint/arah aktif, default true).
- `triggerMode`: `"drag"` (geser kontinu) atau `"tap"` (ketuk sektor).
- `directionEvents`: Map arah (`forward`, `back`, `left`, `right`) ke daftar `ClickEvent`.
  - Default: `forward` -> `GLFW_KEY_W`, `back` -> `GLFW_KEY_S`, `left` -> `GLFW_KEY_A`, `right` -> `GLFW_KEY_D`.
- `lockEvents`: Daftar `ClickEvent` yang dipicu saat joystick terkunci (default: `GLFW_KEY_LEFT_CONTROL` untuk sprint).

### 3.7 Gaya Tampilan: `ButtonStyle` & `JoystickStyle`
- Memisahkan konfigurasi tampilan terang (`lightStyle`) dan gelap (`darkStyle`), atau berbagi konfigurasi sama (`commonStyle = true`).
- Properti tombol mencakup: `alpha`, `pressedAlpha`, `backgroundColor`, `pressedBackgroundColor`, `contentColor`, `pressedContentColor`, `fontSize`, `borderWidth`, `borderColor`, dan `borderRadius`.
- Properti joystick mencakup: warna latar, warna knob netral, warna knob siap kunci, warna knob terkunci, warna tanda kunci, ketebalan border, dan bentuk radius knob.

---

## 4. Sistem Event dan Pemetaan Input

### 4.1 Jenis `ClickEvent`
Setiap event memiliki bidang `type` dan `key`:
1. **`key`**: Memetakan ke tombol fisik keyboard (kode GLFW/AWT, contoh: `GLFW_KEY_W`, `GLFW_KEY_SPACE`, `GLFW_KEY_E`) atau tombol mouse (`GLFW_MOUSE_BUTTON_LEFT`, `GLFW_MOUSE_BUTTON_RIGHT`, `GLFW_MOUSE_BUTTON_MIDDLE`).
2. **`launcher_event`**: Aksi internal launcher:
   - `LAUNCHER_EVENT_SWITCH_IME`: Buka/tutup keyboard layar (virtual OSK).
   - `LAUNCHER_EVENT_SWITCH_MENU`: Buka/tutup menu pause in-game.
   - `LAUNCHER_EVENT_SCROLL_UP` / `LAUNCHER_EVENT_SCROLL_DOWN`: Scroll mouse terus-menerus.
   - `LAUNCHER_EVENT_SCROLL_UP_SINGLE` / `LAUNCHER_EVENT_SCROLL_DOWN_SINGLE`: Scroll mouse 1 takik.
3. **`switch_layer`**: Membalik status sembunyi/tampil layer tujuan berdasarkan UUID layer.
4. **`show_layer`**: Memaksa layer tujuan tampil berdasarkan UUID.
5. **`hide_layer`**: Memaksa layer tujuan tersembunyi berdasarkan UUID.
6. **`send_text`**: Mengirim string teks perintah atau chat secara otomatis ke game.

### 4.2 Siklus Hidup dan Pelepasan Event
- **Momentary (Biasa)**: Pointer down mengirim event KeyDown; Pointer up mengirim event KeyUp.
- **Toggle (`isToggleable = true`)**: Setiap ketukan membalik status tombol (aktif <-> nonaktif). Saat aktif, status tombol tetap ditekan di engine game sampai diketuk kembali.
- **Pelepasan Aman Saat Layer Tersembunyi (Anti-Stuck)**:
  - Jika sebuah layer disembunyikan (melalui `switch_layer`, `hide_layer`, atau pemicu mouse/gamepad), SELURUH tombol pada layer tersebut yang sedang dalam status tertekan WAJIB segera memicu event KeyUp/Release ke engine input game.
- **Pelepasan Out-of-Bounds**:
  - Jika pointer keluar dari batas area tombol dan tombol bukan tipe `isSwipple`, tombol otomatis dilepas (`onReleaseEvent`).

---

## 5. Pipeline Sentuhan (Touch Pipeline)

### 5.1 Multi-Touch dan Manajemen Pointer
- Setiap jari/sentuhan memiliki `pointerId` unik yang dicatat dalam bus sesi pointer.
- Pelacakan dilakukan per-pointer independen: sentuhan tombol A dengan jari 1 tidak membatalkan atau mengganggu sentuhan tombol B dengan jari 2.
- **Pointer Capture**: Saat sebuah tombol menerima PointerDown, `pointerId` tersebut ditangkap (captured) oleh widget tersebut hingga PointerUp diterima atau pointer keluar dari batas area (out of bounds).

### 5.2 Penetrasi Sentuhan (`isPenetrable`)
- Kontrol non-penetrable mengonsumsi pointer event sehingga area di bawahnya tidak menerima input.
- Kontrol dengan `isPenetrable = true` mengirim event tombol tetapi TIDAK mengonsumsi event gestur, sehingga sentuhan diteruskan ke lapisan kontrol kamera 3D di bawahnya.

### 5.3 Gestur Berantai (Swipable Chain)
- Tombol dengan `isSwipple = true` mengizinkan pointer meluncur keluar dari tombol menuju tombol swipable lain tanpa melepaskan gestur secara kaku, memungkinkan gerakan cepat melintasi tombol hotbar atau aksi kombinasi.

### 5.4 Deteksi Mouse dan Gamepad Fisik
- Masukan dari mouse USB/Bluetooth fisik (pergerakan kursor atau klik tombol) memicu transisi `HideLayerWhen.WhenMouse`. Semua layer dengan `hideWhenMouse = true` disembunyikan sementara.
- Masukan sentuh kembali mengaktifkan tampilan layer.

---

## 6. Sistem Joystick dan Sensor Analog

### 6.1 Perilaku Analog Joystick
- **Normalisasi Vektor**:
  $$\Delta x = x_{pointer} - x_{center}, \quad \Delta y = y_{pointer} - y_{center}$$
  $$r = \sqrt{\Delta x^2 + \Delta y^2} / R_{max}$$
- **Zona Mati (Dead Zone)**:
  Jika $r < \text{deadZoneRatio}$, vektor masukan dianggap nol (mencegah pergerakan tidak disengaja).
- **Penentuan Sektor Arah (8 Arah)**:
  Sudut $\theta = \operatorname{atan2}(\Delta y, \Delta x)$:
  - Sektor Maju (W), Mundur (S), Kiri (A), Kanan (D), serta kombinasi diagonal (W+D, W+A, S+D, S+A).
- **Penguncian Sprint (Lock Threshold)**:
  Jika $r > (1.0 + \text{lockThreshold})$, joystick masuk ke status TERKUNCI (locked).
  - Menyalakan `lockEvents` (Sprint / Left Control).
  - Knob tetap berada di posisi atas terkunci meskipun jari dilepaskan, hingga disentuh kembali.

### 6.2 Sensor Giroskop
- Mendengarkan sensor rotasi/giroskop hardware Android.
- Sinyal pitch dan yaw diubah menjadi delta pandangan kamera kursor (rotasi mouse).
- Parameter konfigurasi: Sensitivitas, Laju Sampel (Hz), Smoothing Window, Invert Axis X, Invert Axis Y.

---

## 7. Editor Tata Letak (Layout Editor)

### 7.1 Tata Letak Layar Editor
Editor beroperasi dalam mode layar penuh (landscape) dengan antarmuka Dual-Drawer:
- **Area Kerja Tengah**: Pratinjau kanvas kontrol interaktif dengan panduan garis snapping.
- **Bilah Menu Kiri (Editor Menu)**:
  - Tombol Tambah Tombol Normal
  - Tombol Tambah Teks
  - Tombol Tambah Joystick
  - Kelola Gaya Tombol (Style List)
  - Kelola Gaya Joystick (Joystick Style List)
  - Beralih Mode Pratinjau (Interactive Preview on/off)
  - Opsi Skenario Pratinjau (In Game / In Menu)
  - Simpan (Save)
  - Simpan & Keluar (Save and Exit)
  - Keluar Tanpa Menyimpan (Exit)
- **Bilah Menu Kanan (Layers Menu)**:
  - Tombol Tambah Layer Baru
  - Mode Fokus Layer (hanya menampilkan kontrol pada layer yang sedang dipilih)
  - Daftar Layer (dapat diurutkan kembali / reorderable)
  - Per-layer: tombol visibilitas (mata buka/tutup), tombol buka dialog atribut layer

### 7.2 Manipulasi Elemen (Drag, Resize, Snap)
- **Seleksi**: Mengetuk elemen di kanvas memilih elemen tersebut dan menampilkan kotak pembatas (bounding box).
- **Drag (Pindah Posisi)**: Menyeret bagian tengah elemen memperbarui `position` dalam satuan normalisasi `0..10000`.
- **Resize (Ubah Ukuran)**:
  - Dilengkapi dua handle sudut: **Top-Left** (mengubah ukuran sekaligus posisi) dan **Bottom-Right** (mengubah ukuran dengan posisi tetap).
  - Batas ukuran minimal ditegakkan (minimal 5 dp / 1%).
  - Elemen joystick menjaga rasio aspek 1:1 saat di-resize.
- **Snapping**:
  - Garis bantu snapping otomatis muncul ketika tepi atau titik tengah elemen sejajar dengan elemen lain (dalam radius threshold 4 dp).

### 7.3 Dialog Pengaturan Properti Elemen
- **Tab Properti Dasar**: Teks, Ukuran (satuan & nilai), Posisi presisi, Style yang diterapkan, Perataan, Format teks (Bold, Italic, Underline), Visibilitas (Always / In Game / In Menu).
- **Tab Event**: Daftar event terikat, konfigurasi `isToggleable`, `isPenetrable`, `isSwipple`.
- **Aksi Elemen**:
  - **Duplikasi**: Membuat salinan elemen dengan UUID baru dan posisi sedikit bergeser (+200 koordinat).
  - **Hapus**: Menghapus elemen dari layer saat ini.

---

## 8. Nilai Default dan Kasus Tepi

### 8.1 Layout Bawaan (Default Preset)
Jika pengguna belum memiliki layout, sistem mengekstrak `default_layout.json` yang memuat:
1. Layer `gui`: Tombol "GUI" di sudut kanan atas untuk membuka menu.
2. Layer `tab`: Tombol bantuan fungsi (F3, F5, Tab, Chat, Keyboard).
3. Layer `control`: Tombol aksi game (Lompat / Space, Jongkok / Shift).
4. Layer `layer2`: Tombol inventaris, drop item, tukar tangan.
5. Layer `move`: Joystick analog pergerakan di sudut kiri bawah dengan event W/A/S/D dan kunci sprint.

### 8.2 Adaptasi Resolusi dan Rasio Aspek
- Karena seluruh posisi dinyatakan dalam persepuluh ribu (`0..10000`), tata letak menyesuaikan secara proporsional di semua layar Android (16:9, 18:9, 20:9, tablet 4:3, mode layar lipat, dan mode multi-window / PiP).
- Perubahan orientasi atau ukuran jendela memicu kalkulasi ulang ukuran pixel elemen secara otomatis tanpa mengubah data layout tersimpan.
