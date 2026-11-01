> 🤖 Opened by kaito65452's AI agent (Claude, claude-fable-5-1), not by kaito65452 personally.

# Experimental Built-in Simple Voice Chat (SVC) for Horizon (Desktop & Android)

**Reviewer:** @Vasysik  
**Branch:** `feat/experimental-svc-builtin`  
**Target:** `android` / `main`

---

## 1. Ringkasan Eksekutif & Tujuan PR (Executive Summary & Goals)

Pull Request ini mengimplementasikan fitur **Simple Voice Chat (SVC)** bawaan (*built-in experimental*) secara langsung ke dalam Horizon Client, memberikan kompatibilitas penuh dengan server Minecraft Java yang menjalankan plugin/mod Simple Voice Chat (Spigot, Paper, Purpur, Fabric, Quilt, NeoForge) tanpa memerlukan runtime Java tambahan.

Fitur ini dirancang khusus untuk berjalan mulus baik pada target **Desktop** (Linux, Windows, macOS) maupun **Android** (Handheld/Touch), dengan memperhatikan batasan hardware mobile, efisiensi memori, dan kepatuhan terhadap kebijakan izin sistem Android.

---

## 2. Arsitektur & Spesifikasi Protokol (Architecture & Protocol Implementation)

### A. Dynamic Compatibility Negotiation & Fallback
SVC menggunakan sistem nomor versi kompatibilitas protokol pada packet `voicechat:request_secret`.
- **Default Compatibility Version:** `21` (Simple Voice Chat versi 1.20.x – 1.21.x).
- **Auto-Negotiation & Fallback:** File konfigurasi `svc_protocol.json` menyimpan urutan fallback `[21, 20, 19, 18, 22]`. Jika server tidak merespons `SecretPacket` dalam batas waktu 4 detik, client secara otomatis mencoba versi fallback berikutnya secara dinamis.
- **Runtime Reconfiguration:** Versi protokol dan pengaturan voice chat dapat disesuaikan langsung melalui GUI in-game (Keybind `V` / Touch Voice Menu) tanpa perlu recompilation.

### B. Saluran TCP Minecraft CustomPayload (14 Packet Channels)
Seluruh packet koordinasi ditransmisikan melalui namespace standard `voicechat:*`:
1. `voicechat:request_secret` (`0x01`): Dikirim saat login untuk meminta kunci sesi.
2. `voicechat:secret` (`0x02`): Menerima secret key AES-128, port UDP voice server, keepalive timeout, dan MTU.
3. `voicechat:player_state`: Update status individual pemain (mute, deafen, group).
4. `voicechat:player_states`: Sinkronisasi massal seluruh pemain saat awal bergabung.
5. `voicechat:remove_player_state`: Pembersihan state saat pemain keluar.
6. `voicechat:update_state`: Mengirim state client (muted/disabled) ke server.
7. `voicechat:add_group`: Menambahkan grup voice chat privat.
8. `voicechat:remove_group`: Menghapus grup voice chat.
9. `voicechat:join_group`: Request bergabung ke grup dengan password.
10. `voicechat:leave_group`: Request keluar dari grup.
11. `voicechat:create_group`: Membuat grup baru (Normal / Open / Isolated).
12. `voicechat:joined_group`: Konfirmasi join grup dari server.
13. `voicechat:add_category`: Kategori audio volume server.
14. `voicechat:remove_category`: Pembersihan kategori audio server.

### C. UDP Transport & Cryptographic Security
Transport audio berjalan di atas socket UDP terpisah non-blocking melalui background worker thread (`UdpWorker`):
- **Magic Prefix:** Header byte `0xFF` diikuti oleh 16-byte UUID koneksi client.
- **Kriptografi:** Enkripsi simetris **AES-128-GCM** (`aes-gcm` crate) dengan 12-byte initialization vector (IV) unik per packet dan 128-bit authentication tag.
- **4-Step UDP Handshake Parity:**
  1. Client mengirim `0x05 AuthenticatePacket`.
  2. Server membalas `0x06 AuthenticateAckPacket` membawa 16-byte cryptographic challenge nonce.
  3. Client membalas `0x09 ConnectionCheckPacket` memantulkan challenge nonce tersebut.
  4. Server memvalidasi dan mengirim `0x0A ConnectionCheckAckPacket`.
- **Heartbeat & Latency Telemetry:** Background thread mengirim `0x08 KeepAlive` secara periodik dan mengukur latensi round-trip (RTT ping) menggunakan `0x07 PingPacket`.

### D. Audio Pipeline, RNNoise AI Noise Filter, & 3D Spatial Audio
- **Frame Specifications:** 960 sample PCM 48kHz mono per frame (20ms interval).
- **Dual Sub-Frame Neural Suppression:** Dijalankan melalui `nnnoiseless` (`DenoiseState<'static>`) memproses dua sub-frame 480-sample, diskalakan ke rentang `[-32768.0, 32767.0]`, menghasilkan probabilitas VAD (*Voice Activity Detection*) untuk aktivasi suara dinamis.
- **Opus Wire Format:** Payload terkompresi Opus framing standar SVC untuk kompatibilitas lintas klien Java.
- **3D Spatial Audio (`calculate_spatial_gain`):**
  - Linear distance attenuation berbasis jarak relatif antara listener dan sound source (`max_distance` configurable, default 16-48 blok).
  - Stereo panning azimuth yang dihitung terhadap camera yaw pemain.

