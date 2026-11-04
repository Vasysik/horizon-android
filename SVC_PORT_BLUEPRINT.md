# SVC (Simple Voice Chat) Port Blueprint: Horizon Ecosystem (Desktop & Android)

Dokumen ini adalah cetak biru arsitektur aktual dan referensi teknis implementasi **Simple Voice Chat (SVC)** bawaan (*built-in*) di dalam ekosistem **Horizon (`horizon-client` / Rust / wgpu)** untuk **Desktop (Linux, macOS, Windows)** dan **Android**.

---

## 1. Peta Arsitektur Aktual di Repositori `horizon-android`

Seluruh subsistem SVC diimplementasikan secara *native* di dalam `crates/horizon-client/src/voice/` dan diintegrasikan langsung ke *game loop* serta *renderer HUD*:

| File Modul | Tanggung Jawab Utama |
|---|---|
| `crates/horizon-client/src/voice/mod.rs` | State utama `VoiceChatClient`, negosiasi *handshake* TCP, fallback versi otomatis, dan orkestrator `tick_audio()`. |
| `crates/horizon-client/src/voice/protocol.rs` | Serialisasi `ByteBufReader`/`ByteBufWriter` (dengan proteksi batas memori/OOM), kriptografi `SvcCipher` (AES-128-GCM), 14 paket TCP `voicechat:*`, 10 paket UDP, dan parser `parse_voice_host`. |
| `crates/horizon-client/src/voice/udp_worker.rs` | Thread background non-blocking (`svc-udp-worker`), *4-step UDP handshake* (`0x05` -> `0x06` -> `0x09` -> `0x0A`), *heartbeat* `KeepAlive (0x08)` & `Ping (0x07)`, serta antrean *bounded* `sync_channel`. |
| `crates/horizon-client/src/voice/audio_pipeline.rs` | Pemrosesan frame PCM 48kHz Mono 20ms (960 sampel), *dual sub-frame* RNNoise (2x 480 sampel via `nnnoiseless`), deteksi aktivitas suara (VAD), codec framing, dan matematika 3D spatial stereo panning. |
| `crates/horizon-client/src/voice/hardware_io.rs` | Abstraksi perangkat keras audio lintas-platform berbasis `cpal`: perekaman mikrofon (*input stream* dengan *linear resampler* otomatis ke 48kHz) dan pemutaran speaker (*output stream* stereo dengan pembatas latensi). |
| `crates/horizon-client/src/voice/config.rs` | Konfigurasi persisten `svc_protocol.json` (versi kompatibilitas, daftar *fallback* `[20, 21, 19, 18, 22]`, *mute*, *disable*, *VAD threshold*, *mic gain*, dan *RNNoise toggle*). |
| `crates/horizon-client/src/voice/ui.rs` & `src/game/voice_ui.rs` | State layar pengaturan Voice Chat (`Screen::VoiceChat`), indikator status HUD kiri-bawah (`VoiceHudStatus`), dan pemeriksaan izin mikrofon runtime Android (`ensure_microphone_permission`). |
| `crates/horizon-client/src/game/remote.rs` | Pemicu `initiate_voice_handshake()` saat join server, delegasi `CustomPayload("voicechat:*")`, dan pemanggilan `tick_voice_chat()` setiap tick dengan resolusi posisi 3D entitas pemain (`self.persist.uuids` -> `r.entities`). |

---

## 2. Alur Kerja End-to-End (Sesuai Source Code Java `de.maxhenkel.voicechat`)

### 2.1. Negosiasi TCP Control Channel & Auto-Fallback
1. Saat klien bergabung ke server (`remote.rs`), `VoiceChatClient::initiate_handshake()` mengirimkan paket `CustomPayload` pada channel `voicechat:request_secret` berisi `compatibility_version` (default: `20`, kompatibel luas dengan server Paper/Purpur/Fabric).
2. Jika dalam **4 detik** server belum membalas dengan `voicechat:secret` (misalnya karena server menggunakan versi SVC yang berbeda), `check_handshake_timeout()` otomatis memutar versi ke urutan berikutnya di `fallback_versions` (`[20, 21, 19, 18, 22]`), menyimpan ke `svc_protocol.json`, dan mengirim ulang `voicechat:request_secret`.
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

