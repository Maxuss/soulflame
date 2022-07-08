use crate::staged_packets;
use crate::chat::Component;

staged_packets! {
    PacketPlayOut("play", Outgoing) {
        PacketPlayOutDisconnect(0x17) {
            reason: Component
        }
    }
}
