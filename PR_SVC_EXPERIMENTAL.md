> 🤖 Opened by kaito65452's AI agent (Claude Code), not by kaito65452 personally.

# Experimental Built-in Simple Voice Chat (SVC) for Horizon (Desktop & Android)

**Reviewer:** @Vasysik  
**Branch:** `feat/experimental-svc-builtin`  
**Target:** `android`

---

## 1. Executive Summary & Goals

This pull request implements a **built-in experimental Simple Voice Chat (SVC)** client subsystem natively in Horizon Client. It provides full compatibility with Minecraft Java servers running the Simple Voice Chat mod/plugin (Paper, Purpur, Spigot, Fabric, Quilt, NeoForge) without requiring an external Java runtime or third-party voice bridges.

The implementation is tailored for both **Desktop** (Linux, Windows, macOS) and **Android** (Handheld/Touch), adhering strictly to mobile hardware constraints, memory efficiency, and Android privacy/permission guidelines.

---

## 2. Architecture & Protocol Implementation

### A. Dynamic Compatibility Negotiation & Fallback
SVC uses a protocol compatibility version number during the `voicechat:request_secret` handshake.
- **Default Compatibility Version:** `21` (Simple Voice Chat for Minecraft 1.20.x – 1.21.x).
- **Auto-Negotiation & Fallback:** The `svc_protocol.json` configuration file maintains a fallback version sequence `[21, 20, 19, 18, 22]`. If the server does not respond with a `SecretPacket` within 4 seconds, the client automatically cycles to the next fallback version dynamically.
- **Runtime Reconfiguration:** Protocol version and voice chat settings can be reconfigured directly in-game (Keybind `V` / Touch Voice Menu) without recompilation.

### B. Minecraft CustomPayload TCP Channels (14 Packet Channels)
All coordination packets are transmitted over the standard `voicechat:*` namespace:
1. `voicechat:request_secret` (`0x01`): Sent upon login to request session authentication details.
2. `voicechat:secret` (`0x02`): Receives AES-128 secret key, voice server UDP port, keepalive timeout, and MTU.
3. `voicechat:player_state`: Updates individual player states (mute, deafen, group status).
4. `voicechat:player_states`: Bulk synchronization of all online player voice states upon joining.
5. `voicechat:remove_player_state`: State cleanup when a player disconnects.
6. `voicechat:update_state`: Transmits local client state (muted/disabled) to the server.
7. `voicechat:add_group`: Adds a private voice group.
8. `voicechat:remove_group`: Removes a private voice group.
9. `voicechat:join_group`: Requests to join a group with an optional password.
10. `voicechat:leave_group`: Requests to leave the current group.
11. `voicechat:create_group`: Creates a new voice group (Normal / Open / Isolated).
12. `voicechat:joined_group`: Server confirmation of joining a group.
13. `voicechat:add_category`: Registers server-side audio volume categories.
14. `voicechat:remove_category`: Cleans up audio categories.

### C. UDP Transport & Cryptographic Security
Audio transmission is handled over a dedicated non-blocking UDP background worker thread (`UdpWorker`):
- **Magic Prefix:** Header byte `0xFF` followed by the 16-byte client connection UUID.
- **Cryptography:** Symmetric **AES-128-GCM** encryption (`aes-gcm` crate) using a unique 12-byte initialization vector (IV) per packet and a 128-bit authentication tag.
- **4-Step UDP Handshake Parity:**
  1. Client sends `0x05 AuthenticatePacket`.
  2. Server responds with `0x06 AuthenticateAckPacket` carrying a 16-byte cryptographic challenge nonce.
  3. Client responds with `0x09 ConnectionCheckPacket` echoing the challenge nonce.
  4. Server validates and responds with `0x0A ConnectionCheckAckPacket`.
- **Heartbeat & Latency Telemetry:** Background thread periodically transmits `0x08 KeepAlive` packets and measures round-trip time (RTT ping) via `0x07 PingPacket`.

### D. Audio Pipeline, RNNoise AI Noise Filter, & 3D Spatial Audio
- **Frame Specifications:** 960 PCM samples @ 48kHz mono per frame (20ms interval).
- **Dual Sub-Frame Neural Noise Suppression:** Powered by `nnnoiseless` (`DenoiseState<'static>`), processing two 480-sample sub-frames scaled to `[-32768.0, 32767.0]` and outputting Voice Activity Detection (VAD) probabilities for dynamic voice activation.
- **Opus Wire Format:** Standard Opus framing layout compliant with Java SVC clients.
- **3D Spatial Audio (`calculate_spatial_gain`):**
  - Linear distance attenuation based on relative distance between listener and sound source (configurable `max_distance`, default 16–48 blocks).
  - Stereo azimuth panning relative to the player's camera yaw.

