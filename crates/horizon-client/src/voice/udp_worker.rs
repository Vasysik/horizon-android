//! Simple Voice Chat (SVC) UDP Worker & Handshake Thread.
//!
//! Handles the non-blocking background UDP loop, 4-step cryptographic handshake:
//! 0x05 Authenticate -> 0x06 AuthenticateAck -> 0x09 ConnectionCheck -> 0x0A ConnectionCheckAck,
//! KeepAlive (0x08) & Ping (0x07) heartbeats, and encrypted audio I/O.

use super::audio_pipeline::VoicePlaybackPipeline;
use super::mixer::{GainSource, VoiceMixer};
use super::protocol::*;
use std::net::{ToSocketAddrs, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, sync_channel};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub enum UdpConnectionState {
    Disconnected,
    Connecting,
    Authenticating,
    Connected { ping_ms: u32 },
    Failed(String),
}

/// Handle to control the background UDP worker thread.
pub struct UdpWorkerHandle {
    pub state: Arc<RwLock<UdpConnectionState>>,
    pub outgoing_mic_tx: SyncSender<MicPacket>,
    pub stop_signal: Arc<AtomicBool>,
}

impl UdpWorkerHandle {
    pub fn is_connected(&self) -> bool {
        self.state
            .read()
            .map(|s| matches!(*s, UdpConnectionState::Connected { .. }))
            .unwrap_or(false)
    }

    pub fn ping_ms(&self) -> Option<u32> {
        self.state.read().ok().and_then(|s| match *s {
            UdpConnectionState::Connected { ping_ms } => Some(ping_ms),
            _ => None,
        })
    }

    pub fn stop(&self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}

/// Decodes incoming voice on the UDP thread and feeds the shared mixer, so playback no longer
/// depends on the game tick rate (20 Hz) or on tick hitches.
struct VoiceRx {
    pipeline: VoicePlaybackPipeline,
    mixer: Arc<Mutex<VoiceMixer>>,
    default_max_dist: f32,
    last_maintain: Instant,
}

impl VoiceRx {
    fn new(mixer: Arc<Mutex<VoiceMixer>>, default_max_dist: f32) -> Self {
        Self {
            pipeline: VoicePlaybackPipeline::new(),
            mixer,
            default_max_dist,
            last_maintain: Instant::now(),
        }
    }

    /// Once per second: drop idle Opus decoders and silent mixer streams.
    fn maintain(&mut self) {
        if self.last_maintain.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_maintain = Instant::now();
        self.pipeline.prune_idle_decoders();
        if let Ok(mut m) = self.mixer.lock() {
            m.prune_idle(Duration::from_secs(5));
        }
    }

    fn deliver(
        &mut self,
        sender_uuid: u128,
        sequence_number: i64,
        opus_data: &[u8],
        source: GainSource,
    ) {
        let pcm = self
            .pipeline
            .decode_sender_frame(sender_uuid, sequence_number, opus_data);
        let Ok(mut m) = self.mixer.lock() else {
            return;
        };
        if opus_data.is_empty() {
            m.end_stream(sender_uuid);
        } else if !pcm.is_empty() {
            m.push(sender_uuid, source, &pcm);
        }
    }

    fn on_player(&mut self, pkt: PlayerSoundPacket) {
        let max_dist = if pkt.distance > 0.0 {
            pkt.distance
        } else {
            self.default_max_dist
        };
        self.deliver(
            pkt.sender_uuid,
            pkt.sequence_number,
            &pkt.opus_data,
            GainSource::Player { max_dist },
        );
    }

    fn on_group(&mut self, pkt: GroupSoundPacket) {
        self.deliver(
            pkt.sender_uuid,
            pkt.sequence_number,
            &pkt.opus_data,
            GainSource::Group,
        );
    }

