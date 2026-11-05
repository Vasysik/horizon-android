# SVC (Simple Voice Chat) Port Blueprint: Horizon Ecosystem (Desktop & Android)

Dokumen ini adalah cetak biru arsitektur aktual, spesifikasi perubahan terbaru, serta **syarat wajib kompilasi & cross-compilation (`build-android` & CI)** untuk implementasi **Simple Voice Chat (SVC)** bawaan (*built-in*) di dalam ekosistem **Horizon (`horizon-client` / Rust / wgpu)** untuk **Desktop (Linux, macOS, Windows)** dan **Android**.

---

## 1. Peta Arsitektur Aktual di Repositori `horizon-android`

Seluruh subsistem SVC diimplementasikan secara *native* di dalam `crates/horizon-client/src/voice/` dan diintegrasikan langsung ke *game loop*, *renderer HUD*, serta pipeline build Android:

| File Modul | Tanggung Jawab Utama |
|---|---|
| `crates/horizon-client/src/voice/mod.rs` | State utama `VoiceChatClient`, negosiasi *handshake* TCP, fallback versi otomatis, orkestrator `tick_audio()`, reset encoder saat berhenti bicara, dan pembersihan decoder idle (`prune_idle_decoders`). |
| `crates/horizon-client/src/voice/protocol.rs` | Serialisasi `ByteBufReader`/`ByteBufWriter` (dengan proteksi batas memori/OOM), kriptografi `SvcCipher` (AES-128-GCM), 14 paket TCP `voicechat:*`, 10 paket UDP, dan parser `parse_voice_host`. |
| `crates/horizon-client/src/voice/udp_worker.rs` | Thread background non-blocking (`svc-udp-worker`), *4-step UDP handshake* (`0x05` -> `0x06` -> `0x09` -> `0x0A`), *heartbeat* `KeepAlive (0x08)` & `Ping (0x07)`, serta antrean *bounded* `sync_channel`. |
| `crates/horizon-client/src/voice/audio_pipeline.rs` | Pemrosesan frame PCM 48kHz Mono 20ms (960 sampel), *dual sub-frame* RNNoise (2x 480 sampel via `nnnoiseless`), VAD, encoding/decoding **Opus VOIP stateful** (`audiopus 0.2`), **Packet Loss Concealment (PLC)**, manajemen decoder per-pengirim (`SenderDecoder`), dan kalkulasi 3D spatial stereo panning. |
| `crates/horizon-client/src/voice/hardware_io.rs` | Abstraksi perangkat keras audio lintas-platform berbasis `cpal` (`dep:cpal` pada fitur `audio`): *input stream* mikrofon dengan *linear interpolation resampling* ke 48kHz, serta *output stream* speaker stereo dengan `SpeakerResampler` (**60ms jitter prebuffer**, pemulihan *underrun* otomatis, dan *fractional phase linear interpolation*). |
| `crates/horizon-client/src/voice/config.rs` | Konfigurasi persisten `svc_protocol.json` (versi kompatibilitas, daftar *fallback* `[20, 21, 19, 18, 22]`, *mute*, *disable*, *VAD threshold*, *mic gain*, dan *RNNoise toggle*). |
| `crates/horizon-client/src/voice/ui.rs` & `src/game/voice_ui.rs` | State layar pengaturan Voice Chat (`Screen::VoiceChat`), indikator status HUD kiri-bawah (`VoiceHudStatus`), dan pemeriksaan izin mikrofon runtime Android (`ensure_microphone_permission`). |
| `crates/horizon-client/src/game/remote.rs` | Pemicu `initiate_voice_handshake()` saat join server, delegasi `CustomPayload("voicechat:*")`, dan pemanggilan `tick_voice_chat()` setiap tick dengan konversi `sender_uuid: u128` ke `[i32; 4]` untuk resolusi posisi 3D entitas pemain (`self.persist.uuids` -> `r.entities`). |
| `tools/android_build.py` & `.github/workflows/` | Konfigurasi *cross-compilation* statis `libopus` (`audiopus_sys`) dan `oboe-sys` (`cpal`) terhadap Android NDK (`llvm-ar`, `llvm-ranlib`, `CONFIG_SITE`, `CMAKE_POLICY_VERSION_MINIMUM=3.5`). |

