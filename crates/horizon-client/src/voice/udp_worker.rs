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
        if !(1..=65535).contains(&secret.server_port) {
            return Err(format!(
                "Invalid server port in SecretPacket: {}",
                secret.server_port
            ));
        }

        let state = Arc::new(RwLock::new(UdpConnectionState::Connecting));
        let (mic_tx, mic_rx) = std::sync::mpsc::channel::<MicPacket>();
        let (sound_tx, sound_rx) = std::sync::mpsc::channel::<IncomingSound>();
        let stop_signal = Arc::new(AtomicBool::new(false));

        let (host, port) =
            parse_voice_host(&secret.voice_host, fallback_host, secret.server_port as u16);

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

        log::info!("SVC: Resolving voice target address ({host}, {port})...");
        let remote_addr = match (&host[..], port).to_socket_addrs() {
            Ok(mut addrs) => match addrs.next() {
                Some(addr) => {
                    log::info!("SVC: Resolved voice UDP target address to {addr}");
                    addr
                }
                None => {
                    set_failed(format!(
                        "Failed to resolve voice target address ({host}, {port})"
                    ));
                    return;
                }
            },
            Err(e) => {
                set_failed(format!("DNS resolution failed for ({host}, {port}): {e}"));
                return;
            }
        };

        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => {
                log::info!("SVC: Bound local UDP socket to {:?}", s.local_addr());
                s
            }
            Err(e) => {
                set_failed(format!("Failed to bind UDP socket: {e}"));
                return;
            }
        };

        if let Err(e) = socket.connect(remote_addr) {
            set_failed(format!(
                "Failed to connect UDP socket to {remote_addr}: {e}"
            ));
            return;
        }
        log::info!("SVC: Connected UDP socket to {remote_addr}");

        // Set short timeout to keep loop responsive
        let _ = socket.set_read_timeout(Some(Duration::from_millis(20)));

        let cipher = SvcCipher::new(&secret.secret);
        let conn_id = secret.connection_id;

        if let Ok(mut lock) = state.write() {
            *lock = UdpConnectionState::Authenticating;
        }

        // -------------------------------------------------------------------
        // 4-Step Handshake:
        // 1. Authenticate (0x05) with [player_uuid (16B)] + [secret (16B)]
        // 2. AuthenticateAck (0x06) -> optional nonce (v21) or empty (v20)
        // 3. ConnectionCheck (0x09) with nonce if present, else empty body
        // 4. ConnectionCheckAck (0x0A) -> empty body
        // -------------------------------------------------------------------

        let auth_body = AuthenticatePacket::new(secret.player_uuid, secret.secret).to_body();
        let auth_pkt =
            match build_client_udp_packet(conn_id, &cipher, PACKET_ID_AUTHENTICATE, &auth_body) {
                Ok(b) => b,
                Err(e) => {
                    set_failed(format!("Build auth packet error: {e}"));
                    return;
                }
            };

        let mut handshake_success = false;
        let handshake_start = Instant::now();
        let mut last_send = Instant::now() - Duration::from_secs(10);
        let mut auth_ack_received = false;
        let mut saved_nonce: Option<[u8; 16]> = None;
        let mut check_pkt: Option<Vec<u8>> = None;
        let mut auth_retry_count = 0u32;
        let mut check_retry_count = 0u32;
        let mut recv_buf = [0u8; 2048];

        log::info!("SVC: Starting UDP 4-step handshake with remote {remote_addr}");

        while handshake_start.elapsed() < Duration::from_secs(6) {
            if stop_signal.load(Ordering::Relaxed) {
                log::info!("SVC: UDP handshake aborted by stop signal");
                return;
            }

            // Retry every 500ms
            if last_send.elapsed() >= Duration::from_millis(500) {
                if auth_ack_received {
                    // AuthenticateAck received, retry ConnectionCheck (Step 3)
                    check_retry_count += 1;
                    if let Some(ref pkt) = check_pkt {
                        log::info!(
                            "SVC: [Handshake Step 3] Sending ConnectionCheck (attempt {check_retry_count}, {} bytes, has_nonce={})...",
                            pkt.len(),
                            saved_nonce.is_some()
                        );
                        let _ = socket.send(pkt);
                    } else {
                        let check_body = ConnectionCheckPacket::new(saved_nonce).to_body();
                        if let Ok(pkt) = build_client_udp_packet(
                            conn_id,
                            &cipher,
                            PACKET_ID_CONNECTION_CHECK,
                            &check_body,
                        ) {
                            log::info!(
                                "SVC: [Handshake Step 3] Sending ConnectionCheck (attempt {check_retry_count}, {} bytes, has_nonce={})...",
                                pkt.len(),
                                saved_nonce.is_some()
                            );
                            let _ = socket.send(&pkt);
                            check_pkt = Some(pkt);
                        }
                    }
                } else {
                    // AuthenticateAck not yet received, retry Authenticate (Step 1)
                    auth_retry_count += 1;
                    log::info!(
                        "SVC: [Handshake Step 1] Sending Authenticate (attempt {auth_retry_count}, {} bytes)...",
                        auth_pkt.len()
                    );
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
                                    auth_ack_received = true;
                                    saved_nonce = ack.nonce;
                                    log::info!(
                                        "SVC: [Handshake Step 2] Received AuthenticateAck ({} datagram bytes, has_nonce={}, elapsed: {:.1?})",
                                        len,
                                        saved_nonce.is_some(),
                                        handshake_start.elapsed()
                                    );
                                    // Step 3: Send initial ConnectionCheck (0x09)
                                    check_retry_count += 1;
                                    let check_body =
                                        ConnectionCheckPacket::new(saved_nonce).to_body();
                                    match build_client_udp_packet(
                                        conn_id,
                                        &cipher,
                                        PACKET_ID_CONNECTION_CHECK,
                                        &check_body,
                                    ) {
                                        Ok(pkt) => {
                                            log::info!(
                                                "SVC: [Handshake Step 3] Sending initial ConnectionCheck (attempt {check_retry_count}, {} bytes, has_nonce={})...",
                                                pkt.len(),
                                                saved_nonce.is_some()
                                            );
                                            let _ = socket.send(&pkt);
                                            check_pkt = Some(pkt);
                                            last_send = Instant::now();
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "SVC: Failed to build ConnectionCheck packet: {e}"
                                            );
                                        }
                                    }
                                }
                            }
                            PACKET_ID_CONNECTION_CHECK_ACK => {
                                // Step 4: ConnectionCheckAck received -> Voice Connected!
                                log::info!(
                                    "SVC: [Handshake Step 4] Received ConnectionCheckAck ({} datagram bytes). UDP 4-step handshake completed successfully! Voice Connected! (elapsed: {:.1?})",
                                    len,
                                    handshake_start.elapsed()
                                );
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
                Err(e) => {
                    log::debug!("SVC: UDP socket receive error during handshake: {e}");
                }
            }
        }

        if !handshake_success {
            log::warn!(
                "SVC: UDP handshake timed out after 6s (sent {} Authenticate, {} ConnectionCheck)",
                auth_retry_count,
                check_retry_count
            );
            set_failed("UDP handshake timed out after 6 seconds".to_string());
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
        let keep_alive_interval = Duration::from_millis(secret.keep_alive.clamp(500, 5000) as u64);
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