    fn on_location(&mut self, pkt: LocationSoundPacket) {
        let max_dist = if pkt.distance > 0.0 {
            pkt.distance
        } else {
            self.default_max_dist
        };
        self.deliver(
            pkt.sender_uuid,
            pkt.sequence_number,
            &pkt.opus_data,
            GainSource::Location {
                pos: pkt.location,
                max_dist,
            },
        );
    }
}

pub struct UdpWorker;

impl UdpWorker {
    /// Spawn the background UDP worker thread with SecretPacket parameters.
    pub fn spawn(
        secret: SecretPacket,
        fallback_host: &str,
        mixer: Arc<Mutex<VoiceMixer>>,
    ) -> Result<UdpWorkerHandle, String> {
        if !(1..=65535).contains(&secret.server_port) {
            return Err(format!(
                "Invalid server port in SecretPacket: {}",
                secret.server_port
            ));
        }

        let state = Arc::new(RwLock::new(UdpConnectionState::Connecting));
        let (mic_tx, mic_rx) = sync_channel::<MicPacket>(128);
        let stop_signal = Arc::new(AtomicBool::new(false));

        let (host, port) =
            parse_voice_host(&secret.voice_host, fallback_host, secret.server_port as u16);

        let fallback_str = fallback_host.to_string();
        let thread_state = Arc::clone(&state);
        let thread_stop = Arc::clone(&stop_signal);

        std::thread::Builder::new()
            .name("svc-udp-worker".to_string())
            .spawn(move || {
                Self::run_loop(
                    secret,
                    host,
                    fallback_str,
                    port,
                    thread_state,
                    thread_stop,
                    mic_rx,
                    mixer,
                );
            })
            .map_err(|e| format!("Failed to spawn UDP worker thread: {e}"))?;

        Ok(UdpWorkerHandle {
            state,
            outgoing_mic_tx: mic_tx,
            stop_signal,
        })
    }

    fn run_loop(
        secret: SecretPacket,
        host: String,
        fallback_host: String,
        port: u16,
        state: Arc<RwLock<UdpConnectionState>>,
        stop_signal: Arc<AtomicBool>,
        mic_rx: Receiver<MicPacket>,
        mixer: Arc<Mutex<VoiceMixer>>,
    ) {
        let set_failed = |msg: String| {
            if let Ok(mut lock) = state.write() {
                *lock = UdpConnectionState::Failed(msg);
            }
        };

        log::info!("SVC: Resolving voice target address ({host}, {port})...");
        let mut resolved: Option<SocketAddr> = None;
        let mut last_err = String::new();

        for attempt in 1..=3 {
            if stop_signal.load(Ordering::Relaxed) {
                return;
            }
            match (&host[..], port).to_socket_addrs() {
                Ok(mut addrs) => {
                    if let Some(addr) = addrs.next() {
                        log::info!(
                            "SVC: Resolved voice UDP target address to {addr} on attempt {attempt}"
                        );
                        resolved = Some(addr);
                        break;
                    }
                }
                Err(e) => {
                    last_err = format!("DNS resolution error: {e}");
                    std::thread::sleep(Duration::from_millis(250));
                }
            }
        }

        if resolved.is_none() && !fallback_host.is_empty() && fallback_host != host {
            log::warn!(
                "SVC: Primary host {host} resolution failed, trying fallback {fallback_host}..."
            );
            for attempt in 1..=3 {
                if stop_signal.load(Ordering::Relaxed) {
                    return;
                }
                match (&fallback_host[..], port).to_socket_addrs() {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            log::info!(
                                "SVC: Resolved fallback voice UDP target address to {addr} on attempt {attempt}"
                            );
                            resolved = Some(addr);
                            break;
                        }
                    }
                    Err(e) => {
                        last_err = format!("Fallback DNS resolution error: {e}");
                        std::thread::sleep(Duration::from_millis(250));
                    }
                }
            }
        }

        let remote_addr = match resolved {
            Some(addr) => addr,
            None => {
                set_failed(format!(
                    "Failed to resolve voice target address ({host}, {port}): {last_err}"
                ));
                return;
            }
        };

        let bind_addr = if remote_addr.is_ipv6() {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        };

