# SVC (Simple Voice Chat) Port Blueprint: Horizon Ecosystem (Desktop & Android)

Dokumen ini adalah cetak biru arsitektur aktual, spesifikasi perubahan terbaru (termasuk **RNNoise + Voice-Gated AGC**, **Real-Time VoiceMixer**, dan **Opus VOIP Stateful**), alasan teknis mengapa **SVC wajib bersifat *built-in* (mustahil dijadikan `.hnmod` di Android)**, serta **syarat wajib kompilasi (`build-android` & CI)** di dalam ekosistem **Horizon (`horizon-client` / Rust / wgpu)**.

---

## 1. Peta Arsitektur Aktual di Repositori `horizon-android`

Seluruh subsistem SVC diimplementasikan secara *native* di dalam `crates/horizon-client/src/voice/` dan diintegrasikan langsung ke *game loop* 20 TPS, *audio callback* real-time, serta *renderer HUD*:

| File Modul | Tanggung Jawab Utama |
|---|---|
| `crates/horizon-client/src/voice/mod.rs` | State utama `VoiceChatClient`, negosiasi *handshake* TCP, fallback versi otomatis, orkestrator 20 TPS `tick_audio()`, resolusi alias Gamertag/UUID (`verified_svc_senders`), dan pengiriman target gain 3D ke `VoiceMixer`. |
| `crates/horizon-client/src/voice/protocol.rs` | Serialisasi `ByteBufReader`/`ByteBufWriter` (dengan proteksi batas memori/OOM), kriptografi `SvcCipher` (AES-128-GCM), 14 paket TCP `voicechat:*`, 10 paket UDP, dan parser `parse_voice_host`. |
| `crates/horizon-client/src/voice/udp_worker.rs` | Thread background non-blocking (`svc-udp-worker`), *4-step UDP handshake* (`0x05` -> `0x06` -> `0x09` -> `0x0A`), *heartbeat* `KeepAlive (0x08)` & `Ping (0x07)`, serta dekode langsung paket suara masuk ke `VoiceMixer`. |
| `crates/horizon-client/src/voice/audio_pipeline.rs` | Pemrosesan frame PCM 48kHz Mono 20ms (960 sampel), *dual sub-frame* **RNNoise** (2x 480 sampel via `nnnoiseless`), VAD, **Voice-Gated Mic AGC** + *soft clipper* `tanh`, encoding/decoding **Opus VOIP 32 kbps stateful** (`audiopus 0.2`), **Packet Loss Concealment (PLC)**, dan kurva atensi jarak resmi SVC (`compute_spatial_gains`). |
| `crates/horizon-client/src/voice/mixer.rs` | **Real-time Per-Sender Jitter Buffer & Stereo Mixer (`VoiceMixer`)**: *prebuffer* 80ms (4 frame) per pengirim, *fade-in/fade-out* 5ms (`FADE_SAMPLES = 240`), peluruhan *tail* saat *underrun*, *one-pole gain smoothing* per sampel (`GAIN_SMOOTHING = 0.002`), penjumlahan multi-pengirim, dan *soft limiter* output. |
| `crates/horizon-client/src/voice/hardware_io.rs` | Abstraksi perangkat keras audio lintas-platform berbasis `cpal` (`dep:cpal` pada fitur `audio`): *input stream* mikrofon dengan *linear interpolation resampling* ke 48kHz, serta *output stream* speaker yang menarik sampel dari `VoiceMixer` melalui `OutputResampler`. |
| `crates/horizon-client/src/voice/config.rs` | Konfigurasi persisten `svc_protocol.json` (versi kompatibilitas, daftar *fallback* `[20, 21, 19, 18, 22]`, *mute*, *disable*, *VAD threshold*, *mic gain*, *output volume*, volume/mute per-pemain, dan *RNNoise toggle*). |
| `crates/horizon-client/src/voice/ui.rs` & `src/game/voice_ui.rs` | State layar pengaturan Voice Chat (`Screen::VoiceChat`), filter daftar pemain (`All` / `SVC Only`), indikator status HUD kiri-bawah (`VoiceHudStatus`), dan pemeriksaan izin mikrofon runtime Android (`ensure_microphone_permission`). |
| `crates/horizon-client/src/game/remote.rs` | Pemicu `initiate_voice_handshake()` saat join server, delegasi `CustomPayload("voicechat:*")`, dan sinkronisasi 20 tick `tick_voice_chat()` dengan resolusi posisi entitas 2-tahap (pencocokan UUID `[i32; 4]` di `self.persist.uuids` -> `r.entities` serta fallback Gamertag untuk server offline/proxy). |
| `tools/android_build.py` & `.github/workflows/` | Konfigurasi *cross-compilation* statis `libopus` (`audiopus_sys`) dan `oboe-sys` (`cpal`) terhadap Android NDK (`llvm-ar`, `llvm-ranlib`, `CONFIG_SITE`, `CMAKE_POLICY_VERSION_MINIMUM=3.5`). |

