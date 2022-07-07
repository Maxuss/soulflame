#![allow(unused_variables)]

pub mod packet;

use crate::util::Identifier;
use anyhow::bail;
use async_trait::async_trait;
use log::error;
use std::io::Cursor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::{Bytes, Uuid};

#[async_trait::async_trait]
pub trait PacketWrite: Sized {
    async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32)
        -> anyhow::Result<()>;
}

#[async_trait]
pub trait PacketRead: Sized {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self>;
}

macro_rules! __primitive_impl {
    ($(
    $i:ident, $write:ident, $read:ident
    ),* $(,)?) => {
        $(
            #[async_trait]
            impl PacketRead for $i {
                async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
                    buffer.$read().await.map_err(anyhow::Error::from)
                }
            }

            #[async_trait]
            impl PacketWrite for $i {
                async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
                    buffer.$write(*self).await.map_err(anyhow::Error::from)?;
                    Ok(())
                }
            }
        )*
    };
}

__primitive_impl!(
    u8, write_u8, read_u8, i8, write_i8, read_i8, u16, write_u16, read_u16, i16, write_i16,
    read_i16, u32, write_u32, read_u32, i32, write_i32, read_i32, u64, write_u64, read_u64, i64,
    write_i64, read_i64, u128, write_u128, read_u128, i128, write_i128, read_i128, f32, write_f32,
    read_f32, f64, write_f64, read_f64
);

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VarInt(pub i32);

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct VarLong(pub i64);

impl From<i32> for VarInt {
    fn from(v: i32) -> Self {
        VarInt(v)
    }
}

impl From<VarInt> for i32 {
    fn from(v: VarInt) -> Self {
        v.0
    }
}

impl From<i64> for VarLong {
    fn from(v: i64) -> Self {
        VarLong(v)
    }
}

impl From<VarLong> for i64 {
    fn from(v: VarLong) -> Self {
        v.0
    }
}

