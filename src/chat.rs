use crate::net_io::{PacketRead, PacketWrite, VarInt};
use anyhow::bail;
use log::error;
use std::io::Cursor;
use tokio::io::AsyncReadExt;

pub use lobstermessage::component::*;

pub const MAX_COMPONENT_JSON_SIZE: usize = 262144;

#[async_trait::async_trait]
impl PacketWrite for Component {
    async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
        let str = serde_json::ser::to_string(self)?;

        let bytes = str.as_bytes();
        let size = bytes.len();

        if size > MAX_COMPONENT_JSON_SIZE {
            log::error!(
                "Write Component too long (max size: {}, json size: {})",
                MAX_COMPONENT_JSON_SIZE,
                size
            );
            bail!(
                "Write Component too long (max size: {}, json size: {})",
                MAX_COMPONENT_JSON_SIZE,
                size
            );
        }

        VarInt(size as i32)
            .pack_write(buffer, target_version)
            .await?;

        buffer.extend_from_slice(bytes);

        Ok(())
    }
}

#[async_trait::async_trait]
impl PacketRead for Component {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let size = VarInt::pack_read(buffer, target_version).await?.0 as usize;

        if size > MAX_COMPONENT_JSON_SIZE {
            error!(
                "Read Component too long (max size: {}, received size: {})",
                MAX_COMPONENT_JSON_SIZE, size
            );
            bail!(
                "Read Component too long (max size: {}, received size: {})",
                MAX_COMPONENT_JSON_SIZE,
                size
            );
        }

        let mut buf = vec![0u8; size];
        AsyncReadExt::read_exact(buffer, &mut buf).await?;

        serde_json::de::from_str(&String::from_utf8(buf)?).map_err(anyhow::Error::from)
    }
}

#[macro_export]
macro_rules! component {
    // raw literals
    ($obj:literal $(& $($recurse:tt)*)?) => {
        Component::text($obj.to_string()) $(.append($crate::component!($($recurse)*)))?
    };
    // normal literals
    ( $(@ $hex_color:literal)* $(@ $named_color:ident)* $($attr:ident)* $(! $not_attr:ident)* $obj:literal $(& $($recurse:tt)*)?) => {
        Component::text($obj.to_string()) $(.$attr(true))* $(.$not_attr(false))* $(.color($crate::chat::NamedColor::$named_color))* $(.hex_color($hex_color))* $(.append($crate::component!($($recurse)*)))?
    };
    // variables
    ( $(@ $hex_color:literal)* $(@ $named_color:ident)* $($attr:ident)* $(! $not_attr:ident)* # $obj:ident $(& $($recurse:tt)*)?) => {
        Component::text($obj.to_string()) $(.$attr(true))* $(.$not_attr(false))* $(.color(NamedColor::$named_color))* $(.hex_color($hex_color))* $(.append($crate::component!($($recurse)*)))?
    };
    // expressions (no recursion here)
    ( $(@ $hex_color:literal)* $(@ $named_color:ident)* $($attr:ident)* $(! $not_attr:ident)* # $obj:expr) => {
        Component::text($obj.to_string()) $(.$attr(true))* $(.$not_attr(false))* $(.color(NamedColor::$named_color))* $(.hex_color($hex_color))*
    };
}