---

## 2. Letak & Cara Kerja RNNoise serta Voice-Gated Mic AGC

Seluruh logika **RNNoise (Neural Noise Suppression + VAD)** dan **Voice-Gated Automatic Gain Control (AGC)** berada di dalam **`crates/horizon-client/src/voice/audio_pipeline.rs`** pada struct **`VoiceCapturePipeline`**:

| Komponen | Lokasi File & Baris | Detail Implementasi |
|---|---|---|
| **Struct & State AGC** | `audio_pipeline.rs` baris `33–53` | `VoiceCapturePipeline` menyimpan `denoiser: Box<DenoiseState<'static>>`, `encoder: OpusEncoder` (48kHz Mono VOIP, bitrate `32_000` bps), dan `pub agc_gain: f32` (nilai awal `1.5`, di-reset ke `1.5` pada `reset_encoder()` di baris `78–81`). |
| **Pre-Gain Mikrofon (`base_gain`)** | `audio_pipeline.rs` baris `102` | `let base_gain = (mic_gain.clamp(0.0, 4.0) * 1.8).clamp(0.0, 7.2);` mengonversi pengaturan `mic_gain` pengguna (dari `config.rs` / slider UI) menjadi penguatan dasar sebelum masuk ke RNNoise. |
| **RNNoise Dual Sub-Frame (2x 480 sampel)** | `audio_pipeline.rs` baris `103–124` | Karena RNNoise (`nnnoiseless`) wajib memproses frame 10ms (480 sampel @ 48kHz) dalam skala 16-bit (`[-32768.0, 32767.0]`), setiap frame 20ms (960 sampel) dibagi menjadi `sub1_in` (`0..480`) dan `sub2_in` (`480..960`), diproses oleh `self.denoiser.process_frame(...)`, dinormalisasi kembali ke `[-1.0, 1.0]`, dan diambil nilai probabilitas suara tertinggi `vad_max = vad1.max(vad2)`. |
| **Voice Gate & Peluruhan AGC Saat Hening** | `audio_pipeline.rs` baris `135–139` | Jika suara tidak melewati ambang VAD (`!vad_passed`), AGC **tidak** menaikkan gain (agar derau latar/napas tidak ikut terangkat), melainkan meluruhkan `agc_gain` secara halus kembali ke baseline: `self.agc_gain = self.agc_gain * 0.95 + 1.5 * 0.05;`. |
| **Voice-Gated AGC (Fast Attack, Smooth Release)** | `audio_pipeline.rs` baris `141–156` | Hanya berjalan pada frame yang terkonfirmasi berisi suara manusia (`frame_peak >= 0.02`). Menghitung target `desired_agc = (0.65 / frame_peak).clamp(1.0, 4.0)`:<br>- **Fast Attack (`0.50 / 0.50`)** saat `desired_agc < self.agc_gain` untuk mencegah *clipping* ketika pengguna tiba-tiba berbicara keras.<br>- **Smooth Release (`0.85 / 0.15`)** saat `desired_agc >= self.agc_gain` untuk mengangkat suara pelan secara bertahap tanpa efek *pumping*. |
| **Soft Clipper Hyperbolic Tangent (`soft_clip_mic`)** | `audio_pipeline.rs` baris `24–30` & `158–160` | Setiap sampel pasca-AGC melewati `soft_clip_mic(*sample * self.agc_gain)` yang bersifat linier hingga `|x| <= 0.70` dan melengkung halus dengan `tanh` di atas `0.70` sebelum dienkode ke Opus. |

---

## 3. Mengapa SVC di Android Mustahil Dijadikan `.hnmod` / Mod Eksternal (Wajib Engine Built-in)

Pada arsitektur Android Horizon, mengimplementasikan Simple Voice Chat sebagai mod eksternal (`.hnmod` / `.hmod` berbasis sandbox WebAssembly/WIT maupun plugin API eksternal) adalah **mustahil secara teknis dan arsitektural** karena alasan kritis berikut:

