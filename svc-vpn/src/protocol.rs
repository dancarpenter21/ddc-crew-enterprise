use std::{
    collections::{HashMap, VecDeque},
    net::{IpAddr, SocketAddr},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, bail};
use blake2::{
    Blake2s256, Blake2sMac,
    digest::{Digest, KeyInit as BlakeKeyInit, Mac, consts::U16},
};
use chacha20poly1305::{
    ChaCha20Poly1305,
    aead::{Aead, Payload},
};
use constant_time_eq::constant_time_eq;
use generic_array::GenericArray;
use rand_core::{OsRng, RngCore};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::config::{Config, Key};

const CONSTRUCTION: &[u8] = b"Noise_IKpsk2_25519_ChaChaPoly_BLAKE2s";
const IDENTIFIER: &[u8] = b"WireGuard v1 zx2c4 Jason@zx2c4.com";
const LABEL_MAC1: &[u8] = b"mac1----";
const INITIATION_LEN: usize = 148;
const RESPONSE_LEN: usize = 92;
const TRANSPORT_HEADER_LEN: usize = 16;
const AEAD_TAG_LEN: usize = 16;
const REPLAY_WINDOW_SIZE: usize = 8192;
const REJECT_AFTER_MESSAGES: u64 = u64::MAX - (1 << 13);
const REJECT_AFTER_TIME: Duration = Duration::from_secs(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    HandshakeInitiation,
    HandshakeResponse,
    CookieReply,
    TransportData,
}

