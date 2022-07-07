use crate::{define_enum, staged_packets};
use crate::net_io::VarInt;

define_enum! {
    HandshakeState {
        Status = 1,
        Login = 2
    }
}

staged_packets! {
    InHandshake ("handshake", Inbound) {
        PacketHandshakeIn(0x00) {
            protocol_version: VarInt,
            server_address: String,
            server_port: u16,
            next_state: HandshakeState
        }
    }
}