### 3.1. Sinkronisasi Ketat Game Loop 20 Tick (50ms) vs Frame Audio 20ms (50fps)
- Dalam 1 tick permainan (**50ms** pada 20 TPS), pipeline audio memproses **2,5 frame Opus** (masing-masing berdurasi tepat **20ms** / 960 sampel pada 48.000 Hz), sementara thread callback audio hardware Android (`Oboe` / AAudio via `cpal`) menarik sampel setiap **2–10ms** secara kontinu.
- Setiap tick di `remote.rs` (`tick_voice_chat()`, baris `9257–9360`), engine harus membaca posisi kamera (`self.view.eye`, `self.yaw`) dan melakukan **resolusi entitas 2-tahap** secara langsung pada struktur memori internal `Game`:
  1. Mencocokkan UUID 128-bit pengirim terhadap tabel `self.persist.uuids` dan membaca posisi 3D real-time dari `self.remote.entities` (`Tracked::position`).
  2. Melakukan fallback pencocokan Gamertag melalui `r.tab_players` dan `r.players` untuk server *offline-mode* / BungeeCord / Velocity.
- Jika melewati batas API mod (`.hnmod`), penyalinan tabel entitas dan pemanggilan fungsi lintas-boundary setiap tick + setiap frame audio menimbulkan *overhead* serialisasi dan penjadwalan thread.

### 3.2. Batas Waktu Real-Time Sub-Milidetik (Meleset Beberapa Milidetik = Suara Rusak)
- Pipeline suara memiliki anggaran waktu (*latency budget*) yang sangat ketat:
  - **RNNoise dual sub-frame + Voice-Gated AGC + Stateful Opus Encoder/Decoder** bergantung pada urutan frame 20ms yang presisi dan berkesinambungan.
  - Apabila terjadi keterlambatan (*jitter* / *stall*) **hanya beberapa milidetik saja** akibat *overhead* pemanggilan API mod, *context switch* sandbox WASM, atau *garbage collection*/alokasi memori lintas-batas:
    1. **Buffer Underrun pada Callback Audio**: Antrean sampel di `VoiceMixer` terlambat terisi ketika thread `Oboe`/`cpal` meminta data, menyebabkan suara terputus-putus (*stuttering*), *click/pop*, atau *fade-out* prematur.
    2. **Kerusakan State Prediksi LPC/SILK Opus**: Codec Opus bersifat *stateful* antar-frame; keterlambatan atau *drop* beberapa milidetik memicu *false Packet Loss Concealment (PLC)* atau merusak kontinuitas fase sinyal sehingga suara terdengar robotik/pecah.
    3. **Desinkronisasi Panning 3D**: Pembaruan target gain `(target_l, target_r)` yang terlambat terhadap *per-sample smoothing* (`GAIN_SMOOTHING = 0.002` di `mixer.rs`) membuat arah suara 3D tertinggal atau melompat kasar saat pemain memutar kamera.

### 3.3. Akses Langsung Hardware Audio (`Oboe`), Static `libopus.a`, Socket UDP, & JNI Android
- Sistem mod Horizon (`.hmod` / `.hnmod`) dirancang dengan *content/sandbox fence* demi keamanan dan tidak memiliki akses langsung ke:
  1. Stream audio hardware *low-latency* Android (`AAudio` / `OpenSL ES` via `cpal`/`oboe-sys`).
  2. Simbol C FFI `libopus.a` yang di-*link* statis ke dalam `libhorizon_client.so`.
  3. Thread socket UDP *non-blocking* khusus (`svc-udp-worker`) dengan enkripsi AES-128-GCM per paket.
  4. Pemanggilan JNI langsung ke `Activity` Android (`crate::platform::ensure_microphone_permission()`) untuk meminta izin runtime `android.permission.RECORD_AUDIO`.

Oleh karena itu, SVC di Android **wajib terintegrasi langsung (*built-in*) di dalam crate `horizon-client`**.

---

## 4. Alur Kerja End-to-End (Protokol, UDP Worker, dan VoiceMixer)

### 4.1. Negosiasi TCP Control Channel & Auto-Fallback
1. Saat klien bergabung ke server (`remote.rs`), `VoiceChatClient::initiate_handshake()` mengirimkan paket `CustomPayload` pada channel `voicechat:request_secret` berisi `compatibility_version` (default: `20`).
2. Jika server sedang mengirim/memuat *Server Resource Pack* (`self.server_packs.is_busy()`), timer *handshake* ditunda (`delay_handshake_timer()`) agar tidak terjadi *false timeout*.
3. Jika dalam **4 detik** server belum membalas dengan `voicechat:secret`, `check_handshake_timeout()` otomatis memutar versi ke urutan berikutnya di `fallback_versions` (`[20, 21, 19, 18, 22]`).
4. Ketika `voicechat:secret` diterima, klien mengekstrak kunci AES-128 (`secret`), port UDP, UUID pemain, jarak maksimum suara, serta memulai thread `svc-udp-worker`.

