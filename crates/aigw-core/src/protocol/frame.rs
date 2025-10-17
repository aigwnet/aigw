/// A frame in the Tcp Server protocol.
#[derive(Clone, Debug)]
pub struct Frame {}

impl Frame {
    pub const HANDLESHAKE_REQ: u8 = 0x01;
    pub const HANDLESHAKE_RSP: u8 = 0x02;
    pub const HEARTBEAT_PING: u8 = 0x03;
    pub const HEARTBEAT_PONG: u8 = 0x04;
    pub const DATA: u8 = 0x05;
    pub const ACK: u8 = 0x06;
    pub const CLOSE: u8 = 0xff;
}