---

## 3. Platform Support & Android Permission Guardrails

### Desktop
- **Default Keybindings:**
  - `V`: Opens In-Game Voice Chat Menu modal.
  - `CapsLock`: Push-to-Talk (PTT).
  - `M`: Toggle Microphone Mute.
  - `N`: Toggle Disable Voice Chat.
  - `H`: Toggle Voice HUD Icons.
- Fully registered within Horizon's `KeyBinds` system under the new `voicechat` category.

### Android Touch & Privacy Guardrails
1. **Manifest Configuration:**
   - Added `<uses-permission android:name="android.permission.RECORD_AUDIO" />` to `packaging/android/AndroidManifest.xml`.
2. **JNI Runtime Permission Check (`platform.rs`):**
   - Added `ensure_microphone_permission() -> bool` which checks `checkSelfPermission("android.permission.RECORD_AUDIO")`.
   - If not yet granted, safely invokes `requestPermissions` with request code `9001` without crashing the game thread.
3. **Delayed Audio Stream Policy:**
   - **SVC UDP connection and incoming remote audio playback function normally WITHOUT requesting microphone permissions.**
   - Microphone permission is requested ONLY when the user presses **Push-to-Talk** or toggles **Unmute** for the first time. This guarantees strict adherence to Android permission guidelines and avoids initializing audio input streams before consent.
4. **Custom Touch Actions:**
   - Supports `action:key.voice_chat` and `action:key.push_to_talk` in Android touch layout customizer (`touch/custom.rs`).

---

## 4. In-Game UI & Bottom-Left HUD Indicators

- **Modal Settings (Key 'V'):**
  - Real-time connection status (Disconnected, Connecting, Authenticating, Connected with RTT latency).
  - Microphone toggle (Mute / Active).
  - Voice chat master toggle (Enabled / Disabled).
  - Activation mode toggle (Voice Activity vs Push-To-Talk).
  - RNNoise AI filter toggle.
  - Button to cycle fallback compatibility versions.
- **Bottom-Left HUD Status Indicator:**
  - `[SVC: Disconnected]` (Gray)
  - `[SVC: Disabled]` (Red)
  - `[SVC: Muted]` (Orange)
  - `[SVC: Speaking]` (Green)
  - `[SVC: Whispering]` (Cyan)
  - `[SVC: XXms]` (White when ready / idle)

---

## 5. Verification & Self-Testing

1. **Unit Testing (7 Passed Cleanly):**
   - `protocol::tests::test_varint_roundtrip`: Verifies LEB128 VarInt encoding/decoding.
   - `protocol::tests::test_secret_packet_serialization`: Verifies SecretPacket parsing and serialization.
   - `protocol::tests::test_player_state_packet_serialization`: Verifies CustomPayload player state serialization.
   - `protocol::tests::test_aes_128_gcm_and_udp_framing_roundtrip`: Verifies AES-128-GCM 12-byte IV + Auth Tag wire encryption/decryption roundtrip.
   - `audio_pipeline::tests::test_pcm_codec_roundtrip`: Verifies PCM sample buffer encoding and decoding.
   - `audio_pipeline::tests::test_spatial_gain_calculation`: Verifies 3D spatial distance attenuation and stereo panning.
   - `audio_pipeline::tests::test_voice_capture_dual_subframe_vad`: Verifies dual 480-sample RNNoise neural suppression and VAD detection.
2. **Compiler & Linter Verification:**
   - `cargo check -p horizon-client --no-default-features`: **CLEAN** (0 errors, 0 warnings on voice subsystem).

---

## 6. Testing Guide on a Java SVC Server

1. Start a Paper/Purpur/Fabric Minecraft Java server with the `Simple Voice Chat` plugin/mod installed (default UDP port: `24454`).
2. Run Horizon client and connect to the server:
   ```bash
   cargo run --profile fast -p horizon-client -- --server <server_ip>:<port>
   ```
3. Upon joining, observe the client log:
   ```text
   INFO [horizon_client::voice] SVC: Received secret from server, port: 24454, codec: 0, mtu: 1024
   ```
4. Press `V` to open the Simple Voice Chat menu. Verify the status reads `Status: Connected (XX ms)`.
5. Hold `CapsLock` to speak via Push-to-Talk and verify the bottom-left HUD badge turns green `[SVC: Speaking]`.

---

🤖 Generated with [Claude Code](https://claude.com/claude-code)