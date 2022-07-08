#![allow(dead_code)]

use crate::cfg::{RuntimeConfiguration, SoulflameConfiguration};
use crate::chat::Component;
use crate::net_io::{PacketRead, PacketWrite};
use crate::network::encode::{PacketDecoder, PacketEncoder};
use crate::network::PlayerCount;
use crate::protocol::client::handshake::{HandshakeState, InHandshake};
use crate::protocol::client::play::PacketPlayIn;
use crate::protocol::client::status::{InStatus, PacketStatusInPing};
use crate::protocol::server::login::PacketLoginOutDisconnect;
use crate::protocol::server::play::{PacketPlayOut, PacketPlayOutDisconnect};
use crate::protocol::server::status::{
    OutStatus, PacketStatusOutPong, PacketStatusOutResponse, ServerPlayers, ServerVersion,
    StatusResponse,
};
use anyhow::bail;
use flume::{Receiver, Sender};
use log::{info, warn};
use std::fmt::Debug;
use std::net::SocketAddr;
use std::time::Duration;
use lobstermessage::lobster;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::time::timeout;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ProtocolState {
    Handshake,
    Status,
    Login,
    Play
}

pub struct ClientConnection {
    addr: SocketAddr,
    players: PlayerCount,
    config: SoulflameConfiguration,
    runtime: RuntimeConfiguration,
    state: ProtocolState,

    inbound: InboundPacketChannel,
    outgoing: OutgoingPacketChannel,

    send_packets: Sender<PacketPlayOut>,
    receive_packets: Receiver<PacketPlayIn>,
}

impl ClientConnection {
    pub async fn new(
        stream: TcpStream,
        addr: SocketAddr,
        players: PlayerCount,
        config: SoulflameConfiguration,
        runtime: RuntimeConfiguration,
    ) -> Self {
        let (reader, writer) = stream.into_split();

        let (receive_packets_tx, receive_packets_rx) = flume::bounded(32);
        let (send_packets_tx, send_packets_rx) = flume::unbounded();

        Self {
            addr,

            players,
            config,
            runtime,
            state: ProtocolState::Handshake,
            inbound: InboundPacketChannel::new(reader, receive_packets_tx, addr.clone()),
            outgoing: OutgoingPacketChannel::new(writer, send_packets_rx, addr.clone()),
            send_packets: send_packets_tx,
            receive_packets: receive_packets_rx,
        }
    }

    pub fn start(self) {
        tokio::task::spawn(async move {
            if let Err(e) = self.handle().await {
                info!("Client connection closed: {}", e);
            }
        });
    }

    async fn handle(mut self) -> anyhow::Result<()> {
        self.do_initial_handle().await?;

        Ok(())
    }

    async fn do_initial_handle(&mut self) -> anyhow::Result<()> {
        let InHandshake::PacketHandshakeIn(handshake) = self.read_packet().await?;
        match handshake.next_state() {
            HandshakeState::Status => {
                self.state = ProtocolState::Status;

                let _request = self.read_packet::<InStatus>().await?;

                let payload = StatusResponse::new(
                    ServerVersion::new("Latest".into(), 759),
                    ServerPlayers::new(self.config.max_players as i32, 0, vec![]),
                    lobster(&self.config.motd),
                    self.runtime.favicon.clone(),
                );

                self.send_packet(OutStatus::PacketStatusOutResponse(
                    PacketStatusOutResponse::new(payload),
                ))
                .await?;

                match self.read_packet::<PacketStatusInPing>().await {
                    Ok(ping) => {
                        self.send_packet(PacketStatusOutPong::new(*ping.payload()))
                            .await?;
                    }
                    Err(e) => {
                        warn!("Didn't receive ping packet from status call: {}", e);
                    }
                }
            }
            HandshakeState::Login => {
                self.state = ProtocolState::Login;

                warn!("Logging in is not yet implemented!");

                self.disconnect(lobster("<red>Logging in is not yet supported!")).await?;

                return Ok(())
            }
        };

        Ok(())
    }