#[async_trait]
impl PacketWrite for VarInt {
    async fn pack_write(&self,
        buffer: &mut Vec<u8>,
        target_version: u32,
    ) -> anyhow::Result<()> {
        let mut v = self.0 as u32;
        loop {
            let mut temp = (v & 0b0111_1111) as u8;
            v = v >> 7;
            if v != 0 {
                temp = temp | 0b1000_0000;
            }

            buffer.write_all(&[temp]).await?;

            if v == 0 {
                break;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl PacketRead for VarInt {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let mut size = 0;
        let mut v = 0;

        loop {
            let r = buffer.read_u8().await?;
            let value = i32::from(r & 0b0111_1111);
            v = v | value.overflowing_shl(7 * size).0;

            size += 1;

            if size > 5 {
                error!("VarInt too long (max size: 5, read: {}", v);
                bail!("VarInt too long (max size: 5, read: {}", v);
            }

            if r & 0b1000_0000 == 0 {
                break;
            }
        }

        Ok(VarInt(v))
    }
}

#[async_trait]
impl PacketWrite for VarLong {
    async fn pack_write(&self,
        buffer: &mut Vec<u8>,
        target_version: u32,
    ) -> anyhow::Result<()> {
        let mut v = self.0 as u64;
        loop {
            let mut temp = (v & 0b0111_1111) as u8;
            v = v >> 7;
            if v != 0 {
                temp = temp | 0b1000_0000;
            }

            buffer.write_all(&[temp]).await?;

            if v == 0 {
                break;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl PacketRead for VarLong {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let mut size = 0;
        let mut v = 0;

        loop {
            let r = buffer.read_u8().await?;
            let value = i64::from(r & 0b0111_1111);
            v = v | value.overflowing_shl(7 * size).0;

            size += 1;

            if size > 10 {
                error!("VarLong too long (max size: 10, read: {}", v);
                bail!("VarLong too long (max size: 10, read: {}", v);
            }

            if r & 0b1000_0000 == 0 {
                break;
            }
        }

        Ok(VarLong(v))
    }
}

#[async_trait]
impl<T> PacketWrite for Option<T>
where
    T: PacketWrite + Send + Sync,
{
    async fn pack_write(&self,
        buffer: &mut Vec<u8>,
        target_version: u32,
    ) -> anyhow::Result<()> {
        match self {
            Some(v) => {
                buffer.write_u8(1).await?;
                v.pack_write(buffer, target_version).await?;
            }
            None => {
                buffer.write_u8(0).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl<T> PacketRead for Option<T>
where
    T: PacketRead + Send,
{
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        return if buffer.read_u8().await? == 0 {
            Ok(None)
        } else {
            Ok(Some(T::pack_read(buffer, target_version).await?))
        };
    }
}

#[async_trait]
impl PacketWrite for bool {
    async fn pack_write(&self,
        buffer: &mut Vec<u8>,
        target_version: u32,
    ) -> anyhow::Result<()> {
        buffer.write_u8(if *self { 1 } else { 0 }).await?;
        Ok(())
    }
}

#[async_trait]
impl PacketRead for bool {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        Ok(buffer.read_u8().await? == 1)
    }
}

const MAX_STRING_SIZE: usize = 32767;

#[async_trait]
impl PacketRead for String {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let size = VarInt::pack_read(buffer, target_version).await?.0 as usize;

        if size > MAX_STRING_SIZE {
            error!(
                "Read String too long (max size: {}, received size: {})",
                MAX_STRING_SIZE, size
            );
            bail!(
                "Read String too long (max size: {}, received size: {})",
                MAX_STRING_SIZE,
                size
            );
        }

        let mut buf = vec![0u8; size];
        AsyncReadExt::read_exact(buffer, &mut buf).await?;

        String::from_utf8(buf).map_err(anyhow::Error::from)
    }
}

#[async_trait]
impl PacketWrite for String {
    async fn pack_write(&self,
        buffer: &mut Vec<u8>,
        target_version: u32,
    ) -> anyhow::Result<()> {
        let bytes = self.as_bytes();
        let size = bytes.len();

        if size > MAX_STRING_SIZE {
            error!(
                "Write String too long (max size: {}, string size: {})",
                MAX_STRING_SIZE, size
            );
            bail!(
                "Write String too long (max size: {}, string size: {})",
                MAX_STRING_SIZE,
                size
            );
        }

        VarInt(size as i32).pack_write(buffer, target_version).await?;

        buffer.extend_from_slice(bytes);

        Ok(())
    }
}

#[async_trait]
impl PacketRead for Identifier {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        Identifier::parse(String::pack_read(buffer, target_version).await?)
    }
}

#[async_trait]
impl PacketWrite for Identifier {
    async fn pack_write(&self,
        buffer: &mut Vec<u8>,
        target_version: u32,
    ) -> anyhow::Result<()> {
        self.to_string().pack_write(buffer, target_version).await
    }
}

#[async_trait]
impl PacketRead for Uuid {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let mut bytes = Bytes::default();

        AsyncReadExt::read_exact(buffer, &mut bytes).await?;

        Ok(Uuid::from_bytes(bytes))
    }
}

#[async_trait]
impl PacketWrite for Uuid {
    async fn pack_write(&self,
        buffer: &mut Vec<u8>,
        target_version: u32,
    ) -> anyhow::Result<()> {
        buffer.extend_from_slice(self.as_bytes());
        Ok(())
    }
}

const MAX_ARRAY_SIZE: usize = 1024 * 1024; // 2^20

#[async_trait]
impl<T> PacketRead for Vec<T>
where T: PacketRead + Send {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let size = VarInt::pack_read(buffer, target_version).await?.0 as usize;

        if size > MAX_ARRAY_SIZE {
            error!("Tried to read array of size {}, which is larger than max size ({})", size, MAX_ARRAY_SIZE);
            bail!("Tried to read array of size {}, which is larger than max size ({})", size, MAX_ARRAY_SIZE);
        }

        let mut vals = vec![];

        for _ in 0..size {
            vals.push(T::pack_read(buffer, target_version).await?);
        }

        Ok(vals)
    }
}

#[async_trait]
impl<T> PacketWrite for Vec<T>
where T: PacketWrite + Send + Sync {
    async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
        let size = self.len();

        if size > MAX_ARRAY_SIZE {
            error!("Tried to write array of size {}, which is larger than max size ({})", size, MAX_ARRAY_SIZE);
            bail!("Tried to write array of size {}, which is larger than max size ({})", size, MAX_ARRAY_SIZE);
        }

        VarInt(size as i32).pack_write(buffer, target_version).await?;

        for v in self {
            v.pack_write(buffer, target_version).await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ByteArray(pub Vec<u8>);

#[async_trait]
impl PacketWrite for ByteArray {
    async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
        buffer.extend_from_slice(&self.0);
        Ok(())
    }
}

#[async_trait]
impl PacketRead for ByteArray {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let mut buf = vec![];

        buffer.read_to_end(&mut buf).await?;

        Ok(ByteArray(buf))
    }
}