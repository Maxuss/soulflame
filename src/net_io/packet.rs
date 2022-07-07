use serde::{Serialize, Deserialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ProtocolSide {
    Outgoing,
    Inbound
}

pub trait PacketStage {
    fn name() -> String;
    fn side() -> ProtocolSide;
}

pub trait Packet<S> {
    fn packet_id() -> u32;
    fn side() -> ProtocolSide;
    fn into_stage(self) -> S;
}

#[doc(hidden)]
#[macro_export]
macro_rules! simplify {
    (VarInt) => {
        i32
    };
    (VarLong) => {
        i64
    };
    (ByteArray) => {
        Vec<u8>
    };
    ($typ:ty) => {
        $typ
    }
}

#[macro_export]
macro_rules! json_packet_struct {
    ($(
    $name:ident {
        $(
        $field:ident: $ty:ident $(<$generic:ident>)?
        ),* $(,)?
    }
    );* $(;)?) => {
        $(
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct $name {
             $(
             $field: $ty$(<$generic>)?
             ),*
        }

        impl $name {
            pub fn new($($field: $ty$(<$generic>)?),*) -> Self {
                Self {
                    $($field),*
                }
            }
        }

        #[async_trait::async_trait]
        impl $crate::net_io::PacketRead for $name {
            async fn pack_read(buffer: &mut std::io::Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
                Ok(serde_json::de::from_str::<$name>(&String::pack_read(buffer, target_version).await?)?)
            }
        }

        #[async_trait::async_trait]
        impl $crate::net_io::PacketWrite for $name {
            async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
                serde_json::ser::to_string(self)?.pack_write(buffer, target_version).await
            }
        }
        )*
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! writeable {
    (VarInt, $e:expr) => {
        VarInt($e as i32)
    };
    (VarLong, $e:expr) => {
        VarLong($e as i64)
    };
    (ByteArray, $e:expr) => {
        ByteArray($e)
    };
    ($typ:ty, $e:expr) => {
        $e
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! storage {
    (VarInt, $e:expr) => {
        $e.0
    };
    (VarLong, $e:expr) => {
        $e.0
    };
    (ByteArray, $e:expr) => {
        $e.0
    };
    ($typ:ty, $e:expr) => {
        $e
    }
}

#[macro_export]
macro_rules! define_enum {
    ($(
    $name:ident {
        $(
        $field:ident = $value:literal
        ),* $(,)*
    }
    );* $(;)*) => {
        $(
        #[derive(Debug, Copy, Clone, serde::Serialize, serde::Deserialize)]
        pub enum $name {
            $(
            $field
            ),*
        }

        impl $name {
            fn id(&self) -> $crate::net_io::VarInt {
                use $name::*;
                $crate::net_io::VarInt(match self {
                    $(
                    $field => $value,
                    )*
                })
            }

            fn from_id(id: i32) -> anyhow::Result<$name> {
                use $name::*;
                Ok(match id {
                    $(
                    $value => $field,
                    )*
                    _ => {
                        log::warn!("Invalid {} ID provided: {}!", stringify!($name), id);
                        anyhow::bail!("Invalid {} ID provided: {}!", stringify!($name), id)
                    }
                })
            }
        }

        #[async_trait::async_trait]
        impl $crate::net_io::PacketRead for $name {
            async fn pack_read(buffer: &mut std::io::Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
                $name::from_id($crate::net_io::VarInt::pack_read(buffer, target_version).await?.0)
            }
        }

        #[async_trait::async_trait]
        impl $crate::net_io::PacketWrite for $name {
            async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
                self.id().pack_write(buffer, target_version).await
            }
        }
        )*
    };
}

#[macro_export]
macro_rules! staged_packets {
    (
        $stage:ident ($stage_name:literal, $side:ident) {
            $(
            $name:ident ($id:literal) {
                $(
                $field_name:ident: $field_ty:ident $(<$generic:ident>)?
                ),* $(,)?
            }
            );* $(;)?
        }
    ) => {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        #[serde(untagged)]
        pub enum $stage {
            $(
            $name($name)
            ),*
        }

        impl $crate::net_io::packet::PacketStage for $stage {
            fn name() -> String {
                $stage_name.to_string()
            }

            fn side() -> $crate::net_io::packet::ProtocolSide {
                $crate::net_io::packet::ProtocolSide::$side
            }
        }

        #[async_trait::async_trait]
        impl $crate::net_io::PacketRead for $stage {
            async fn pack_read(buffer: &mut std::io::Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
                let id = $crate::net_io::VarInt::pack_read(buffer, target_version).await?.0;
                Ok(match id {
                    $(
                    $id => $stage::$name(<$name>::pack_read(buffer, target_version).await?),
                    )*
                    _ => {
                        use $crate::net_io::packet::PacketStage;
                        log::warn!("Received invalid packet type: {:#01x} in stage {}", id, $stage::name());
                        anyhow::bail!("Received invalid packet type: {:#01x} in stage {}", id, $stage::name());
                    }
                })
            }
        }

        #[async_trait::async_trait]
        impl $crate::net_io::PacketWrite for $stage {
            async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
                use $stage::*;
                match self {
                    $(
                    $name(v) => v.pack_write(buffer, target_version).await?
                    ),*
                };

                Ok(())
            }
        }

        $(
            #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
            pub struct $name {
                $(
                    $field_name: $crate::simplify!($field_ty $(<$generic>)?)
                ),*
            }

            impl $name {
                pub fn new($($field_name: $crate::simplify!($field_ty $(<$generic>)?)),*) -> Self {
                    Self {
                        $(
                        $field_name
                        ),*
                    }
                }

                $(
                pub fn $field_name(&self) -> &$crate::simplify!($field_ty $(<$generic>)?) {
                    &self.$field_name
                }
                )*
            }

            impl $crate::net_io::packet::Packet<$stage> for $name {
                fn packet_id() -> u32 {
                    $id
                }

                fn side() -> $crate::net_io::packet::ProtocolSide {
                    $crate::net_io::packet::ProtocolSide::$side
                }

                fn into_stage(self) -> $stage {
                    $stage::$name(self)
                }
            }

            #[async_trait::async_trait]
            impl $crate::net_io::PacketRead for $name {
                #[allow(unused_variables)]
                async fn pack_read(buffer: &mut std::io::Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
                    $(
                    let $field_name = <$field_ty>::pack_read(buffer, target_version).await?;
                    )*
                    Ok(Self {
                        $(
                        $field_name: $crate::storage!($field_ty, $field_name),
                        )*
                    })
                }
            }

            #[async_trait::async_trait]
            impl $crate::net_io::PacketWrite for $name {
                async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
                    $crate::net_io::VarInt($id).pack_write(buffer, target_version).await?;
                    $(
                    $crate::writeable!($field_ty$(<$generic>)?, self.$field_name).pack_write(buffer, target_version).await?;
                    )*

                    Ok(())
                }
            }
        )*
    };
}