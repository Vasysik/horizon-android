//! Simple Voice Chat (SVC) UDP Worker & Handshake Thread.
//!
//! Handles the non-blocking background UDP loop, 4-step cryptographic handshake:
//! 0x05 Authenticate -> 0x06 AuthenticateAck -> 0x09 ConnectionCheck -> 0x0A ConnectionCheckAck,
//! KeepAlive (0x08) & Ping (0x07) heartbeats, and encrypted audio I/O.

use super::protocol::*;
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum UdpConnectionState {
    Disconnected,
    Connecting,
    Authenticating,
    Connected { ping_ms: u32 },
    Failed(String),
}

/// Incoming audio sound event dispatched to the audio engine.
#[derive(Debug, Clone)]
pub enum IncomingSound {
    Player(PlayerSoundPacket),
    Group(GroupSoundPacket),
    Location(LocationSoundPacket),
}

/// Handle to control the background UDP worker thread.
pub struct UdpWorkerHandle {
    pub state: Arc<RwLock<UdpConnectionState>>,
    pub outgoing_mic_tx: Sender<MicPacket>,
    pub incoming_sound_rx: Receiver<IncomingSound>,
    pub stop_signal: Arc<AtomicBool>,
}

impl UdpWorkerHandle {
    pub fn is_connected(&self) -> bool {
        matches!(
            *self.state.read().unwrap(),
            UdpConnectionState::Connected { .. }
        )
    }

    pub fn ping_ms(&self) -> Option<u32> {
        match *self.state.read().unwrap() {
            UdpConnectionState::Connected { ping_ms } => Some(ping_ms),
            _ => None,
        }
    }

    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

pub struct UdpWorker;

impl UdpWorker {
    /// Spawn the background UDP worker thread with SecretPacket parameters.
    pub fn spawn(secret: SecretPacket, fallback_host: &str) -> Result<UdpWorkerHandle, String> {
        let state = Arc::new(RwLock::new(UdpConnectionState::Connecting));
        let (mic_tx, mic_rx) = std::sync::mpsc::channel::<MicPacket>();
        let (sound_tx, sound_rx) = std::sync::mpsc::channel::<IncomingSound>();
        let stop_signal = Arc::new(AtomicBool::new(false));

        let host = if secret.voice_host.trim().is_empty() {
            fallback_host.trim().to_string()
        } else {
            secret.voice_host.trim().to_string()
        };
        let port = secret.server_port as u16;

        let thread_state = Arc::clone(&state);
        let thread_stop = Arc::clone(&stop_signal);

        std::thread::Builder::new()
            .name("svc-udp-worker".to_string())
            .spawn(move || {
                Self::run_loop(
                    secret,
                    host,
                    port,
                    thread_state,
                    thread_stop,
                    mic_rx,
                    sound_tx,
                );
            })
            .map_err(|e| format!("Failed to spawn UDP worker thread: {e}"))?;

        Ok(UdpWorkerHandle {
            state,
            outgoing_mic_tx: mic_tx,
            incoming_sound_rx: sound_rx,
            stop_signal,
        })
    }