impl TryFrom<u32> for MessageKind {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::HandshakeInitiation),
            2 => Ok(Self::HandshakeResponse),
            3 => Ok(Self::CookieReply),
            4 => Ok(Self::TransportData),
            _ => bail!("unknown WireGuard message type {value}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainPacket {
    pub destination: IpAddr,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundUdp {
    pub destination: SocketAddr,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ProtocolEvents {
    pub udp: Vec<OutboundUdp>,
    pub tun: Vec<PlainPacket>,
}

pub fn classify_wireguard_message(packet: &[u8]) -> anyhow::Result<MessageKind> {
    if packet.len() < 4 {
        bail!("packet too short for WireGuard message type");
    }
    let message_type = u32::from_le_bytes(packet[0..4].try_into()?);
    MessageKind::try_from(message_type)
}

pub fn parse_ipv4_destination(packet: &[u8]) -> anyhow::Result<IpAddr> {
    if packet.len() < 20 {
        bail!("packet too short for IPv4 header");
    }
    let version = packet[0] >> 4;
    if version != 4 {
        bail!("only IPv4 packets are supported in this MVP");
    }
    Ok(IpAddr::from([
        packet[16], packet[17], packet[18], packet[19],
    ]))
}

#[derive(Debug, Clone)]
pub struct ReplayWindow {
    newest: u64,
    seen: VecDeque<u64>,
    capacity: usize,
}

impl ReplayWindow {
    pub fn new(capacity: usize) -> Self {
        Self {
            newest: 0,
            seen: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn accept(&mut self, counter: u64) -> bool {
        if counter == 0 {
            return false;
        }
        if counter + self.capacity as u64 <= self.newest {
            return false;
        }
        if self.seen.contains(&counter) {
            return false;
        }
        self.newest = self.newest.max(counter);
        self.seen.push_back(counter);
        while self.seen.len() > self.capacity {
            self.seen.pop_front();
        }
        true
    }
}

pub struct WireGuardEngine {
    static_private: StaticSecret,
    static_public: [u8; 32],
    peers: Vec<ProtocolPeer>,
    peers_by_static: HashMap<[u8; 32], usize>,
    sessions_by_receiver: HashMap<u32, SessionRef>,
    sessions: HashMap<u32, Session>,
}

#[derive(Debug)]
struct ProtocolPeer {
    name: String,
    static_public: [u8; 32],
    preshared_key: [u8; 32],
    endpoint: Option<SocketAddr>,
    last_timestamp: Option<[u8; 12]>,
    active: Option<SessionRef>,
    pending: Option<SessionRef>,
}

#[derive(Debug, Clone, Copy)]
struct SessionRef {
    peer_index: usize,
    local_index: u32,
    remote_index: u32,
}

#[derive(Zeroize, ZeroizeOnDrop)]
struct Session {
    created_at_secs: u64,
    send_key: [u8; 32],
    recv_key: [u8; 32],
    send_counter: u64,
    #[zeroize(skip)]
    replay: ReplayWindow,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("created_at_secs", &self.created_at_secs)
            .field("send_counter", &self.send_counter)
            .field("replay", &self.replay)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Zeroize, ZeroizeOnDrop)]
struct HandshakeState {
    chaining_key: [u8; 32],
    hash: [u8; 32],
}

impl WireGuardEngine {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        let static_private = StaticSecret::from(*config.server.private_key.as_bytes());
        let static_public = PublicKey::from(&static_private).to_bytes();

        let peers = config
            .peers
            .iter()
            .map(|peer| ProtocolPeer {
                name: peer.name.clone(),
                static_public: *peer.public_key.as_bytes(),
                preshared_key: peer
                    .preshared_key
                    .as_ref()
                    .map(key_bytes)
                    .unwrap_or([0u8; 32]),
                endpoint: None,
                last_timestamp: None,
                active: None,
                pending: None,
            })
            .collect::<Vec<_>>();

        let peers_by_static = peers
            .iter()
            .enumerate()
            .map(|(index, peer)| (peer.static_public, index))
            .collect();

        Ok(Self {
            static_private,
            static_public,
            peers,
            peers_by_static,
            sessions_by_receiver: HashMap::new(),
            sessions: HashMap::new(),
        })
    }

    pub fn new() -> Self {
        let private = StaticSecret::random_from_rng(OsRng);
        Self {
            static_public: PublicKey::from(&private).to_bytes(),
            static_private: private,
            peers: Vec::new(),
            peers_by_static: HashMap::new(),
            sessions_by_receiver: HashMap::new(),
            sessions: HashMap::new(),
        }
    }

    pub fn handle_udp(
        &mut self,
        packet: &[u8],
        source: SocketAddr,
    ) -> anyhow::Result<ProtocolEvents> {
        match classify_wireguard_message(packet)? {
            MessageKind::HandshakeInitiation => self.handle_handshake_initiation(packet, source),
            MessageKind::TransportData => self.handle_transport(packet, source),
            MessageKind::HandshakeResponse | MessageKind::CookieReply => {
                Ok(ProtocolEvents::default())
            }
        }
    }

    pub fn handle_tun(
        &mut self,
        peer_index: usize,
        packet: &[u8],
    ) -> anyhow::Result<ProtocolEvents> {
        let mut events = ProtocolEvents::default();
        let peer = self
            .peers
            .get(peer_index)
            .with_context(|| format!("unknown peer index {peer_index}"))?;
        let Some(session_ref) = peer.active else {
            bail!("peer {} has no active WireGuard session", peer.name);
        };
        let Some(endpoint) = peer.endpoint else {
            bail!("peer {} has no authenticated endpoint", peer.name);
        };
        let encrypted = self.encrypt_transport(session_ref.local_index, packet)?;
        events.udp.push(OutboundUdp {
            destination: endpoint,
            bytes: encrypted,
        });
        Ok(events)
    }

    pub fn tick(&mut self, _now: SystemTime) -> anyhow::Result<ProtocolEvents> {
        Ok(ProtocolEvents::default())
    }

    pub fn receive_udp(&mut self, packet: &[u8]) -> anyhow::Result<Option<PlainPacket>> {
        let source = "127.0.0.1:0".parse().expect("valid socket address");
        Ok(self.handle_udp(packet, source)?.tun.into_iter().next())
    }

    pub fn encrypt_for_peer(
        &mut self,
        peer_index: usize,
        packet: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        Ok(self
            .handle_tun(peer_index, packet)?
            .udp
            .into_iter()
            .next()
            .context("no outbound packet produced")?
            .bytes)
    }

    fn handle_handshake_initiation(
        &mut self,
        packet: &[u8],
        source: SocketAddr,
    ) -> anyhow::Result<ProtocolEvents> {
        if packet.len() != INITIATION_LEN {
            bail!("invalid handshake initiation length {}", packet.len());
        }
        verify_mac1(&self.static_public, packet)?;

        let sender = read_u32(packet, 4);
        let initiator_ephemeral = read_array::<32>(packet, 8);
        let encrypted_static = &packet[40..88];
        let encrypted_timestamp = &packet[88..116];

        let mut state = HandshakeState::new();
        state.mix_hash(&self.static_public);
        state.mix_hash(&initiator_ephemeral);
        state.mix_key(&initiator_ephemeral);

        let ephemeral_public = PublicKey::from(initiator_ephemeral);
        let es = self
            .static_private
            .diffie_hellman(&ephemeral_public)
            .to_bytes();
        let static_key = state.mix_key(&es);
        let initiator_static = decrypt_aead(&static_key, 0, &state.hash, encrypted_static)
            .context("decrypt initiator static key")?;
        if initiator_static.len() != 32 {
            bail!("invalid initiator static key length");
        }
        state.mix_hash(encrypted_static);

        let initiator_static = array_from_slice::<32>(&initiator_static)?;
        let Some(&peer_index) = self.peers_by_static.get(&initiator_static) else {
            bail!("unknown initiator static key");
        };

        let ss = self
            .static_private
            .diffie_hellman(&PublicKey::from(initiator_static))
            .to_bytes();
        let timestamp_key = state.mix_key(&ss);
        let timestamp = decrypt_aead(&timestamp_key, 0, &state.hash, encrypted_timestamp)
            .context("decrypt timestamp")?;
        if timestamp.len() != 12 {
            bail!("invalid timestamp length");
        }
        state.mix_hash(encrypted_timestamp);

        let timestamp = array_from_slice::<12>(&timestamp)?;
        let peer = &mut self.peers[peer_index];
        if peer
            .last_timestamp
            .is_some_and(|last| timestamp.as_slice() <= last.as_slice())
        {
            bail!("replayed or stale handshake timestamp");
        }
        peer.last_timestamp = Some(timestamp);
        peer.endpoint = Some(source);

        let response_private = StaticSecret::random_from_rng(OsRng);
        let response_public = PublicKey::from(&response_private).to_bytes();
        state.mix_hash(&response_public);
        state.mix_key(&response_public);

        let ee = response_private
            .diffie_hellman(&PublicKey::from(initiator_ephemeral))
            .to_bytes();
        state.mix_key(&ee);
        let se = response_private
            .diffie_hellman(&PublicKey::from(initiator_static))
            .to_bytes();
        state.mix_key(&se);

        let psk = peer.preshared_key;
        let (tau, send_key, recv_key) = state.mix_psk(&psk);
        state.mix_hash(&tau);
        let encrypted_empty = encrypt_aead(&send_key, 0, &state.hash, &[])?;
        state.mix_hash(&encrypted_empty);

        let receiver = random_index();
        let mut response = Vec::with_capacity(RESPONSE_LEN);
        response.extend_from_slice(&2u32.to_le_bytes());
        response.extend_from_slice(&receiver.to_le_bytes());
        response.extend_from_slice(&sender.to_le_bytes());
        response.extend_from_slice(&response_public);
        response.extend_from_slice(&encrypted_empty);
        append_macs(&self.static_public, &mut response);

        let local_ref = SessionRef {
            peer_index,
            local_index: receiver,
            remote_index: sender,
        };
        self.install_session(local_ref, send_key, recv_key, false);
        self.peers[peer_index].pending = Some(local_ref);

        Ok(ProtocolEvents {
            udp: vec![OutboundUdp {
                destination: source,
                bytes: response,
            }],
            tun: Vec::new(),
        })
    }

    fn handle_transport(
        &mut self,
        packet: &[u8],
        source: SocketAddr,
    ) -> anyhow::Result<ProtocolEvents> {
        if packet.len() < TRANSPORT_HEADER_LEN + AEAD_TAG_LEN {
            bail!("transport packet too short");
        }
        if packet[1..4] != [0, 0, 0] {
            bail!("transport reserved bytes must be zero");
        }

        let receiver = read_u32(packet, 4);
        let counter = read_u64(packet, 8);
        let session_ref = *self
            .sessions_by_receiver
            .get(&receiver)
            .context("unknown receiver index")?;

        let session = self
            .session_mut(session_ref)
            .context("receiver index references missing session")?;
        if !session.replay.accept(counter) {
            bail!("replayed transport counter");
        }
        if now_secs().saturating_sub(session.created_at_secs) > REJECT_AFTER_TIME.as_secs() {
            bail!("transport session expired");
        }

        let plaintext = decrypt_aead(
            &session.recv_key,
            counter,
            &[],
            &packet[TRANSPORT_HEADER_LEN..],
        )
        .context("decrypt transport packet")?;
        let plaintext = strip_padding(plaintext);
        let destination = parse_ipv4_destination(&plaintext)?;

        let peer = &mut self.peers[session_ref.peer_index];
        peer.endpoint = Some(source);
        if peer
            .pending
            .is_some_and(|pending| pending.local_index == receiver)
        {
            peer.active = peer.pending.take();
        }

        Ok(ProtocolEvents {
            udp: Vec::new(),
            tun: vec![PlainPacket {
                destination,
                bytes: plaintext,
            }],
        })
    }

    fn encrypt_transport(&mut self, receiver: u32, packet: &[u8]) -> anyhow::Result<Vec<u8>> {
        let session_ref = *self
            .sessions_by_receiver
            .get(&receiver)
            .context("unknown local session index")?;
        let remote_index = session_ref.remote_index;
        let session = self
            .session_mut(session_ref)
            .context("local session index references missing session")?;
        if session.send_counter >= REJECT_AFTER_MESSAGES {
            bail!("transport send counter exhausted");
        }
        let counter = session.send_counter;
        session.send_counter += 1;

        let padded = pad_packet(packet);
        let ciphertext = encrypt_aead(&session.send_key, counter, &[], &padded)?;

        let mut out = Vec::with_capacity(TRANSPORT_HEADER_LEN + ciphertext.len());
        out.extend_from_slice(&4u32.to_le_bytes());
        out.extend_from_slice(&remote_index.to_le_bytes());
        out.extend_from_slice(&counter.to_le_bytes());
        out.extend_from_slice(&ciphertext);
        Ok(out)
    }

    fn install_session(
        &mut self,
        session_ref: SessionRef,
        send_key: [u8; 32],
        recv_key: [u8; 32],
        active: bool,
    ) {
        self.sessions_by_receiver
            .insert(session_ref.local_index, session_ref);
        let session = Session {
            created_at_secs: now_secs(),
            send_key,
            recv_key,
            send_counter: 0,
            replay: ReplayWindow::new(REPLAY_WINDOW_SIZE),
        };
        let peer = &mut self.peers[session_ref.peer_index];
        if active {
            peer.active = Some(session_ref);
        } else {
            peer.pending = Some(session_ref);
        }
        self.sessions.insert(session_ref.local_index, session);
    }

    fn session_mut(&mut self, session_ref: SessionRef) -> Option<&mut Session> {
        self.sessions.get_mut(&session_ref.local_index)
    }
}

impl Default for WireGuardEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl HandshakeState {
    fn new() -> Self {
        let chaining_key = blake2s(CONSTRUCTION);
        let mut hash = blake2s(&chaining_key);
        hash_update(&mut hash, IDENTIFIER);
        Self { chaining_key, hash }
    }

    fn mix_hash(&mut self, data: &[u8]) {
        hash_update(&mut self.hash, data);
    }

    fn mix_key(&mut self, input: &[u8]) -> [u8; 32] {
        let (ck, temp) = kdf1(&self.chaining_key, input);
        self.chaining_key = ck;
        temp
    }

    fn mix_psk(&mut self, psk: &[u8; 32]) -> ([u8; 32], [u8; 32], [u8; 32]) {
        let (ck, tau, key) = kdf3(&self.chaining_key, psk);
        self.chaining_key = ck;
        (tau, key, key)
    }
}

fn key_bytes(key: &Key) -> [u8; 32] {
    *key.as_bytes()
}

fn verify_mac1(static_public: &[u8; 32], packet: &[u8]) -> anyhow::Result<()> {
    let body_len = packet
        .len()
        .checked_sub(32)
        .context("packet too short for MACs")?;
    let key = blake2s_join(LABEL_MAC1, static_public);
    let expected = keyed_blake2s_16(&key, &packet[..body_len]);
    if !constant_time_eq(&expected, &packet[body_len..body_len + 16]) {
        bail!("invalid mac1");
    }
    Ok(())
}

fn append_macs(static_public: &[u8; 32], packet: &mut Vec<u8>) {
    let key = blake2s_join(LABEL_MAC1, static_public);
    let mac1 = keyed_blake2s_16(&key, packet);
    packet.extend_from_slice(&mac1);
    packet.extend_from_slice(&[0u8; 16]);
}

fn encrypt_aead(
    key: &[u8; 32],
    counter: u64,
    aad: &[u8],
    plaintext: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    let nonce = nonce(counter);
    cipher
        .encrypt(
            GenericArray::from_slice(&nonce),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| anyhow::anyhow!("AEAD encryption failed"))
}

fn decrypt_aead(
    key: &[u8; 32],
    counter: u64,
    aad: &[u8],
    ciphertext: &[u8],
) -> anyhow::Result<Vec<u8>> {
    let cipher = ChaCha20Poly1305::new(GenericArray::from_slice(key));
    let nonce = nonce(counter);
    cipher
        .decrypt(
            GenericArray::from_slice(&nonce),
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| anyhow::anyhow!("AEAD decryption failed"))
}

fn nonce(counter: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[4..].copy_from_slice(&counter.to_le_bytes());
    nonce
}

fn kdf1(key: &[u8; 32], input: &[u8]) -> ([u8; 32], [u8; 32]) {
    let temp_key = hmac_blake2s(key, input);
    let out1 = hmac_blake2s(&temp_key, &[1]);
    (out1, out1)
}

fn kdf3(key: &[u8; 32], input: &[u8]) -> ([u8; 32], [u8; 32], [u8; 32]) {
    let temp_key = hmac_blake2s(key, input);
    let out1 = hmac_blake2s(&temp_key, &[1]);
    let mut in2 = Vec::from(out1);
    in2.push(2);
    let out2 = hmac_blake2s(&temp_key, &in2);
    let mut in3 = Vec::from(out2);
    in3.push(3);
    let out3 = hmac_blake2s(&temp_key, &in3);
    (out1, out2, out3)
}

fn hmac_blake2s(key: &[u8; 32], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for index in 0..key.len() {
        ipad[index] ^= key[index];
        opad[index] ^= key[index];
    }

    let mut inner = Blake2s256::new();
    inner.update(ipad);
    inner.update(data);
    let inner = inner.finalize();

    let mut outer = Blake2s256::new();
    outer.update(opad);
    outer.update(inner);
    outer.finalize().into()
}

fn blake2s(data: &[u8]) -> [u8; 32] {
    Blake2s256::digest(data).into()
}

fn blake2s_join(left: &[u8], right: &[u8]) -> [u8; 32] {
    let mut hasher = Blake2s256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

fn hash_update(hash: &mut [u8; 32], data: &[u8]) {
    let mut hasher = Blake2s256::new();
    hasher.update(*hash);
    hasher.update(data);
    *hash = hasher.finalize().into();
}

fn keyed_blake2s_16(key: &[u8; 32], data: &[u8]) -> [u8; 16] {
    type Blake2sMac128 = Blake2sMac<U16>;
    let mut mac =
        <Blake2sMac128 as BlakeKeyInit>::new_from_slice(key).expect("BLAKE2s accepts 32-byte keys");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

fn pad_packet(packet: &[u8]) -> Vec<u8> {
    let padded_len = packet.len().div_ceil(16) * 16;
    let mut padded = Vec::with_capacity(padded_len);
    padded.extend_from_slice(packet);
    padded.resize(padded_len, 0);
    padded
}

fn strip_padding(mut packet: Vec<u8>) -> Vec<u8> {
    while packet.last() == Some(&0) {
        packet.pop();
    }
    packet
}

fn read_u32(packet: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        packet[offset..offset + 4]
            .try_into()
            .expect("valid u32 slice"),
    )
}

fn read_u64(packet: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        packet[offset..offset + 8]
            .try_into()
            .expect("valid u64 slice"),
    )
}

fn read_array<const N: usize>(packet: &[u8], offset: usize) -> [u8; N] {
    packet[offset..offset + N]
        .try_into()
        .expect("valid fixed slice")
}

fn array_from_slice<const N: usize>(slice: &[u8]) -> anyhow::Result<[u8; N]> {
    slice
        .try_into()
        .map_err(|_| anyhow::anyhow!("expected {N} bytes, got {}", slice.len()))
}

fn random_index() -> u32 {
    loop {
        let index = OsRng.next_u32();
        if index != 0 {
            return index;
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_transport_message() {
        assert_eq!(
            classify_wireguard_message(&4u32.to_le_bytes()).unwrap(),
            MessageKind::TransportData
        );
    }

    #[test]
    fn parses_ipv4_destination() {
        let mut packet = vec![0u8; 20];
        packet[0] = 0x45;
        packet[16..20].copy_from_slice(&[10, 44, 0, 2]);
        assert_eq!(
            parse_ipv4_destination(&packet).unwrap(),
            "10.44.0.2".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn replay_window_rejects_repeats_and_old_packets() {
        let mut replay = ReplayWindow::new(4);
        assert!(replay.accept(1));
        assert!(!replay.accept(1));
        assert!(replay.accept(5));
        assert!(!replay.accept(1));
    }
}
