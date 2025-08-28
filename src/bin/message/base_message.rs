
use crate::message::message::MessageType;
extern crate alloc;
use super::message::Message;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;

/// 基本类型，主要包含了RID的字符串
#[derive(Debug, Clone, PartialEq)]
pub struct BaseMessage {
    pub id_type: u8 ,          // 高位 4 位 (7-4 位)
    pub ua_type: u8,          // 低位 4 位 (3-0 位)
    pub uas_id: String,       // UAS 识别身份信息（字符串类型）
    pub reserved: [u8; 3],    // 3 字节预留空间
}

impl BaseMessage {
    pub const MESSAGE_TYPE: u8 = 0x00;
    const EXPECTED_LENGTH: usize = 24;

    pub fn new(uas_id: &str) -> Self {
        Self { id_type: 1, ua_type: 1, uas_id: uas_id.to_string(), reserved: [0, 0, 0] }
    }
}

impl Message for BaseMessage {

    fn encode(&self) -> Vec<u8> {
        let mut bytes:Vec<u8> = Vec::new();
        
        let message_type = MessageType::BaseMessageType as u8;
        let message_protocol = (message_type << 4) | 0x01;
        bytes.push(message_protocol);
        // 编码第一个字节：id_type（高4位） + ua_type（低4位）
        let type_byte = (self.id_type << 4) | (self.ua_type & 0x0F);
        bytes.push(type_byte);
        
        // 编码UAS ID（最多20字节）
        let uas_bytes = self.uas_id.as_bytes().to_vec();
        bytes.extend_from_slice(&uas_bytes);
        
        //不足的位置写0
        let id_len = uas_bytes.len();
        let reserved = vec![0u8; 23-id_len];
        bytes.extend(&reserved);
        
        bytes
    }

}