        let socket = match UdpSocket::bind(bind_addr) {
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

        let _ = socket.set_read_timeout(Some(Duration::from_millis(15)));

        let cipher = SvcCipher::new(&secret.secret);
        let conn_id = secret.connection_id;

        if let Ok(mut lock) = state.write() {
            *lock = UdpConnectionState::Authenticating;
        }

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

        while handshake_start.elapsed() < Duration::from_secs(15) {
            if stop_signal.load(Ordering::Relaxed) {
                log::info!("SVC: UDP handshake aborted by stop signal");
                return;
            }

            if last_send.elapsed() >= Duration::from_millis(500) {
                if auth_ack_received {
                    check_retry_count += 1;
                    if let Some(ref pkt) = check_pkt {
                        let _ = socket.send(pkt);
                    } else {
                        let check_body = ConnectionCheckPacket::new(saved_nonce).to_body();
                        if let Ok(pkt) = build_client_udp_packet(
                            conn_id,
                            &cipher,
                            PACKET_ID_CONNECTION_CHECK,
                            &check_body,
                        ) {
                            let _ = socket.send(&pkt);
                            check_pkt = Some(pkt);
                        }
                    }
                } else {
                    auth_retry_count += 1;
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
                                    check_retry_count += 1;
                                    let check_body =
                                        ConnectionCheckPacket::new(saved_nonce).to_body();
                                    if let Ok(pkt) = build_client_udp_packet(
                                        conn_id,
                                        &cipher,
                                        PACKET_ID_CONNECTION_CHECK,
                                        &check_body,
                                    ) {
                                        let _ = socket.send(&pkt);
                                        check_pkt = Some(pkt);
                                        last_send = Instant::now();
                                    }
                                }
                            }
                            PACKET_ID_CONNECTION_CHECK_ACK => {
                                log::info!(
                                    "SVC: [Handshake Step 4] Voice Connected! (elapsed: {:.1?})",
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
                        || e.kind() == std::io::ErrorKind::TimedOut => {}
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

        // Opus decoders are created on this thread, so they never have to be `Send`.
        let mut rx = VoiceRx::new(mixer, secret.voice_chat_distance as f32);

        let mut last_keep_alive = Instant::now();
        let mut last_ping_sent = Instant::now();
        let keep_alive_interval = Duration::from_millis(secret.keep_alive.clamp(500, 5000) as u64);
        let ping_interval = Duration::from_millis(1000);
        let mut current_ping_id: u128 = 0;
        let mut current_ping_time = Instant::now();

        while !stop_signal.load(Ordering::Relaxed) {
            rx.maintain();

            while let Ok(mic) = mic_rx.try_recv() {
                let body = mic.to_body();
                if let Ok(pkt) = build_client_udp_packet(conn_id, &cipher, PACKET_ID_MIC, &body) {
                    let _ = socket.send(&pkt);
                }
            }

            if last_keep_alive.elapsed() >= keep_alive_interval {
                if let Ok(ka_pkt) =
                    build_client_udp_packet(conn_id, &cipher, PACKET_ID_KEEP_ALIVE, &[])
                {
                    let _ = socket.send(&ka_pkt);
                }
                last_keep_alive = Instant::now();
            }

            if last_ping_sent.elapsed() >= ping_interval {
                current_ping_id = current_ping_id.wrapping_add(1);
                current_ping_time = Instant::now();
                let unix_ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                let ping = PingPacket {
                    id: current_ping_id,
                    timestamp: unix_ms,
                };
                if let Ok(pkt) =
                    build_client_udp_packet(conn_id, &cipher, PACKET_ID_PING, &ping.to_body())
                {
                    let _ = socket.send(&pkt);
                }
                last_ping_sent = Instant::now();
            }

            for _ in 0..16 {
                match socket.recv(&mut recv_buf) {
                    Ok(len) => {
                        if let Ok((pkt_id, body)) =
                            parse_server_udp_packet(&recv_buf[..len], &cipher)
                        {
                            match pkt_id {
                                PACKET_ID_PING => {
                                    if let Ok(pong) = PingPacket::from_body(&body)
                                        && pong.id == current_ping_id
                                    {
                                        let rtt_ms = current_ping_time.elapsed().as_millis() as u32;
                                        if let Ok(mut lock) = state.write() {
                                            *lock =
                                                UdpConnectionState::Connected { ping_ms: rtt_ms };
                                        }
                                    }
                                }
                                PACKET_ID_KEEP_ALIVE => {
                                    if let Ok(ka_pkt) = build_client_udp_packet(
                                        conn_id,
                                        &cipher,
                                        PACKET_ID_KEEP_ALIVE,
                                        &[],
                                    ) {
                                        let _ = socket.send(&ka_pkt);
                                    }
                                }
                                PACKET_ID_PLAYER_SOUND => {
                                    if let Ok(sound) = PlayerSoundPacket::from_body(&body) {
                                        rx.on_player(sound);
                                    }
                                }
                                PACKET_ID_GROUP_SOUND => {
                                    if let Ok(sound) = GroupSoundPacket::from_body(&body) {
                                        rx.on_group(sound);
                                    }
                                }
                                PACKET_ID_LOCATION_SOUND => {
                                    if let Ok(sound) = LocationSoundPacket::from_body(&body) {
                                        rx.on_location(sound);
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
                        break;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        }

        if let Ok(mut lock) = state.write() {
            *lock = UdpConnectionState::Disconnected;
        }
    }
}
