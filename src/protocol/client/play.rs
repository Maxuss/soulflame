use crate::staged_packets;

staged_packets! {
    PacketPlayIn("play", Inbound) {
        Null(0x00) {

        }
    }
}
