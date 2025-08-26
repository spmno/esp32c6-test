extern crate alloc;
use alloc::vec::Vec;
// 公共消息错误类型
#[derive(Debug, PartialEq)]
pub enum MessageError {
    InsufficientLength(usize, usize),  // 期望长度, 实际长度
    InvalidUtf8(u8),        // UTF-8 格式错误
    UnknownMessageType(u8),             // 未知消息类型
}

// 公共消息类型，目前根据大疆，有3种
#[derive(Debug, PartialEq)]
pub enum MessageType {
    BaseMessageType = 0,
    PositionVectorMessageType = 1,
    SystemMessageType = 4,
}

/// 所有消息类型必须实现的 trait
pub trait Message {
    // 从结构体到字节的编码
    fn encode(&self) -> Vec<u8> ;
}