use crate::staged_packets;

staged_packets! {
    PacketPlayOut("play", Outgoing) {
        Null(0x00) {

        }
    }
}