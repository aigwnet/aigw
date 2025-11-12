use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct IpUpdate {
    pub start_time: i64,
    pub end_time: i64,
    pub prefix_len: u32,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IpDelete {
    pub prefix_len: u32,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IpUpdateList {
    pub item_type: u32, // 1 whtie, 2 block
    pub data: Vec<IpUpdate>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IpDeleteList {
    pub item_type: u32, // 1 whtie, 2 block
    pub data: Vec<IpDelete>,
}
