use std::io::Cursor;

#[async_trait::async_trait]
pub trait PacketWrite {
    async fn write(&mut self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait PacketRead {
    async fn read(buffer: &mut Cursor<Vec<u8>>, target_version: u32) -> anyhow::Result<Self>;
}