### 4.2. 4-Step UDP Handshake, Enkripsi AES-128-GCM, & Dekode ke `VoiceMixer`
1. **4-Step UDP Handshake**: `0x05 Authenticate` -> `0x06 AuthenticateAck` -> `0x09 ConnectionCheck` -> `0x0A ConnectionCheckAck`.
2. **Dekode Langsung di Thread UDP (`udp_worker.rs` -> `mixer.rs`)**:
   - Paket suara masuk (`0x02 PlayerSound`, `0x03 GroupSound`, `0x04 LocationSound`) didekripsi dengan AES-128-GCM, didekode melalui `VoicePlaybackPipeline::decode_sender_frame` (termasuk penanganan **Opus PLC** hingga `MAX_PLC_FRAMES = 5` saat ada *sequence gap*), lalu didorong langsung ke `VoiceMixer::push()` per `sender_uuid`.
   - Jika menerima paket kosong (`opus_data.is_empty()`), decoder pengirim di-reset dan `VoiceMixer::end_stream(uuid)` dipanggil agar sisa *buffer* dimainkan hingga habis lalu ditutup dengan *fade-out* 5ms (`FADE_SAMPLES = 240`).

### 4.3. Pencampuran Real-Time (`VoiceMixer`) & Kompensasi Volume Speaker
1. **Per-Sender Prebuffer 80ms**: Setiap pengirim di `VoiceMixer` (`crates/horizon-client/src/voice/mixer.rs`) memiliki `SenderStream` tersendiri dengan `PREBUFFER_SAMPLES = FRAME_SIZE * 4` (80ms) dan batas maksimum `MAX_BUFFER_SAMPLES = FRAME_SIZE * 20` (400ms).
2. **Penjumlahan Multi-Pengirim & Per-Sample Gain Smoothing**:
   - Callback output `cpal` memanggil `VoiceMixer::next_frame()`, yang menjumlahkan (*summing*) sinyal seluruh pengirim aktif secara bersamaan (bukan mengantrekan satu per satu), menghaluskan perubahan gain 3D setiap sampel (`GAIN_SMOOTHING = 0.002`), dan membatasi puncak sinyal dengan `soft_limit()`.
3. **Makeup Boost & Kompensasi Ducking Android (`voice/mod.rs:350–380`)**:
   - Menerapkan `SPEAKER_MAKEUP_BOOST = 3.2` serta kompensasi `mic_active_compensation = 1.30` saat stream mikrofon aktif untuk mengatasi penurunan volume otomatis (*communication audio mode ducking*) pada beberapa perangkat Android.

---

## 5. Syarat Wajib Kode Rust & Kompilasi `audiopus 0.2.0`

Untuk mencegah error kompilasi Rust (`E0252` duplicate import dan `E0277` trait bound `TryFrom` tidak terpenuhi pada edisi Rust 2021+), aturan berikut **wajib dipatuhi** di `crates/horizon-client/src/voice/audio_pipeline.rs`:

1. **Jangan gunakan `OpusPacket::try_from` atau `MutSignals::try_from`**:
   - Pada crate `audiopus = "0.2"` (`0.2.0`), tipe `&[u8]` dan `&mut [f32]` mengimplementasikan trait kustom `audiopus::TryInto<Packet<'a>>` dan `audiopus::TryInto<MutSignals<'a, f32>>`, **bukan** `std::convert::TryFrom` ataupun `audiopus::TryFrom`.
   - Memanggil `OpusPacket::try_from(opus_data)` akan bentrok dengan *blanket implementation* `std::convert::TryFrom` dari prelude Rust 2021 (`error[E0277]`).
2. **Operasikan slice secara langsung ke `Decoder::decode_float`**:
   ```rust
   fn decode_packet(&mut self, opus_data: &[u8], out: &mut [f32; FRAME_SIZE]) -> Option<usize> {
       self.decoder
           .decode_float(Some(opus_data), &mut out[..], false)
           .ok()
   }

   fn decode_plc(&mut self, out: &mut [f32; FRAME_SIZE]) -> Option<usize> {
       self.decoder
           .decode_float(None::<&[u8]>, &mut out[..], false)
           .ok()
   }
   ```
