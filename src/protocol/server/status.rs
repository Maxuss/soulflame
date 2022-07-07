use crate::chat::Component;
use crate::{json_packet_struct, staged_packets};
use uuid::Uuid;

json_packet_struct! {
    ServerVersion {
        name: String,
        protocol: i32
    };

    PlayerSample {
        name: String,
        id: Uuid
    };

    ServerPlayers {
        max: i32,
        online: i32,
        sample: Vec<PlayerSample>
    };

    StatusResponse {
        version: ServerVersion,
        players: ServerPlayers,
        description: Component,
        favicon: String,
    };
}

staged_packets! {
    OutStatus ("status", Outgoing) {
        PacketStatusOutResponse(0x00) {
            response: StatusResponse
        };

        PacketStatusOutPong(0x01) {
            payload: i64
        }
    }
}
