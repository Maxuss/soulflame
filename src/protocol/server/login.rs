use crate::{packet_struct, staged_packets};
use crate::net_io::{ByteArray, VarInt};
use crate::chat::Component;
use uuid::Uuid;
use crate::util::Identifier;

packet_struct! {
    ProfileProperty {
        name: String,
        value: String,
        signature: Option<String>
    }
}

staged_packets! {
    OutLogin("login", Outgoing) {
        PacketLoginOutDisconnect(0x00) {
            reason: Component
        };

        PacketLoginOutEncryptionRequest(0x01) {
            server_id: String,
            public_key: Vec<u8>,
            verify_token: Vec<u8>
        };

        PacketLoginOutSuccess(0x02) {
            player_uuid: Uuid,
            username: String,
            properties: Vec<ProfileProperty>
        };

        PacketLoginOutCompression(0x03) {
            threshold: VarInt
        };

        PacketLoginOutPluginMessage(0x04) {
            message_id: VarInt,
            channel: Identifier,
            message: ByteArray
        }
    }
}