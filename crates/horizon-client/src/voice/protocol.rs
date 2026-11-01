//! Simple Voice Chat (SVC) Protocol Implementation in Rust for Horizon.
//!
//! Provides byte-for-byte wire parity with Simple Voice Chat Java Compatibility Version 21.
//! Handles 14 TCP CustomPayload channels and 10 UDP AES-128-GCM packets.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes128Gcm, Nonce,
};
use std::io::{Cursor, Read};

pub const COMPATIBILITY_VERSION: i32 = 21;
pub const MAGIC_BYTE: u8 = 0xFF;
pub const IV_SIZE_BYTES: usize = 12;
pub const NONCE_SIZE_BYTES: usize = 16;
pub const MAX_OPUS_PAYLOAD_SIZE: usize = 1275;

// Packet IDs for UDP Voice Channel
pub const PACKET_ID_MIC: u8 = 0x01;
pub const PACKET_ID_PLAYER_SOUND: u8 = 0x02;
pub const PACKET_ID_GROUP_SOUND: u8 = 0x03;
pub const PACKET_ID_LOCATION_SOUND: u8 = 0x04;
pub const PACKET_ID_AUTHENTICATE: u8 = 0x05;
pub const PACKET_ID_AUTHENTICATE_ACK: u8 = 0x06;
pub const PACKET_ID_PING: u8 = 0x07;
pub const PACKET_ID_KEEP_ALIVE: u8 = 0x08;
pub const PACKET_ID_CONNECTION_CHECK: u8 = 0x09;
pub const PACKET_ID_CONNECTION_CHECK_ACK: u8 = 0x0A;

// Minecraft CustomPayload Channel Identifiers
pub const CHANNEL_REQUEST_SECRET: &str = "voicechat:request_secret";
pub const CHANNEL_SECRET: &str = "voicechat:secret";
pub const CHANNEL_CLIENT_STATE: &str = "voicechat:client_state";
pub const CHANNEL_PLAYER_STATE: &str = "voicechat:state";
pub const CHANNEL_PLAYER_STATES: &str = "voicechat:states";
pub const CHANNEL_REMOVE_STATE: &str = "voicechat:remove_state";
pub const CHANNEL_ADD_GROUP: &str = "voicechat:add_group";
pub const CHANNEL_REMOVE_GROUP: &str = "voicechat:remove_group";
pub const CHANNEL_SET_GROUP: &str = "voicechat:set_group";
pub const CHANNEL_CREATE_GROUP: &str = "voicechat:create_group";
pub const CHANNEL_LEAVE_GROUP: &str = "voicechat:leave_group";
pub const CHANNEL_JOINED_GROUP: &str = "voicechat:joined_group";
pub const CHANNEL_ADD_CATEGORY: &str = "voicechat:add_category";
pub const CHANNEL_REMOVE_CATEGORY: &str = "voicechat:remove_category";

