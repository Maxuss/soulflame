use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoulflameConfiguration {
    pub max_players: u32,
    pub motd: String,
    pub favicon: PathBuf,
}

impl Default for SoulflameConfiguration {
    fn default() -> Self {
        SoulflameConfiguration {
            max_players: 20,
            motd: "A Soulflame server.".to_string(),
            favicon: Path::new("./soulflame/favicon.png").to_path_buf(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeConfiguration {
    pub favicon: String,
}

impl RuntimeConfiguration {
    pub async fn from_cfg(cfg: &SoulflameConfiguration) -> anyhow::Result<Self> {
        let mut favicon = File::open(&cfg.favicon).await?;
        let mut buf = vec![];
        favicon.read_to_end(&mut buf).await?;

        Ok(RuntimeConfiguration {
            favicon: build_favicon(&buf[..]),
        })
    }
}

fn build_favicon(bytes: &[u8]) -> String {
    let b = base64::encode(bytes);
    format!("data:image/png;base64,{}", b)
}
