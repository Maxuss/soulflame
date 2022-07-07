use crate::net_io::{ByteArray, VarInt};
use crate::staged_packets;
pub type ByteVec = Vec<u8>;

staged_packets! {
    InLogin("login", Inbound) {
        PacketLoginInStart(0x00) {
            name: String,
            key_expiration: Option<i64>,
            public_key: Option<ByteVec>,
            signature: Option<ByteVec>
        };

        PacketLoginInEncryptionResponse(0x01) {
            shared_secret: Vec<u8>,
            verify_token: Option<ByteVec>,
            salt: Option<i64>,
            message_signature: Option<ByteVec>
        };

        PacketLoginInPluginResponse(0x02) {
            message_id: VarInt,
            data: Option<ByteArray>
        };
    }
}