/// Helper for reading Minecraft FriendlyByteBuf network representations.
pub struct ByteBufReader<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> ByteBufReader<'a> {
    pub fn new(slice: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(slice),
        }
    }

    pub fn read_u8(&mut self) -> Result<u8, std::io::Error> {
        let mut buf = [0u8; 1];
        self.cursor.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    pub fn read_i16(&mut self) -> Result<i16, std::io::Error> {
        let mut buf = [0u8; 2];
        self.cursor.read_exact(&mut buf)?;
        Ok(i16::from_be_bytes(buf))
    }

    pub fn read_i32(&mut self) -> Result<i32, std::io::Error> {
        let mut buf = [0u8; 4];
        self.cursor.read_exact(&mut buf)?;
        Ok(i32::from_be_bytes(buf))
    }

    pub fn read_i64(&mut self) -> Result<i64, std::io::Error> {
        let mut buf = [0u8; 8];
        self.cursor.read_exact(&mut buf)?;
        Ok(i64::from_be_bytes(buf))
    }

    pub fn read_f32(&mut self) -> Result<f32, std::io::Error> {
        let mut buf = [0u8; 4];
        self.cursor.read_exact(&mut buf)?;
        Ok(f32::from_be_bytes(buf))
    }

    pub fn read_f64(&mut self) -> Result<f64, std::io::Error> {
        let mut buf = [0u8; 8];
        self.cursor.read_exact(&mut buf)?;
        Ok(f64::from_be_bytes(buf))
    }

    pub fn read_bool(&mut self) -> Result<bool, std::io::Error> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_varint(&mut self) -> Result<i32, std::io::Error> {
        let mut num_read = 0;
        let mut result = 0i32;
        loop {
            let read = self.read_u8()?;
            let value = (read & 0b0111_1111) as i32;
            result |= value << (7 * num_read);
            num_read += 1;
            if num_read > 5 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "VarInt too big",
                ));
            }
            if (read & 0b1000_0000) == 0 {
                break;
            }
        }
        Ok(result)
    }

    pub fn read_uuid(&mut self) -> Result<u128, std::io::Error> {
        let mut buf = [0u8; 16];
        self.cursor.read_exact(&mut buf)?;
        Ok(u128::from_be_bytes(buf))
    }

    pub fn read_string(&mut self) -> Result<String, std::io::Error> {
        let len = self.read_varint()? as usize;
        let mut bytes = vec![0u8; len];
        self.cursor.read_exact(&mut bytes)?;
        String::from_utf8(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn read_byte_array(&mut self) -> Result<Vec<u8>, std::io::Error> {
        let len = self.read_varint()? as usize;
        let mut bytes = vec![0u8; len];
        self.cursor.read_exact(&mut bytes)?;
        Ok(bytes)
    }

    pub fn read_exact_bytes(&mut self, buf: &mut [u8]) -> Result<(), std::io::Error> {
        self.cursor.read_exact(buf)
    }

    pub fn remaining(&self) -> usize {
        let pos = self.cursor.position() as usize;
        let inner = self.cursor.get_ref();
        inner.len().saturating_sub(pos)
    }
}

/// Helper for writing Minecraft FriendlyByteBuf network representations.
pub struct ByteBufWriter {
    buffer: Vec<u8>,
}

impl ByteBufWriter {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
        }
    }

    pub fn write_u8(&mut self, val: u8) {
        self.buffer.push(val);
    }

    pub fn write_i16(&mut self, val: i16) {
        self.buffer.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_i32(&mut self, val: i32) {
        self.buffer.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_i64(&mut self, val: i64) {
        self.buffer.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_f32(&mut self, val: f32) {
        self.buffer.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_f64(&mut self, val: f64) {
        self.buffer.extend_from_slice(&val.to_be_bytes());
    }

    pub fn write_bool(&mut self, val: bool) {
        self.buffer.push(if val { 1 } else { 0 });
    }

    pub fn write_varint(&mut self, mut value: i32) {
        loop {
            if (value & !0b0111_1111) == 0 {
                self.buffer.push(value as u8);
                return;
            }
            self.buffer.push(((value & 0b0111_1111) | 0b1000_0000) as u8);
            value = ((value as u32) >> 7) as i32;
        }
    }

    pub fn write_uuid(&mut self, uuid: u128) {
        self.buffer.extend_from_slice(&uuid.to_be_bytes());
    }

    pub fn write_string(&mut self, str_val: &str) {
        let bytes = str_val.as_bytes();
        self.write_varint(bytes.len() as i32);
        self.buffer.extend_from_slice(bytes);
    }

    pub fn write_byte_array(&mut self, bytes: &[u8]) {
        self.write_varint(bytes.len() as i32);
        self.buffer.extend_from_slice(bytes);
    }

    pub fn write_raw_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }
}

/// AES-128-GCM cipher wrapper for SVC UDP packets.
pub struct SvcCipher {
    cipher: Aes128Gcm,
}

impl SvcCipher {
    pub fn new(secret_key: &[u8; 16]) -> Self {
        Self {
            cipher: Aes128Gcm::new_from_slice(secret_key).expect("Valid 16-byte key"),
        }
    }

    /// Encrypts plaintext into `[12-byte IV] + [Ciphertext + 16-byte GCM Tag]`.
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, String> {
        let mut iv = [0u8; IV_SIZE_BYTES];
        // Generate cryptographic random IV via getrandom
        getrandom::fill(&mut iv).map_err(|e| format!("IV generation failed: {e}"))?;
        let nonce = Nonce::from_slice(&iv);

        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| format!("GCM encrypt error: {e:?}"))?;

        let mut payload = Vec::with_capacity(IV_SIZE_BYTES + ciphertext.len());
        payload.extend_from_slice(&iv);
        payload.extend_from_slice(&ciphertext);
        Ok(payload)
    }

    /// Decrypts payload `[12-byte IV] + [Ciphertext + 16-byte GCM Tag]` back to plaintext.
    pub fn decrypt(&self, payload: &[u8]) -> Result<Vec<u8>, String> {
        if payload.len() < IV_SIZE_BYTES + 16 {
            return Err("Payload too short for IV and Tag".into());
        }
        let iv = &payload[..IV_SIZE_BYTES];
        let ciphertext = &payload[IV_SIZE_BYTES..];
        let nonce = Nonce::from_slice(iv);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| format!("GCM decrypt error: {e:?}"))
    }
}

