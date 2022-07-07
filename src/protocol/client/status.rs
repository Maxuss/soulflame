use crate::staged_packets;

staged_packets! {
    InStatus ("status", Inbound) {
        PacketStatusInRequest(0x00) {

        };

        PacketStatusInPing(0x01) {
            payload: i64
        };
    }
}
