use std::io::Cursor;
use crate::net_io::{PacketRead, PacketWrite, VarInt};

#[derive(Debug, Clone)]
pub struct PacketProxy {
    staging: Vec<u8>,
    start_index: usize
}

impl PacketProxy {
    pub fn new() -> Self {
        Self {
            staging: vec![],
            start_index: 0
        }
    }

    pub async fn encode<P: PacketWrite>(&mut self, buffer: &mut Vec<u8>, packet: &P) -> anyhow::Result<()> {
        packet.pack_write(&mut self.staging, 759).await?;

        self.encode_uncompressed(buffer).await?;

        self.staging.clear();

        Ok(())
    }

    async fn encode_uncompressed(&mut self, buffer: &mut Vec<u8>) -> anyhow::Result<()> {
        let slice = self.staging.as_slice();

        let packet_len = slice.len();

        VarInt(packet_len as i32).pack_write(buffer, 759).await?;

        buffer.extend_from_slice(slice);

        Ok(())
    }

    pub fn accept(&mut self, bytes: &[u8]) {
        self.start_index += bytes.len();
        self.staging.extend(bytes);
    }

    pub async fn next<P: PacketRead>(&mut self) -> anyhow::Result<Option<P>> {
        let mut cursor = Cursor::new(&self.staging[..]);
        let packet = if let Ok(length) = VarInt::pack_read(&mut cursor, 759).await {
            let pos = cursor.position() as usize;
            let len = length.0 as usize;

            if self.staging.len() - pos >= len {
                cursor = Cursor::new(&self.staging[pos..(pos + len)]);

                let packet = P::pack_read(&mut cursor, 759).await?;

                let amount = len + pos;
                self.staging = self.staging.split_off(amount);


                Some(packet)
            } else {
                None
            }
        } else {
            None
        };

        Ok(packet)
    }
}