## 3. Pipeline Audio Hardware (`cpal`), RNNoise DSP, dan 3D Spatial Mixing

Setiap tick di `remote.rs`, fungsi `tick_voice_chat()` memanggil `self.voice.tick_audio(listener_pos, listener_yaw, resolve_player_pos)`:

### 3.1. Jalur Input Mikrofon (TX)
1. **Izin Runtime Android**: Saat pengguna membuka *unmute* atau menekan tombol PTT di Android, `crate::platform::ensure_microphone_permission()` memeriksa & meminta izin `android.permission.RECORD_AUDIO` melalui JNI.
2. **Akuisisi Hardware (`VoiceHardwareIo::poll_mic_frame`)**:
   - Membuka *input stream* `cpal` secara *lazy* (hanya saat tidak di-mute dan PTT/Voice Activation aktif).
   - Mengubah input multi-channel menjadi mono `f32` dan melakukan *linear resampling* dari sample rate hardware perangkat (misal 44.1kHz atau 48kHz) menjadi tepat **48.000 Hz**.
   - Mengumpulkan sampel di dalam `VecDeque<f32>` (dibatasi maksimal ~200ms agar bebas *lag*) dan mengeluarkan potongan tepat **960 sampel (20ms)**.
3. **RNNoise Dual Sub-Frame & VAD (`VoiceCapturePipeline::process_frame`)**:
   - Karena model neural RNNoise (`nnnoiseless`) beroperasi pada jendela 10ms (480 sampel) dalam skala `[-32768.0, 32767.0]`, setiap frame 960 sampel dipecah menjadi 2 sub-frame 480 sampel, dikalikan `mic_gain`, diproses melalui `DenoiseState::process_frame`, lalu dinormalisasi kembali ke `[-1.0, 1.0]`.
   - Probabilitas bicara (`vad_max = vad1.max(vad2)`) dibandingkan dengan `vad_threshold` jika mode *Voice Activation* aktif.
   - Frame yang lolos dikemas dengan nomor urut `sequence_number` dan flag `whispering` ke dalam `MicPacket`.

### 3.2. Jalur Output Speaker & 3D Spatial Audio (RX)
1. **Resolusi Posisi Entitas Pemain**:
   - Di `remote.rs`, posisi kamera pendengar diambil dari `self.view.eye` (`[f64; 3]`) dan rotasi horizontal dari `self.yaw`.
   - Untuk `PlayerSoundPacket (0x02)`, `sender_uuid` dicocokkan terhadap tabel `self.persist.uuids` (`entity_id -> uuid`) lalu diambil koordinat 3D-nya dari `self.remote.entities` (`Tracked::position`).
2. **Kalkulasi Atensi Jarak & Stereo Panning (`calculate_spatial_gain`)**:
   - Jarak Euclidean 3D dihitung antara kamera dan sumber suara.
   - Redaman linier diterapkan dari `1.0` (jarak 0 blok) hingga `0.0` pada `max_distance` (default `voice_chat_distance` dari `SecretPacket`, atau `pkt.distance`).
   - Vektor arah horizontal diproyeksikan terhadap sudut `listener_yaw` untuk menghasilkan gain kanal kiri (`left_gain`) dan kanan (`right_gain`).
   - Untuk `GroupSoundPacket (0x03)` (komunikasi grup non-posisional), gain diatur penuh `(1.0, 1.0)`.
3. **Pemutaran Hardware (`VoiceHardwareIo::push_spatial_audio`)**:
   - Sampel mono dikalikan dengan `(left_gain, right_gain)` dan didorong ke antrean stereo `speaker_buffer` (dibatasi maksimal ~300ms untuk menjaga latensi real-time) yang di-*drain* oleh *output stream* `cpal`.