---

## 2. Alur Kerja End-to-End (Sesuai Source Code Java `de.maxhenkel.voicechat`)

### 2.1. Negosiasi TCP Control Channel & Auto-Fallback
1. Saat klien bergabung ke server (`remote.rs`), `VoiceChatClient::initiate_handshake()` mengirimkan paket `CustomPayload` pada channel `voicechat:request_secret` berisi `compatibility_version` (default: `20`, kompatibel luas dengan server Paper/Purpur/Fabric).
2. Jika dalam **4 detik** server belum membalas dengan `voicechat:secret`, `check_handshake_timeout()` otomatis memutar versi ke urutan berikutnya di `fallback_versions` (`[20, 21, 19, 18, 22]`), menyimpan ke `svc_protocol.json`, dan mengirim ulang `voicechat:request_secret`.
3. Ketika `voicechat:secret` diterima:
   - Klien mengekstrak `secret` (16B AES key), `server_port`, `player_uuid`, `codec`, `mtu_size`, `voice_chat_distance`, `keep_alive`, `groups_enabled`, `voice_host`, dan `allow_recording`.
   - Klien mem-parsing target UDP menggunakan `parse_voice_host(&secret.voice_host, &current_server_host, secret.server_port as u16)` yang mendukung override hostname, port saja (`"8000"`), domain+port (`"edge.example.com:8000"`), maupun IPv4/IPv6.
   - Klien langsung membalas dengan `voicechat:client_state` (`UpdateStatePacket`) berisi status `muted` dan `disabled`.

### 2.2. 4-Step UDP Handshake & Enkripsi AES-128-GCM
Thread `svc-udp-worker` membuka socket UDP non-blocking (IPv4 `0.0.0.0:0` atau IPv6 `[::]:0` sesuai hasil DNS) dan menjalankan *4-step handshake* selama maksimal 6 detik (interval *retry* 500ms):
1. **Step 1 (`0x05 AuthenticatePacket`)**: Klien mengirim `[player_uuid (16B)] + [secret (16B)]` terenkripsi AES-128-GCM dengan header `[Magic 0xFF] + [ConnectionID UUID (16B)] + [VarInt Len] + [12B IV + Ciphertext + 16B GCM Tag]`.
2. **Step 2 (`0x06 AuthenticateAckPacket`)**: Server membalas dengan paket terenkripsi `[Magic 0xFF] + [VarInt Len] + [12B IV + Ciphertext + 16B GCM Tag]`. Pada SVC v20 body kosong; pada SVC v21 body berisi 16-byte `nonce`.
3. **Step 3 (`0x09 ConnectionCheckPacket`)**: Klien mengirim paket verifikasi koneksi dua arah (menyertakan 16-byte `nonce` jika diterima pada Step 2, atau kosong untuk v20).
4. **Step 4 (`0x0A ConnectionCheckAckPacket`)**: Server mengonfirmasi jalur UDP aktif. Status berubah menjadi `UdpConnectionState::Connected { ping_ms }`.

### 2.3. Loop Aktif: KeepAlive, Ping RTT, dan Transmisi Audio
Setelah terhubung, `svc-udp-worker` menjalankan siklus berikut:
- **Outgoing Mic Queue**: Mengambil `MicPacket` dari `mic_rx` (`sync_channel` kapasitas 128), mengenkripsi dengan ID `0x01`, dan mengirimkannya ke socket UDP.
- **KeepAlive (`0x08`)**: Mengirim *heartbeat* setiap `keep_alive` ms (di-clamp 500..5000 ms) **dan** langsung membalas setiap paket `0x08` yang datang dari server agar sesi tidak diputus oleh `Server.java`.
- **Ping (`0x07`)**: Mengirim UUID ping unik + timestamp setiap 1000 ms dan menghitung *Round-Trip Time (RTT)* saat server membalas dengan ID ping yang cocok.
- **Incoming Sound Queue**: Mendekripsi paket suara masuk (`0x02 PlayerSound`, `0x03 GroupSound`, `0x04 LocationSound`) dan mendorongnya secara non-blocking (`try_send`) ke `sound_tx` (`sync_channel` kapasitas 256).