---

## 3. Dukungan Platform & Android Permission Guardrails

### Desktop
- **Default Keybindings:**
  - `V`: Membuka In-Game Voice Chat Menu modal.
  - `CapsLock`: Push-to-Talk (PTT).
  - `M`: Toggle Microphone Mute.
  - `N`: Toggle Disable Voice Chat.
  - `H`: Toggle Voice HUD Icons.
- Terintegrasi penuh ke dalam sistem pengaturan `KeyBinds` Horizon di bawah kategori baru `voicechat`.

### Android Touch & Privacy Guardrails
1. **Manifest Configuration:**
   - Menambahkan `<uses-permission android:name="android.permission.RECORD_AUDIO" />` ke `packaging/android/AndroidManifest.xml`.
2. **JNI Runtime Permission Check (`platform.rs`):**
   - Menambahkan helper `ensure_microphone_permission() -> bool` yang memeriksa status `checkSelfPermission("android.permission.RECORD_AUDIO")`.
   - Jika belum diberikan, otomatis memanggil `requestPermissions` dengan request code `9001` secara aman tanpa crash.
3. **Delayed Audio Stream Policy:**
   - **Koneksi UDP SVC dan playback audio pemain lain BERJALAN NORMAL tanpa pernah meminta izin mikrofon.**
   - Izin mikrofon hanya diminta saat pemain menekan tombol **Push-to-Talk** atau melakukan **Unmute** untuk pertama kalinya. Hal ini menjaga kepatuhan privasi pengguna dan mencegah pembukaan resource Oboe/cpal yang tidak perlu.
4. **Custom Touch Actions:**
   - Mendukung `action:key.voice_chat` dan `action:key.push_to_talk` pada kustomisasi layout sentuh Android (`touch/custom.rs`).

---

## 4. UI In-Game & Bottom-Left HUD Indicators

- **Modal Settings (Key 'V'):**
  - Status koneksi real-time (Disconnected, Connecting, Authenticating, Connected dengan RTT ms).
  - Toggle Microphone (Mute / Active).
  - Toggle Voice Chat (Enabled / Disabled).
  - Toggle Mode Aktivasi (Voice Activity vs Push-To-Talk).
  - Toggle RNNoise AI Filter.
  - Tombol Cycle Fallback Compatibility Version.
- **Bottom-Left HUD Status Indicator:**
  - `[SVC: Disconnected]` (Abu-abu)
  - `[SVC: Disabled]` (Merah)
  - `[SVC: Muted]` (Kuning-Oranye)
  - `[SVC: Speaking]` (Hijau)
  - `[SVC: Whispering]` (Cyan)
  - `[SVC: XXms]` (Putih saat ready / idle)

---

## 5. Verifikasi & Pengujian Mandiri (Self-Testing & Verification)

1. **Unit Testing (7 Passed Cleanly):**
   - `protocol::tests::test_varint_roundtrip`: Memverifikasi LEB128 VarInt encoding/decoding.
   - `protocol::tests::test_secret_packet_serialization`: Memverifikasi parsing dan serialisasi SecretPacket.
   - `protocol::tests::test_player_state_packet_serialization`: Memverifikasi CustomPayload player states.
   - `protocol::tests::test_aes_128_gcm_and_udp_framing_roundtrip`: Memverifikasi enkripsi/dekripsi AES-128-GCM 12-byte IV + Auth Tag roundtrip wire packet.
   - `audio_pipeline::tests::test_pcm_codec_roundtrip`: Memverifikasi encoding/decoding PCM sample buffer.
   - `audio_pipeline::tests::test_spatial_gain_calculation`: Memverifikasi jarak dan panning stereo 3D spatial gain.
   - `audio_pipeline::tests::test_voice_capture_dual_subframe_vad`: Memverifikasi pemrosesan dual 480-sample RNNoise suppression dan deteksi VAD.
2. **Compiler & Linter Verification:**
   - `cargo check -p horizon-client --no-default-features`: **CLEAN** (0 error, 0 clippy warning pada modul voice).

---

## 6. Panduan Pengujian di Server Java SVC (How to Test)

1. Jalankan server Paper/Purpur/Fabric Minecraft Java dengan plugin `Simple Voice Chat` (port UDP default: `24454`).
2. Jalankan Horizon client dan connect ke server:
   ```bash
   cargo run --profile fast -p horizon-client -- --server <server_ip>:<port>
   ```
3. Saat berhasil login ke server, amati log:
   ```text
   INFO [horizon_client::voice] SVC: Received secret from server, port: 24454, codec: 0, mtu: 1024
   ```
4. Tekan tombol `V` untuk membuka menu Simple Voice Chat. Pastikan status menunjukkan `Status: Connected (XX ms)`.
5. Tahan tombol `CapsLock` untuk berbicara via Push-to-Talk dan amati indikator bottom-left berubah menjadi hijau `[SVC: Speaking]`.

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)