    fn run_loop(
        secret: SecretPacket,
        host: String,
        port: u16,
        state: Arc<RwLock<UdpConnectionState>>,
        stop_signal: Arc<AtomicBool>,
        mic_rx: Receiver<MicPacket>,
        sound_tx: Sender<IncomingSound>,
    ) {
        let set_failed = |msg: String| {
            if let Ok(mut lock) = state.write() {
                *lock = UdpConnectionState::Failed(msg);
            }
        };

        let target_addr_str = format!("{}:{}", host, port);
        let remote_addr = match target_addr_str.to_socket_addrs() {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => addr,
                None => {
                    set_failed(format!("Failed to resolve voice host: {target_addr_str}"));
                    return;
                }
            },
            Err(e) => {
                set_failed(format!("DNS resolution failed for {target_addr_str}: {e}"));
                return;
            }
        };

        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                set_failed(format!("Failed to bind UDP socket: {e}"));
                return;
            }
        };

        if let Err(e) = socket.connect(remote_addr) {
            set_failed(format!("Failed to connect UDP socket to {remote_addr}: {e}"));
            return;
        }

        // Set short timeout to keep loop responsive
        let _ = socket.set_read_timeout(Some(Duration::from_millis(20)));

        let cipher = SvcCipher::new(&secret.secret);
        let conn_id = secret.connection_id;

        if let Ok(mut lock) = state.write() {
            *lock = UdpConnectionState::Authenticating;
        }

        // -------------------------------------------------------------------
        // 4-Step Handshake:
        // 1. Authenticate (0x05)
        // 2. AuthenticateAck (0x06) -> Nonce
        // 3. ConnectionCheck (0x09) with Nonce
        // 4. ConnectionCheckAck (0x0A)
        // -------------------------------------------------------------------

        let auth_pkt = match build_client_udp_packet(conn_id, &cipher, PACKET_ID_AUTHENTICATE, &[])
        {
            Ok(b) => b,
            Err(e) => {
                set_failed(format!("Build auth packet error: {e}"));
                return;
            }
        };

        let mut handshake_success = false;
        let handshake_start = Instant::now();
        let mut last_send = Instant::now() - Duration::from_secs(10);
        let mut saved_nonce: Option<[u8; 16]> = None;
        let mut check_pkt: Option<Vec<u8>> = None;
        let mut recv_buf = [0u8; 2048];

        while handshake_start.elapsed() < Duration::from_secs(6) {
            if stop_signal.load(Ordering::Relaxed) {
                return;
            }

            // Retry every 500ms
            if last_send.elapsed() >= Duration::from_millis(500) {
                if let Some(ref nonce) = saved_nonce {
                    // Nonce is known, retry ConnectionCheck (Step 3) with saved nonce
                    if let Some(ref pkt) = check_pkt {
                        let _ = socket.send(pkt);
                    } else if let Ok(pkt) = build_client_udp_packet(
                        conn_id,
                        &cipher,
                        PACKET_ID_CONNECTION_CHECK,
                        nonce,
                    ) {
                        let _ = socket.send(&pkt);
                        check_pkt = Some(pkt);
                    }
                } else {
                    // Nonce not yet received, retry Authenticate (Step 1)
                    let _ = socket.send(&auth_pkt);
                }
                last_send = Instant::now();
            }

            match socket.recv(&mut recv_buf) {
                Ok(len) => {
                    if let Ok((pkt_id, body)) = parse_server_udp_packet(&recv_buf[..len], &cipher) {
                        match pkt_id {
                            PACKET_ID_AUTHENTICATE_ACK => {
                                if let Ok(ack) = AuthenticateAckPacket::from_body(&body) {
                                    // Step 2: Extract and save 16-byte nonce
                                    saved_nonce = Some(ack.nonce);
                                    // Step 3: Send ConnectionCheck (0x09) with nonce
                                    match build_client_udp_packet(
                                        conn_id,
                                        &cipher,
                                        PACKET_ID_CONNECTION_CHECK,
                                        &ack.nonce,
                                    ) {
                                        Ok(pkt) => {
                                            let _ = socket.send(&pkt);
                                            check_pkt = Some(pkt);
                                            last_send = Instant::now();
                                        }
                                        Err(e) => {
                                            log::warn!("SVC: Failed to build ConnectionCheck packet: {e}");
                                        }
                                    }
                                }
                            }
                            PACKET_ID_CONNECTION_CHECK_ACK => {
                                // Step 4: ConnectionCheckAck received
                                log::info!("SVC: UDP 4-step handshake completed successfully");
                                handshake_success = true;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // Non-blocking / short read timeout, continue loop
                }
                Err(_) => {
                    // Socket error, continue until retry or timeout
                }
            }
        }

        if !handshake_success {
            set_failed("UDP handshake timed out (no Authenticate/ConnectionCheck Ack)".to_string());
            return;
        }

        if let Ok(mut lock) = state.write() {
            *lock = UdpConnectionState::Connected { ping_ms: 0 };
        }

        // -------------------------------------------------------------------
        // Active Connected Loop: KeepAlive, Ping, and Audio Transmission
        // -------------------------------------------------------------------
        let mut last_keep_alive = Instant::now();
        let mut last_ping_sent = Instant::now();
        let keep_alive_interval =
            Duration::from_millis(secret.keep_alive.max(500).min(5000) as u64);
        let ping_interval = Duration::from_millis(1000);
        let mut current_ping_id: u128 = 0;
        let mut current_ping_time = Instant::now();

        while !stop_signal.load(Ordering::Relaxed) {
            // 1. Process Outgoing Mic Packets
            while let Ok(mic) = mic_rx.try_recv() {
                let body = mic.to_body();
                if let Ok(pkt) = build_client_udp_packet(conn_id, &cipher, PACKET_ID_MIC, &body) {
                    let _ = socket.send(&pkt);
                }
            }

            // 2. KeepAlive periodic heartbeat
            if last_keep_alive.elapsed() >= keep_alive_interval {
                if let Ok(ka_pkt) =
                    build_client_udp_packet(conn_id, &cipher, PACKET_ID_KEEP_ALIVE, &[])
                {
                    let _ = socket.send(&ka_pkt);
                }
                last_keep_alive = Instant::now();
            }

            // 3. Ping latency check
            if last_ping_sent.elapsed() >= ping_interval {
                current_ping_id = current_ping_id.wrapping_add(1);
                current_ping_time = Instant::now();
                let ping = PingPacket {
                    id: current_ping_id,
                    timestamp: current_ping_time.elapsed().as_millis() as i64,
                };
                if let Ok(pkt) =
                    build_client_udp_packet(conn_id, &cipher, PACKET_ID_PING, &ping.to_body())
                {
                    let _ = socket.send(&pkt);
                }
                last_ping_sent = Instant::now();
            }

            // 4. Receive Incoming Packets
            match socket.recv(&mut recv_buf) {
                Ok(len) => {
                    if let Ok((pkt_id, body)) = parse_server_udp_packet(&recv_buf[..len], &cipher) {
                        match pkt_id {
                            PACKET_ID_PING => {
                                let rtt_ms = current_ping_time.elapsed().as_millis() as u32;
                                if let Ok(mut lock) = state.write() {
                                    *lock = UdpConnectionState::Connected { ping_ms: rtt_ms };
                                }
                            }
                            PACKET_ID_KEEP_ALIVE => {
                                // Server keepalive ack
                            }
                            PACKET_ID_PLAYER_SOUND => {
                                if let Ok(sound) = PlayerSoundPacket::from_body(&body) {
                                    let _ = sound_tx.send(IncomingSound::Player(sound));
                                }
                            }
                            PACKET_ID_GROUP_SOUND => {
                                if let Ok(sound) = GroupSoundPacket::from_body(&body) {
                                    let _ = sound_tx.send(IncomingSound::Group(sound));
                                }
                            }
                            PACKET_ID_LOCATION_SOUND => {
                                if let Ok(sound) = LocationSoundPacket::from_body(&body) {
                                    let _ = sound_tx.send(IncomingSound::Location(sound));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // Normal timeout from short read_timeout
                }
                Err(_) => {
                    // UDP socket error
                }
            }
        }

        if let Ok(mut lock) = state.write() {
            *lock = UdpConnectionState::Disconnected;
        }
    }
}