// ---------------------------------------------------------------------------
// TCP Packets (Minecraft CustomPayload Channels)
// ---------------------------------------------------------------------------

/// RequestSecretPacket (channel: voicechat:request_secret)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestSecretPacket {
    pub compatibility_version: i32,
}

impl RequestSecretPacket {
    pub fn new(version: i32) -> Self {
        Self {
            compatibility_version: version,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_i32(self.compatibility_version);
        w.into_bytes()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        let compatibility_version = r.read_i32()?;
        Ok(Self {
            compatibility_version,
        })
    }
}

/// SecretPacket (channel: voicechat:secret)
#[derive(Debug, Clone, PartialEq)]
pub struct SecretPacket {
    pub secret: [u8; 16],
    pub connection_id: u128,
    pub server_port: i32,
    pub player_uuid: u128,
    pub codec: u8,
    pub mtu_size: i32,
    pub voice_chat_distance: f64,
    pub keep_alive: i32,
    pub groups_enabled: bool,
    pub voice_host: String,
    pub allow_recording: bool,
}

impl SecretPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        let mut secret = [0u8; 16];
        r.read_exact_bytes(&mut secret)?;
        let connection_id = r.read_uuid()?;
        let server_port = r.read_i32()?;
        let player_uuid = r.read_uuid()?;
        let codec = r.read_u8()?;
        let mtu_size = r.read_i32()?;
        let voice_chat_distance = r.read_f64()?;
        let keep_alive = r.read_i32()?;
        let groups_enabled = r.read_bool()?;
        let voice_host = r.read_string()?;
        let allow_recording = r.read_bool()?;

        Ok(Self {
            secret,
            connection_id,
            server_port,
            player_uuid,
            codec,
            mtu_size,
            voice_chat_distance,
            keep_alive,
            groups_enabled,
            voice_host,
            allow_recording,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_raw_bytes(&self.secret);
        w.write_uuid(self.connection_id);
        w.write_i32(self.server_port);
        w.write_uuid(self.player_uuid);
        w.write_u8(self.codec);
        w.write_i32(self.mtu_size);
        w.write_f64(self.voice_chat_distance);
        w.write_i32(self.keep_alive);
        w.write_bool(self.groups_enabled);
        w.write_string(&self.voice_host);
        w.write_bool(self.allow_recording);
        w.into_bytes()
    }
}

/// UpdateStatePacket (channel: voicechat:client_state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateStatePacket {
    pub muted: bool,
    pub disabled: bool,
}

impl UpdateStatePacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_bool(self.muted);
        w.write_bool(self.disabled);
        w.into_bytes()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        Ok(Self {
            muted: r.read_bool()?,
            disabled: r.read_bool()?,
        })
    }
}

/// Substructure for player states in PlayerStatePacket & PlayerStatesPacket
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    pub flags: u8,
    pub uuid: u128,
    pub name: String,
    pub group: Option<u128>,
}