---

## 3. Perubahan Inti Pipeline Audio: Opus Stateful, PLC, dan Jitter Prebuffer

### 3.1. Jalur Input Mikrofon (TX) & Stateful Opus VOIP Encoder
1. **Izin Runtime Android**: Saat pengguna membuka *unmute* atau menekan tombol PTT di Android, `crate::platform::ensure_microphone_permission()` memeriksa & meminta izin `android.permission.RECORD_AUDIO` melalui JNI.
2. **Akuisisi Hardware & Linear Interpolation Resampling (`VoiceHardwareIo::poll_mic_frame`)**:
   - Membuka *input stream* `cpal` secara *lazy* (hanya saat tidak di-mute dan PTT/Voice Activation aktif).
   - Mengubah input multi-channel menjadi mono `f32` dan melakukan **linear interpolation resampling** (`s0 + (s1 - s0) * frac`) dari sample rate hardware perangkat (misal 44.1kHz) menjadi tepat **48.000 Hz**.
   - Mengumpulkan sampel di dalam `VecDeque<f32>` (dibatasi maksimal ~200ms agar bebas *lag*) dan mengeluarkan potongan tepat **960 sampel (20ms)**.
3. **RNNoise Dual Sub-Frame, VAD, & Opus Encoding (`VoiceCapturePipeline`)**:
   - Setiap frame 960 sampel dipecah menjadi 2 sub-frame 480 sampel (10ms), dikalikan `mic_gain`, diproses melalui `DenoiseState::process_frame` dalam skala `[-32768.0, 32767.0]`, lalu dinormalisasi kembali ke `[-1.0, 1.0]`.
   - Probabilitas bicara (`vad_max = vad1.max(vad2)`) dibandingkan dengan `vad_threshold` jika mode *Voice Activation* aktif.
   - Frame yang lolos dienkode menggunakan **Opus Encoder stateful** (`OpusEncoder::new(SampleRate::Hz48000, Channels::Mono, Application::Voip)`) via `encoder.encode_float(pcm, &mut out)` dengan ukuran paket maksimum `MAX_OPUS_PACKET_SIZE = 1275` byte.
   - Saat pemain berhenti berbicara (`is_talking` berubah dari `true` ke `false`) atau terputus dari server (`stop()`), `capture_pipeline.reset_encoder()` memanggil `GenericCtl::reset_state()` pada encoder Opus agar prediksi LPC/SILK bersih untuk sesi bicara berikutnya.

### 3.2. Jalur Output Speaker (RX): Per-Sender Opus Decoder, PLC, & 3D Spatial Audio
1. **Per-Sender Stateful Decoder & Packet Loss Concealment (`VoicePlaybackPipeline`)**:
   - Menyimpan state decoder terpisah untuk setiap pengirim di dalam `HashMap<u128, SenderDecoder>` agar state prediksi Opus antar-pemain tidak saling merusak.
   - **Stop-Talking Signal (`opus_data.is_empty()`)**: Jika paket berisi payload kosong, decoder pengirim tersebut di-reset (`reset_state()`) dan fungsi mengembalikan `Vec::new()` tanpa mendorong sampel ke speaker.
   - **Deteksi Gap & Opus PLC (`MAX_PLC_FRAMES = 5`)**:
     - Jika `sequence_number` melompat (`gap = sequence_number - last_sequence > 1`), pipeline menjalankan **Opus Packet Loss Concealment (PLC)** dengan memanggil `decoder.decode_float(None::<&[u8]>, &mut plc_buf[..], false)` sebanyak maksimal 5 frame (100ms) sebelum mendekode paket saat ini.
     - Jika paket datang terlambat/duplikat (`gap <= 0` dan `gap >= -100`), paket diabaikan agar state urutan decoder tetap utuh. Jika urutan melompat jauh ke belakang (`gap < -100`, misalnya pengirim reconnect), decoder otomatis di-reset.
   - **Pembersihan Otomatis (`prune_idle_decoders`)**: Setiap tick, decoder pengirim yang tidak aktif lebih dari `DECODER_IDLE_TIMEOUT` (10 detik) dihapus dari memori.
