pub mod client;
pub mod proxy;

use crate::cfg::{RuntimeConfiguration, SoulflameConfiguration};
use crate::network::client::ClientConnection;
use anyhow::{bail, Context};
use log::{info, warn};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

pub struct NetworkListener {
    inner: TcpListener,
    players: PlayerCount,
    config: SoulflameConfiguration,
    runtime: RuntimeConfiguration,
}

impl NetworkListener {
    pub async fn init(
        addr: String,
        port: u16,
        configuration: SoulflameConfiguration,
    ) -> anyhow::Result<()> {
        let listener = TcpListener::bind(format!("{}:{}", addr, port))
            .await
            .context("Failed to bind to port! Is it already in use?")?;

        info!("Started network listener successfully!");

        let this = NetworkListener {
            inner: listener,
            players: PlayerCount::new(configuration.max_players),
            config: configuration.clone(),
            runtime: RuntimeConfiguration::from_cfg(&configuration).await?,
        };

        this.network_loop().await;

        Ok(())
    }

    async fn network_loop(mut self) {
        loop {
            if let Ok((stream, addr)) = self.inner.accept().await {
                self.proceed(stream, addr).await;
            }
        }
    }

    async fn proceed(&mut self, stream: TcpStream, addr: SocketAddr) {
        let connection = ClientConnection::new(
            stream,
            addr,
            self.players.clone(),
            self.config.clone(),
            self.runtime.clone(),
        )
        .await;
        connection.start();
    }
}

#[derive(Clone)]
pub struct PlayerCount {
    inner: Arc<Players>,
}

impl PlayerCount {
    pub fn new(max: u32) -> Self {
        Self {
            inner: Arc::new(Players {
                count: AtomicU32::new(0),
                max,
            }),
        }
    }

    pub fn try_add(&mut self) -> anyhow::Result<()> {
        loop {
            let count = self.inner.count.load(Ordering::SeqCst);
            let new = count + 1;

            if new > self.inner.max {
                warn!(
                    "Client tried to join, but max amount of players were online ({})",
                    self.inner.max
                );
                bail!("Max player amount reached!")
            }

            if self
                .inner
                .count
                .compare_exchange(count, new, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                return Ok(());
            }
        }
    }

    pub fn remove_player(&mut self) {
        self.inner.count.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn get(&self) -> u32 {
        self.inner.count.load(Ordering::Acquire)
    }
}

struct Players {
    count: AtomicU32,
    max: u32,
}