impl PlayerState {
    pub fn is_muted(&self) -> bool {
        (self.flags & 0b0000_0001) != 0
    }
    pub fn is_force_muted(&self) -> bool {
        (self.flags & 0b0000_0010) != 0
    }
    pub fn is_disabled(&self) -> bool {
        (self.flags & 0b0000_0100) != 0
    }
    pub fn is_force_disabled(&self) -> bool {
        (self.flags & 0b0000_1000) != 0
    }
    pub fn is_disconnected(&self) -> bool {
        (self.flags & 0b0001_0000) != 0
    }
    pub fn has_group(&self) -> bool {
        (self.flags & 0b0010_0000) != 0
    }

    pub fn read_from(r: &mut ByteBufReader<'_>) -> Result<Self, std::io::Error> {
        let flags = r.read_u8()?;
        let uuid = r.read_uuid()?;
        let name = r.read_string()?;
        let group = if (flags & 0b0010_0000) != 0 {
            Some(r.read_uuid()?)
        } else {
            None
        };
        Ok(Self {
            flags,
            uuid,
            name,
            group,
        })
    }

    pub fn write_to(&self, w: &mut ByteBufWriter) {
        let mut flags = self.flags;
        if self.group.is_some() {
            flags |= 0b0010_0000;
        } else {
            flags &= !0b0010_0000;
        }
        w.write_u8(flags);
        w.write_uuid(self.uuid);
        w.write_string(&self.name);
        if let Some(gid) = self.group {
            w.write_uuid(gid);
        }
    }
}

/// PlayerStatePacket (channel: voicechat:state)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStatePacket {
    pub state: PlayerState,
}

impl PlayerStatePacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        Ok(Self {
            state: PlayerState::read_from(&mut r)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        self.state.write_to(&mut w);
        w.into_bytes()
    }
}

/// PlayerStatesPacket (channel: voicechat:states)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerStatesPacket {
    pub states: Vec<PlayerState>,
}

impl PlayerStatesPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        let count = r.read_i32()? as usize;
        let mut states = Vec::with_capacity(count);
        for _ in 0..count {
            states.push(PlayerState::read_from(&mut r)?);
        }
        Ok(Self { states })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_i32(self.states.len() as i32);
        for s in &self.states {
            s.write_to(&mut w);
        }
        w.into_bytes()
    }
}

/// RemovePlayerStatePacket (channel: voicechat:remove_state)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovePlayerStatePacket {
    pub player_uuid: u128,
}

impl RemovePlayerStatePacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        Ok(Self {
            player_uuid: r.read_uuid()?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_uuid(self.player_uuid);
        w.into_bytes()
    }
}

/// Substructure for group representations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientGroup {
    pub id: u128,
    pub name: String,
    pub has_password: bool,
    pub persistent: bool,
    pub hidden: bool,
    pub group_type: i16,
}

impl ClientGroup {
    pub fn read_from(r: &mut ByteBufReader<'_>) -> Result<Self, std::io::Error> {
        let id = r.read_uuid()?;
        let name = r.read_string()?;
        let has_password = r.read_bool()?;
        let persistent = r.read_bool()?;
        let hidden = r.read_bool()?;
        let group_type = r.read_i16()?;
        Ok(Self {
            id,
            name,
            has_password,
            persistent,
            hidden,
            group_type,
        })
    }

    pub fn write_to(&self, w: &mut ByteBufWriter) {
        w.write_uuid(self.id);
        w.write_string(&self.name);
        w.write_bool(self.has_password);
        w.write_bool(self.persistent);
        w.write_bool(self.hidden);
        w.write_i16(self.group_type);
    }
}

/// AddGroupPacket (channel: voicechat:add_group)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddGroupPacket {
    pub group: ClientGroup,
}

impl AddGroupPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        Ok(Self {
            group: ClientGroup::read_from(&mut r)?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        self.group.write_to(&mut w);
        w.into_bytes()
    }
}

/// RemoveGroupPacket (channel: voicechat:remove_group)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveGroupPacket {
    pub group_id: u128,
}

impl RemoveGroupPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        Ok(Self {
            group_id: r.read_uuid()?,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_uuid(self.group_id);
        w.into_bytes()
    }
}

