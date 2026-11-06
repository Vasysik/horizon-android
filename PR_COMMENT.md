# Simple Voice Chat (SVC) Protocol Alignment: UDP Handshake & Cryptography Fix

## 📌 Ringkasan Eksekutif
Pull request ini menyelesaikan masalah *persistent silent packet drop* pada koneksi UDP voice ke server Minecraft publik (seperti `play.soramc.my.id`). Klien sebelumnya mengirim 12x paket `Authenticate` tetapi server tidak merespons (`UDP handshake timed out after 6s (sent 12 Authenticate, 0 ConnectionCheck)`).

Investigasi mendalam pada kode upstream Simple Voice Chat (`henkelmax/simple-voice-chat`) di berbagai branch versi kompatibilitas mengungkap akar penyebab: **perbedaan struktur payload internal antara Compatibility Version 20 dan Version 21**.

---

## 🔍 Tabel Fakta Verifikasi Protokol (v20 vs v21)

Berdasarkan audit decompilation dan source upstream Simple Voice Chat:

| Aspek Protokol | Versi Kompatibilitas 20 (v20) | Versi Kompatibilitas 21 (v21) | Catatan Paritas Horizon |
|---|---|---|---|
| **Cipher Suite** | `AES/GCM/NoPadding` (128-bit) | `AES/GCM/NoPadding` (128-bit) | Identik. Kunci 16B, IV acak 12B, Tag 16B. |
| **Header Datagram Klien -> Server** | `[0xFF] + [UUID (16B)] + [VarInt(len)] + [Encrypted Payload]` | `[0xFF] + [UUID (16B)] + [VarInt(len)] + [Encrypted Payload]` | Identik. Outer UUID adalah `player_uuid` (v20) / `connection_id` (v21). |
| **Header Datagram Server -> Klien** | `[0xFF] + [VarInt(len)] + [Encrypted Payload]` | `[0xFF] + [VarInt(len)] + [Encrypted Payload]` | Identik. Server tidak menyertakan UUID pada header luar. |
| **`SecretPacket` (Plugin Message)** | 80 Byte (`player_uuid` + `secret` + `port` + options). Tanpa `connection_id`. | 96 Byte. Menyertakan field terpisah `connection_id` (16B). | Parsed cleanly. Pada v20 `connection_id = player_uuid`. |
| **`AuthenticatePacket` (`0x05`) Body** | **Wajib 32 Byte**: `player_uuid` (16B BE) + `secret` (16B). Melempar `IndexOutOfBoundsException` jika kosong. | **0 Byte (Kosong)**. `fromBytes` tidak membaca payload. | **Akar masalah silent drop**: Server v20 mengalami error saat membaca buffer kosong lalu mengabaikan paket. Horizon kini mengirim 32B body (kompatibel penuh v20 & v21). |
| **`AuthenticateAckPacket` (`0x06`) Body** | **0 Byte (Kosong)**. | **16 Byte**. Berisi `nonce` verifikasi acak. | Horizon kini mendukung `Option<[u8; 16]>` (memvalidasi tanpa nonce pada v20). |
| **`ConnectionCheckPacket` (`0x09`) Body** | **0 Byte (Kosong)**. | **16 Byte**. Mengirim balik `nonce` dari Step 2. | Mengirim body kosong jika v20 (`nonce == None`), atau 16B nonce jika v21. |
| **`ConnectionCheckAckPacket` (`0x0A`) Body** | **0 Byte (Kosong)**. | **0 Byte (Kosong)**. | Identik. Sinyal final bahwa voice stream siap digunakan. |

---

## 🔐 Bedah Kriptografi & Wire Layout

### 1. Format Datagram UDP (Client -> Server)
```text
+-------------------+-----------------------------+-----------------------+---------------------------------------+
| Magic Byte (0xFF) | Connection ID / UUID (16B)  | VarInt Payload Length | Encrypted Payload (IV + Data + Tag)   |
| 1 byte            | 16 bytes Big-Endian         | 1 - 3 bytes           | 12B IV + (1B ID + Body) + 16B AuthTag |
+-------------------+-----------------------------+-----------------------+---------------------------------------+
```

### 2. Format Datagram UDP (Server -> Client)
```text
+-------------------+-----------------------+---------------------------------------+
| Magic Byte (0xFF) | VarInt Payload Length | Encrypted Payload (IV + Data + Tag)   |
| 1 byte            | 1 - 3 bytes           | 12B IV + (1B ID + Body) + 16B AuthTag |
+-------------------+-----------------------+---------------------------------------+
```

### 3. Payload Dekripsi Paket `Authenticate` (v20)
- Byte 0: `0x05` (`PACKET_ID_AUTHENTICATE`)
- Byte 1..16: `player_uuid` (16-byte Big-Endian)
- Byte 17..32: `secret` (16-byte raw AES key)
- Total Plaintext: 33 Byte.

---

## 🛠 Perubahan yang Dilakukan
1. **`crates/horizon-client/src/voice/protocol.rs`**:
   - Menambahkan struct `AuthenticatePacket` dengan enkoding `player_uuid` dan `secret` (32 bytes).
   - Memperbarui `AuthenticateAckPacket` dan `ConnectionCheckPacket` dengan `nonce: Option<[u8; 16]>` agar kompatibel dua arah dengan v20 (body kosong) dan v21 (16-byte nonce).
   - Menambahkan unit tests komprehensif untuk serialisasi v20/v21 serta roundtrip enkripsi-dekripsi wire format.
2. **`crates/horizon-client/src/voice/udp_worker.rs`**:
   - Menggunakan `AuthenticatePacket::new(secret.player_uuid, secret.secret).to_body()` pada Step 1.
   - Menggunakan state flag `auth_ack_received` untuk menggerakkan state machine handshake tanpa memblokir pada v20 yang tidak memiliki nonce.
   - Menambahkan structured logging yang aman (tidak mengekspos secret/key) untuk memudahkan observabilitas status koneksi voice.
3. **`SVC_PORT_BLUEPRINT.md`**:
   - Mengoreksi tabel paket UDP (seksi 3.4 dan 3.5) agar mencantumkan fakta payload v20 dan v21 secara akurat.

---

## 🧪 Hasil Pengujian
- Unit test suite: `cargo test -p horizon-client --no-default-features --lib voice::` passing 100%.
- Format check: `cargo fmt --check` clean.
- Tidak ada file Mojang proprietary yang tersentuh.