2. **Resolusi Posisi Entitas Pemain (`remote.rs`)**:
   - `sender_uuid: u128` dikonversi menjadi representasi `[i32; 4]` big-endian:
     ```rust
     let target_uuid = [
         (sender_uuid >> 96) as i32,
         (sender_uuid >> 64) as i32,
         (sender_uuid >> 32) as i32,
         sender_uuid as i32,
     ];
     ```
   - Dicocokkan terhadap `self.persist.uuids` (`entity_id -> [i32; 4]`) untuk mengambil posisi 3D entitas dari `self.remote.entities`.
3. **Speaker Jitter Prebuffering & Resampling (`SpeakerResampler`)**:
   - Menerapkan **60ms Prebuffer** (`PREBUFFER_SAMPLES = FRAME_SIZE * 3 = 2880` sampel stereo pada 48kHz): stream output menahan pemutaran (mengisi `0.0` / `0`) sampai antrean memiliki minimal 3 frame (60ms) untuk menyerap *network jitter*.
   - Jika terjadi *buffer underrun* (antrean habis), `SpeakerResampler` langsung kembali ke mode `prebuffering = true` (`enter_prebuffering()`) untuk mencegah suara *crackling/popping*.
   - Melakukan **fractional phase linear interpolation** (`prev + (curr - prev) * frac`) dari 48.000 Hz ke sample rate output hardware (`out_rate`) baik untuk format sampel `cpal::SampleFormat::F32` maupun `cpal::SampleFormat::I16`.

---

## 4. Syarat Wajib Kode Rust & Kompilasi `audiopus 0.2.0`

Untuk mencegah error kompilasi Rust (`E0252` duplicate import dan `E0277` trait bound `TryFrom` tidak terpenuhi pada edisi Rust 2021+), aturan berikut **wajib dipatuhi** di `crates/horizon-client/src/voice/audio_pipeline.rs`:

1. **Jangan gunakan `OpusPacket::try_from` atau `MutSignals::try_from`**:
   - Pada crate `audiopus = "0.2"` (`0.2.0`), tipe `&[u8]` dan `&mut [f32]` mengimplementasikan trait kustom `audiopus::TryInto<Packet<'a>>` dan `audiopus::TryInto<MutSignals<'a, f32>>`, **bukan** `std::convert::TryFrom` ataupun `audiopus::TryFrom`.
   - Memanggil `OpusPacket::try_from(opus_data)` akan bentrok dengan *blanket implementation* `std::convert::TryFrom` dari prelude Rust 2021 (`error[E0277]`).
2. **Operasikan slice secara langsung ke `Decoder::decode_float`**:
   - Karena signature `decode_float` adalah `fn decode_float<'a, TP: TryInto<Packet<'a>>, TS: TryInto<MutSignals<'a, f32>>>(&mut self, input: Option<TP>, output: TS, fec: bool)`, selalu panggil langsung dengan slice:
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
   - Hanya import `use audiopus::{Application, Channels, SampleRate, coder::{Decoder as OpusDecoder, Encoder as OpusEncoder, GenericCtl}};` tanpa menduplikasi `use audiopus::coder::GenericCtl;` (`error[E0252]`).
4. **Feature Gate `Cargo.toml`**:
   - Di `crates/horizon-client/Cargo.toml`, fitur `audio` wajib menyertakan `"dep:cpal"`:
     ```toml
     audio = ["horizon-client-render/audio", "dep:cpal"]
     ```

---

## 5. Syarat Wajib Build Android (`tools/android_build.py`) & CI Workflows

Karena `horizon-client` kini mengompilasi library C/C++ native untuk audio (`audiopus_sys` -> `libopus` C dan `cpal` -> `oboe-sys` C++), sistem build Android dan GitHub Actions memiliki **syarat wajib** berikut agar proses *cross-compilation* ke arsitektur Android (`arm64-v8a`, `armeabi-v7a`, `x86_64`, `x86`) berhasil:

