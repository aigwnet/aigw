use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct IpItem {
    pub prefix_len: u32,
    pub data: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IpList {
    pub item_type: u32, // 1 whtie, 2 block
    pub data: Vec<IpItem>,
}
