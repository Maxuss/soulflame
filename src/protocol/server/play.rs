use crate::chat::Component;
use crate::staged_packets;

staged_packets! {
    PacketPlayOut("play", Outgoing) {
        PacketPlayOutDisconnect(0x17) {
            reason: Component
        }
    }
}