### 5.1. Syarat Wajib di `tools/android_build.py`
1. **Static Linking `libopus` & Isolasi `pkg-config` Host**:
   - Wajib mengekspor environment variable berikut sebelum menjalankan `cargo build`:
     ```python
     environment['LIBOPUS_STATIC'] = '1'
     environment['OPUS_STATIC'] = '1'
     environment['LIBOPUS_NO_PKG'] = '1'
     environment['OPUS_NO_PKG'] = '1'
     ```
   - **Alasan**: `LIBOPUS_STATIC=1` dan `OPUS_STATIC=1` memastikan `libopus.a` di-*link* secara statis ke dalam `libhorizon_client.so` (jika dinamis, APK akan *crash* saat startup karena Android tidak memiliki `libopus.so` sistem). `LIBOPUS_NO_PKG=1` dan `OPUS_NO_PKG=1` mencegah `audiopus_sys` mendeteksi `libopus` milik host Linux (`x86_64`) melalui `pkg-config`.
2. **Definisi Lengkap Tooling LLVM NDK (`CC`, `CXX`, `AR`, `RANLIB`, `LINKER`)**:
   - Wajib menyetel variabel per-target triple (`aarch64_linux_android`, dll.):
     - `CC_<triple>` -> `<ndk>/bin/<compiler_target><min_sdk>-clang`
     - `CXX_<triple>` -> `<ndk>/bin/<compiler_target><min_sdk>-clang++`
     - `AR_<triple>` -> `<ndk>/bin/llvm-ar`
     - `RANLIB_<triple>` -> `<ndk>/bin/llvm-ranlib`
     - `CARGO_TARGET_<TRIPLE>_LINKER` -> `<ndk>/bin/<compiler_target><min_sdk>-clang`
3. **File `CONFIG_SITE` untuk Script Autotools `./configure` `libopus`**:
   - Pada host Linux/CI, `audiopus_sys` menjalankan script autotools `./configure` untuk membangun `libopus`. Tanpa `CONFIG_SITE`, `./configure` gagal mengenali mode *cross-compile* atau memakai `ar`/`ranlib` host.
   - `tools/android_build.py` wajib membuat file `opus-config-<target>.site` di dalam `target_dir` dan mengarahkannya ke `environment['CONFIG_SITE']`:
     ```ini
     host_alias=<compiler_target>
     cross_compiling=yes
     CC="<ndk>/bin/<compiler_target><min_sdk>-clang"
     AR="<ndk>/bin/llvm-ar"
     RANLIB="<ndk>/bin/llvm-ranlib"
     STRIP="<ndk>/bin/llvm-strip"
     ```

### 5.2. Syarat Wajib di `.github/workflows/build-apk.yml` & `.github/workflows/rust-ci.yml`
1. **Kompatibilitas CMake 4.x (`CMAKE_POLICY_VERSION_MINIMUM="3.5"`)**:
   - Runner GitHub Actions `ubuntu-latest` menggunakan CMake 4.x yang menolak `CMakeLists.txt` lama milik sub-dependensi audio (`audiopus_sys` / `oboe-sys`).
   - Wajib menyetel `CMAKE_POLICY_VERSION_MINIMUM: "3.5"` pada blok `env:` di `.github/workflows/build-apk.yml` dan `.github/workflows/rust-ci.yml`.
2. **Paket Sistem Linux di `rust-ci.yml` (`lint-and-test`)**:
   - Langkah *Install Linux build dependencies* wajib menginstal `libopus-dev` dan `cmake` selain paket audio/windowing standar:
     ```sh
     sudo apt-get update && sudo apt-get install --yes libasound2-dev libudev-dev libxkbcommon-dev libopus-dev cmake pkg-config
     ```
3. **Konfigurasi NDK pada Job `android-check` di `rust-ci.yml`**:
   - Saat menjalankan `cargo check -p horizon-android --target aarch64-linux-android`, wajib menyetel `CC_aarch64_linux_android`, `CXX_aarch64_linux_android`, `AR_aarch64_linux_android`, `RANLIB_aarch64_linux_android`, `CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER`, `CMAKE_TOOLCHAIN_FILE_aarch64_linux_android`, `ANDROID_ABI=arm64-v8a`, `ANDROID_PLATFORM=android-26`, `CMAKE_POLICY_VERSION_MINIMUM=3.5`, dan `PKG_CONFIG_ALLOW_CROSS=0`.