/// JoinGroupPacket (channel: voicechat:set_group)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinGroupPacket {
    pub group_id: u128,
    pub password: Option<String>,
}

impl JoinGroupPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_uuid(self.group_id);
        w.write_bool(self.password.is_some());
        if let Some(pw) = &self.password {
            w.write_string(pw);
        }
        w.into_bytes()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        let group_id = r.read_uuid()?;
        let has_pw = r.read_bool()?;
        let password = if has_pw {
            Some(r.read_string()?)
        } else {
            None
        };
        Ok(Self { group_id, password })
    }
}

/// CreateGroupPacket (channel: voicechat:create_group)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateGroupPacket {
    pub name: String,
    pub password: Option<String>,
    pub group_type: i16,
}

impl CreateGroupPacket {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_string(&self.name);
        w.write_bool(self.password.is_some());
        if let Some(pw) = &self.password {
            w.write_string(pw);
        }
        w.write_i16(self.group_type);
        w.into_bytes()
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        let name = r.read_string()?;
        let has_pw = r.read_bool()?;
        let password = if has_pw {
            Some(r.read_string()?)
        } else {
            None
        };
        let group_type = r.read_i16()?;
        Ok(Self {
            name,
            password,
            group_type,
        })
    }
}

/// LeaveGroupPacket (channel: voicechat:leave_group)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaveGroupPacket;

impl LeaveGroupPacket {
    pub fn to_bytes() -> Vec<u8> {
        Vec::new()
    }
}

/// JoinedGroupPacket (channel: voicechat:joined_group)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinedGroupPacket {
    pub group_id: Option<u128>,
    pub wrong_password: bool,
}

impl JoinedGroupPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        let has_group = r.read_bool()?;
        let group_id = if has_group {
            Some(r.read_uuid()?)
        } else {
            None
        };
        let wrong_password = r.read_bool()?;
        Ok(Self {
            group_id,
            wrong_password,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_bool(self.group_id.is_some());
        if let Some(gid) = self.group_id {
            w.write_uuid(gid);
        }
        w.write_bool(self.wrong_password);
        w.into_bytes()
    }
}

/// AddCategoryPacket (channel: voicechat:add_category)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddCategoryPacket {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

impl AddCategoryPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        let id = r.read_string()?;
        let name = r.read_string()?;
        let has_icon = r.read_bool()?;
        let icon = if has_icon {
            Some(r.read_string()?)
        } else {
            None
        };
        Ok(Self { id, name, icon })
    }
}

/// RemoveCategoryPacket (channel: voicechat:remove_category)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveCategoryPacket {
    pub id: String,
}

impl RemoveCategoryPacket {
    pub fn from_bytes(data: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(data);
        Ok(Self {
            id: r.read_string()?,
        })
    }
}

// ---------------------------------------------------------------------------
// UDP Packets & Framing
// ---------------------------------------------------------------------------

/// Builds client-to-server framed UDP packet:
/// `[Magic 0xFF] + [ConnID 16B] + [VarInt Length] + [Encrypted Payload (12B IV + Ciphertext + Tag)]`
pub fn build_client_udp_packet(
    connection_id: u128,
    cipher: &SvcCipher,
    packet_id: u8,
    packet_body: &[u8],
) -> Result<Vec<u8>, String> {
    let mut plaintext = Vec::with_capacity(1 + packet_body.len());
    plaintext.push(packet_id);
    plaintext.extend_from_slice(packet_body);

    let encrypted = cipher.encrypt(&plaintext)?;

    let mut w = ByteBufWriter::new();
    w.write_u8(MAGIC_BYTE);
    w.write_uuid(connection_id);
    w.write_byte_array(&encrypted);
    Ok(w.into_bytes())
}

