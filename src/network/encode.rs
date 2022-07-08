use crate::net_io::{PacketRead, PacketWrite, VarInt};
use std::io::Cursor;
use aes::Aes128;
use aes::cipher::{AsyncStreamCipher, KeyIvInit};
use async_compression::tokio::bufread::{ZlibDecoder, ZlibEncoder};
use cfb8::{Decryptor, Encryptor};
use tokio::io::AsyncReadExt;
use crate::LATEST_PROTOCOL_VERSION;

pub type AesEnc = Encryptor<Aes128>;
pub type AesDec = Decryptor<Aes128>;

#[derive(Debug, Clone)]
pub struct PacketEncoder {
    encryptor: Option<AesEnc>,
    shared_secret: Option<[u8; 16]>,
    staging_buf: Vec<u8>,

    compression_threshold: Option<usize>,
    compression_buf: Vec<u8>
}

impl PacketEncoder {
    pub fn new() -> Self {
        Self {
            encryptor: None,
            shared_secret: None,
            staging_buf: vec![],
            compression_threshold: None,
            compression_buf: vec![]
        }
    }

    pub fn set_encryption(&mut self, key: [u8; 16]) {
        self.encryptor = Some(AesEnc::new_from_slices(&key, &key).expect("Invalid key size!"));
        self.shared_secret = Some(key);
    }

    pub fn set_compression(&mut self, threshold: usize) {
        self.compression_threshold = Some(threshold);
    }

    pub async fn consume<P: PacketWrite>(
        &mut self,
        out_buffer: &mut Vec<u8>,
        packet: &P
    ) -> anyhow::Result<()> {
        packet.pack_write(&mut self.staging_buf, LATEST_PROTOCOL_VERSION).await?;

        if let Some(_) = self.compression_threshold {
            self.write_compressed(out_buffer).await?;
        } else {
            self.write(out_buffer).await?;
        }

        if let Some(enc) = &mut self.encryptor {
            enc.clone().encrypt(out_buffer);
        }

        self.staging_buf.clear();

        Ok(())
    }

    async fn write_compressed(&mut self, buffer: &mut Vec<u8>) -> anyhow::Result<()> {
        let threshold = self.compression_threshold.unwrap();
        let mut data_len = 0;
        let mut slice = self.staging_buf.as_slice();
        if slice.len() >= threshold {
            // packet is bigger than threshold, compressing
            let mut encoder = ZlibEncoder::new(slice);
            encoder.read_to_end(&mut self.compression_buf).await?;
            slice = self.compression_buf.as_slice();
            data_len = self.staging_buf.len();
        }

        let mut buf: Vec<u8> = vec![];
        VarInt(data_len as i32).pack_write(&mut buf, LATEST_PROTOCOL_VERSION).await?;

        let packet_size = buf.len() + slice.len();
        VarInt(packet_size as i32).pack_write(&mut buf, LATEST_PROTOCOL_VERSION).await?;
        VarInt(data_len as i32).pack_write(&mut buf, LATEST_PROTOCOL_VERSION).await?;
        buffer.extend_from_slice(&buf);

        self.compression_buf.clear();

        Ok(())
    }

    async fn write(&mut self, buffer: &mut Vec<u8>) -> anyhow::Result<()> {
        let packet_len = self.staging_buf.len() as i32;
        VarInt(packet_len).pack_write(buffer, LATEST_PROTOCOL_VERSION).await?;
        buffer.extend_from_slice(&self.staging_buf);

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PacketDecoder {
    decryptor: Option<AesDec>,
    shared_secret: Option<[u8; 16]>,
    staging_buf: Vec<u8>,

    compression_threshold: Option<usize>,
    compression_buf: Vec<u8>
}

impl PacketDecoder {
    pub fn new() -> Self {
        Self {
            decryptor: None,
            shared_secret: None,
            staging_buf: vec![],
            compression_threshold: None,
            compression_buf: vec![]
        }
    }

    pub fn set_encryption(&mut self, key: [u8; 16]) {
        self.decryptor = Some(AesDec::new_from_slices(&key, &key).expect("Invalid key size!"));
        self.shared_secret = Some(key);
    }

    pub fn set_compression(&mut self, threshold: usize) {
        self.compression_threshold = Some(threshold);
    }

    pub fn digest(&mut self, packet_bytes: &[u8]) {
        self.staging_buf.extend(packet_bytes);

        if let Some(dec) = &mut self.decryptor {
            dec.clone().decrypt(&mut self.staging_buf[..]);
        }
    }

    pub async fn read<P: PacketRead>(&mut self) -> anyhow::Result<Option<P>> {
        let mut reader = Cursor::new(&self.staging_buf[..]);
        let packet = if let Ok(VarInt(size)) = VarInt::pack_read(&mut reader, LATEST_PROTOCOL_VERSION).await {
            let varint_len = reader.position() as usize;

            if self.staging_buf.len() - varint_len >= size as usize {
                reader = Cursor::new(&self.staging_buf[varint_len..varint_len + size as usize]);

                if let Some(_) = self.compression_threshold {
                    let VarInt(data_len) = VarInt::pack_read(&mut reader, LATEST_PROTOCOL_VERSION).await?;

                    if data_len > 0 {
                        let mut dec = ZlibDecoder::new(&reader.get_ref()[reader.position() as usize ..]);
                        dec.read_to_end(&mut self.compression_buf).await?;
                        reader = Cursor::new(&self.compression_buf);
                    }
                }

                let packet = P::pack_read(&mut reader, LATEST_PROTOCOL_VERSION).await?;

                let read = size as usize + varint_len;
                self.staging_buf = self.staging_buf.split_off(read);

                self.compression_buf.clear();

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