3. **Import `GenericCtl` hanya satu kali**:
   - Hanya import `use audiopus::{Application, Bitrate, Channels, SampleRate, coder::{Decoder as OpusDecoder, Encoder as OpusEncoder, GenericCtl}};` tanpa menduplikasi `use audiopus::coder::GenericCtl;` (`error[E0252]`).
4. **Feature Gate `Cargo.toml`**:
   - Di `crates/horizon-client/Cargo.toml`, fitur `audio` wajib menyertakan `"dep:cpal"`:
     ```toml
     audio = ["horizon-client-render/audio", "dep:cpal"]
     ```

---

## 6. Syarat Wajib Build Android (`tools/android_build.py`) & CI Workflows

Karena `horizon-client` mengompilasi library C/C++ native untuk audio (`audiopus_sys` -> `libopus` C dan `cpal` -> `oboe-sys` C++), sistem build Android dan GitHub Actions memiliki **syarat wajib** berikut agar *cross-compilation* ke arsitektur Android (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`) berhasil:

### 6.1. Syarat Wajib di `tools/android_build.py`
1. **Static Linking `libopus` & Isolasi `pkg-config` Host**:
   ```python
   environment['LIBOPUS_STATIC'] = '1'
   environment['OPUS_STATIC'] = '1'
   environment['LIBOPUS_NO_PKG'] = '1'
   environment['OPUS_NO_PKG'] = '1'
   ```
   - Memastikan `libopus.a` di-*link* statis ke dalam `libhorizon_client.so` (mencegah *crash* `libopus.so not found` saat APK dijalankan) dan mencegah `audiopus_sys` memakai `libopus` host Linux (`x86_64`) via `pkg-config`.
2. **Definisi Lengkap Tooling LLVM NDK (`CC`, `CXX`, `AR`, `RANLIB`, `LINKER`)**:
   - Wajib menyetel `CC_<triple>`, `CXX_<triple>`, `AR_<triple>` (`llvm-ar`), `RANLIB_<triple>` (`llvm-ranlib`), dan `CARGO_TARGET_<TRIPLE>_LINKER` ke binary NDK yang sesuai.
3. **File `CONFIG_SITE` untuk Script Autotools `./configure` `libopus`**:
   - `tools/android_build.py` wajib membuat file `opus-config-<target>.site` di dalam `target_dir` dan menyetel `environment['CONFIG_SITE']`:
     ```ini
     host_alias=<compiler_target>
     cross_compiling=yes
     CC="<ndk>/bin/<compiler_target><min_sdk>-clang"
     AR="<ndk>/bin/llvm-ar"
     RANLIB="<ndk>/bin/llvm-ranlib"
     STRIP="<ndk>/bin/llvm-strip"
     ```

### 6.2. Syarat Wajib di `.github/workflows/build-apk.yml` & `.github/workflows/rust-ci.yml`
1. **Kompatibilitas CMake 4.x (`CMAKE_POLICY_VERSION_MINIMUM="3.5"`)**:
   - Wajib menyetel `CMAKE_POLICY_VERSION_MINIMUM: "3.5"` pada blok `env:` di `.github/workflows/build-apk.yml` dan `.github/workflows/rust-ci.yml` agar CMake 4.x pada runner `ubuntu-latest` tidak menolak `CMakeLists.txt` milik sub-dependensi audio.
2. **Paket Sistem Linux di `rust-ci.yml` (`lint-and-test`)**:
   - Langkah *Install Linux build dependencies* wajib menginstal `libopus-dev` dan `cmake`:
     ```sh
     sudo apt-get update && sudo apt-get install --yes libasound2-dev libudev-dev libxkbcommon-dev libopus-dev cmake pkg-config
     ```
3. **Konfigurasi NDK pada Job `android-check` di `rust-ci.yml`**:
   - Wajib mengekspor `CC_aarch64_linux_android`, `CXX_aarch64_linux_android`, `AR_aarch64_linux_android`, `RANLIB_aarch64_linux_android`, `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`, `CMAKE_TOOLCHAIN_FILE_aarch64_linux_android`, `ANDROID_ABI=arm64-v8a`, `ANDROID_PLATFORM=android-26`, `CMAKE_POLICY_VERSION_MINIMUM=3.5`, dan `PKG_CONFIG_ALLOW_CROSS=0`.