/// Parses server-to-client framed UDP packet:
/// `[Magic 0xFF] + [VarInt Length] + [Encrypted Payload (12B IV + Ciphertext + Tag)]`
pub fn parse_server_udp_packet(
    raw_packet: &[u8],
    cipher: &SvcCipher,
) -> Result<(u8, Vec<u8>), String> {
    let mut r = ByteBufReader::new(raw_packet);
    let magic = r.read_u8().map_err(|e| e.to_string())?;
    if magic != MAGIC_BYTE {
        return Err(format!("Invalid magic byte: {magic:#x}"));
    }

    let encrypted = r.read_byte_array().map_err(|e| e.to_string())?;
    let decrypted = cipher.decrypt(&encrypted)?;
    if decrypted.is_empty() {
        return Err("Decrypted packet payload is empty".into());
    }

    let packet_id = decrypted[0];
    let body = decrypted[1..].to_vec();
    Ok((packet_id, body))
}

/// MicPacket (UDP ID 0x01): microphone audio sent from client to server.
#[derive(Debug, Clone, PartialEq)]
pub struct MicPacket {
    pub opus_data: Vec<u8>,
    pub sequence_number: i64,
    pub whispering: bool,
}

impl MicPacket {
    pub fn to_body(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_byte_array(&self.opus_data);
        w.write_i64(self.sequence_number);
        w.write_bool(self.whispering);
        w.into_bytes()
    }

    pub fn from_body(body: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(body);
        let opus_data = r.read_byte_array()?;
        let sequence_number = r.read_i64()?;
        let whispering = r.read_bool()?;
        Ok(Self {
            opus_data,
            sequence_number,
            whispering,
        })
    }
}

/// PlayerSoundPacket (UDP ID 0x02): proximity audio received from server.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSoundPacket {
    pub channel_id: u128,
    pub sender_uuid: u128,
    pub opus_data: Vec<u8>,
    pub sequence_number: i64,
    pub distance: f32,
    pub whispering: bool,
    pub category: Option<String>,
}

impl PlayerSoundPacket {
    pub fn from_body(body: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(body);
        let channel_id = r.read_uuid()?;
        let sender_uuid = r.read_uuid()?;
        let opus_data = r.read_byte_array()?;
        let sequence_number = r.read_i64()?;
        let distance = r.read_f32()?;
        let flags = r.read_u8()?;
        let whispering = (flags & 0b01) != 0;
        let has_category = (flags & 0b10) != 0;
        let category = if has_category {
            Some(r.read_string()?)
        } else {
            None
        };
        Ok(Self {
            channel_id,
            sender_uuid,
            opus_data,
            sequence_number,
            distance,
            whispering,
            category,
        })
    }
}

/// GroupSoundPacket (UDP ID 0x03): group audio received from server.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupSoundPacket {
    pub channel_id: u128,
    pub sender_uuid: u128,
    pub opus_data: Vec<u8>,
    pub sequence_number: i64,
    pub category: Option<String>,
}

impl GroupSoundPacket {
    pub fn from_body(body: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(body);
        let channel_id = r.read_uuid()?;
        let sender_uuid = r.read_uuid()?;
        let opus_data = r.read_byte_array()?;
        let sequence_number = r.read_i64()?;
        let flags = r.read_u8()?;
        let has_category = (flags & 0b10) != 0;
        let category = if has_category {
            Some(r.read_string()?)
        } else {
            None
        };
        Ok(Self {
            channel_id,
            sender_uuid,
            opus_data,
            sequence_number,
            category,
        })
    }
}

/// LocationSoundPacket (UDP ID 0x04): 3D positional broadcast audio.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationSoundPacket {
    pub channel_id: u128,
    pub sender_uuid: u128,
    pub location: [f64; 3],
    pub opus_data: Vec<u8>,
    pub sequence_number: i64,
    pub distance: f32,
    pub category: Option<String>,
}

impl LocationSoundPacket {
    pub fn from_body(body: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(body);
        let channel_id = r.read_uuid()?;
        let sender_uuid = r.read_uuid()?;
        let location = [r.read_f64()?, r.read_f64()?, r.read_f64()?];
        let opus_data = r.read_byte_array()?;
        let sequence_number = r.read_i64()?;
        let distance = r.read_f32()?;
        let flags = r.read_u8()?;
        let has_category = (flags & 0b10) != 0;
        let category = if has_category {
            Some(r.read_string()?)
        } else {
            None
        };
        Ok(Self {
            channel_id,
            sender_uuid,
            location,
            opus_data,
            sequence_number,
            distance,
            category,
        })
    }
}