    pub async fn disconnect(&mut self, reason: Component) -> anyhow::Result<()> {
        let mut r = reason.clone();
        match self.state {
            ProtocolState::Login => {
                self.outgoing.send_packet(PacketLoginOutDisconnect::new(reason)).await?
            }
            ProtocolState::Play => {
                self.outgoing.send_packet(PacketPlayOutDisconnect::new(reason)).await?
            }
            _ => bail!("Can not disconnect player during {:?} state!", self.state)
        };
        info!("Client {} lost connection: {}", self.addr.ip(), r.flatten());

        Ok(())
    }

    pub async fn read_packet<P: PacketRead>(&mut self) -> anyhow::Result<P> {
        self.inbound.read_packet().await
    }

    pub async fn send_packet<P: PacketWrite + Debug>(&mut self, packet: P) -> anyhow::Result<()> {
        self.outgoing.send_packet(packet).await
    }
}

pub struct InboundPacketChannel {
    reader: OwnedReadHalf,
    packets: Sender<PacketPlayIn>,
    dec: PacketDecoder,
    buffer: [u8; 1024],
    addr: SocketAddr,
}

impl InboundPacketChannel {
    pub fn new(reader: OwnedReadHalf, packets: Sender<PacketPlayIn>, addr: SocketAddr) -> Self {
        Self {
            reader,
            packets,
            dec: PacketDecoder::new(),
            buffer: [0u8; 1024],
            addr,
        }
    }

    pub fn set_encryption(&mut self, key: [u8; 16]) {
        self.dec.set_encryption(key);
    }

    pub fn set_compression(&mut self, threshold: usize) {
        self.dec.set_compression(threshold);
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        loop {
            let packet = self.read_packet::<PacketPlayIn>().await?;
            if let Err(_) = self.packets.send_async(packet).await {
                info!("Server dropped connection for client {}!", self.addr.ip());
                return Ok(());
            }
        }
    }

    pub async fn read_packet<P: PacketRead>(&mut self) -> anyhow::Result<P> {
        loop {
            let next: Option<P> = self.dec.read::<P>().await?;
            if let Some(packet) = next {
                return Ok(packet);
            }

            // 5s timeout
            let time = Duration::from_secs(5);

            let read = timeout(time, self.reader.read(&mut self.buffer)).await??;
            if read == 0 {
                warn!("Read 0 bytes from client!");
                bail!("Read 0 bytes from client!")
            }

            let bytes = &self.buffer[..read];
            self.dec.digest(bytes);
        }
    }
}

pub struct OutgoingPacketChannel {
    writer: OwnedWriteHalf,
    packets: Receiver<PacketPlayOut>,
    enc: PacketEncoder,
    buffer: Vec<u8>,
    addr: SocketAddr,
}

impl OutgoingPacketChannel {
    pub fn new(writer: OwnedWriteHalf, packets: Receiver<PacketPlayOut>, addr: SocketAddr) -> Self {
        Self {
            writer,
            packets,
            enc: PacketEncoder::new(),
            buffer: vec![],
            addr,
        }
    }

    pub fn set_encryption(&mut self, key: [u8; 16]) {
        self.enc.set_encryption(key);
    }

    pub fn set_compression(&mut self, threshold: usize) {
        self.enc.set_compression(threshold);
    }

    pub async fn start(mut self) -> anyhow::Result<()> {
        while let Ok(packet) = self.packets.recv_async().await {
            self.send_packet(packet).await?;
        }
        Ok(())
    }

    pub async fn send_packet<P: PacketWrite + Debug>(&mut self, packet: P) -> anyhow::Result<()> {
        self.enc.consume(&mut self.buffer, &packet).await?;
        self.writer.write_all(&self.buffer).await?;
        self.buffer.clear();
        Ok(())
    }
}