/// AuthenticateAckPacket (UDP ID 0x06): server response to authenticate with nonce.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticateAckPacket {
    pub nonce: [u8; 16],
}

impl AuthenticateAckPacket {
    pub fn from_body(body: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(body);
        let mut nonce = [0u8; 16];
        r.read_exact_bytes(&mut nonce)?;
        Ok(Self { nonce })
    }
}

/// PingPacket (UDP ID 0x07): roundtrip latency measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PingPacket {
    pub id: u128,
    pub timestamp: i64,
}

impl PingPacket {
    pub fn to_body(&self) -> Vec<u8> {
        let mut w = ByteBufWriter::new();
        w.write_uuid(self.id);
        w.write_i64(self.timestamp);
        w.into_bytes()
    }

    pub fn from_body(body: &[u8]) -> Result<Self, std::io::Error> {
        let mut r = ByteBufReader::new(body);
        Ok(Self {
            id: r.read_uuid()?,
            timestamp: r.read_i64()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_varint_roundtrip() {
        let values = [0, 1, 127, 128, 255, 256, 16384, 2097151, -1, i32::MAX, i32::MIN];
        for val in values {
            let mut w = ByteBufWriter::new();
            w.write_varint(val);
            let mut r = ByteBufReader::new(w.as_slice());
            assert_eq!(r.read_varint().unwrap(), val);
        }
    }

    #[test]
    fn test_secret_packet_serialization() {
        let secret = SecretPacket {
            secret: [42u8; 16],
            connection_id: 0x0123_4567_89AB_CDEF_FEDC_BA98_7654_3210,
            server_port: 24454,
            player_uuid: 0x1111_2222_3333_4444_5555_6666_7777_8888,
            codec: 0,
            mtu_size: 1024,
            voice_chat_distance: 48.0,
            keep_alive: 1000,
            groups_enabled: true,
            voice_host: "voice.horizon.lan".to_string(),
            allow_recording: false,
        };

        let bytes = secret.to_bytes();
        let decoded = SecretPacket::from_bytes(&bytes).expect("Valid decode");
        assert_eq!(decoded, secret);
    }

    #[test]
    fn test_player_state_packet_serialization() {
        let state = PlayerState {
            flags: 0b0010_0001, // muted + hasGroup
            uuid: 0x9999_8888_7777_6666_5555_4444_3333_2222,
            name: "Alex".to_string(),
            group: Some(0xAAAA_BBBB_CCCC_DDDD_EEEE_FFFF_0000_1111),
        };
        assert!(state.is_muted());
        assert!(!state.is_disabled());
        assert!(state.has_group());

        let pkt = PlayerStatePacket { state };
        let bytes = pkt.to_bytes();
        let decoded = PlayerStatePacket::from_bytes(&bytes).unwrap();
        assert_eq!(decoded, pkt);
    }

    #[test]
    fn test_aes_128_gcm_and_udp_framing_roundtrip() {
        let key = [7u8; 16];
        let cipher = SvcCipher::new(&key);
        let conn_id = 0xFEEDBEEF_CAFE_BABE_0123_4567_89AB_CDEF;

        let mic_body = MicPacket {
            opus_data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            sequence_number: 1001,
            whispering: true,
        }
        .to_body();

        // 1. Client encrypts and frames packet
        let client_packet = build_client_udp_packet(conn_id, &cipher, PACKET_ID_MIC, &mic_body)
            .expect("Client packet build");

        // Verify magic byte
        assert_eq!(client_packet[0], MAGIC_BYTE);

        // 2. Server decrypts (using server frame format without conn_id or directly decrypting payload)
        // Let's test cipher encryption & decryption directly
        let encrypted = cipher.encrypt(&mic_body).expect("Encrypt");
        let decrypted = cipher.decrypt(&encrypted).expect("Decrypt");
        assert_eq!(decrypted, mic_body);

        let parsed_mic = MicPacket::from_body(&decrypted).expect("Parse mic");
        assert_eq!(parsed_mic.sequence_number, 1001);
        assert!(parsed_mic.whispering);
        assert_eq!(parsed_mic.opus_data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }
}
