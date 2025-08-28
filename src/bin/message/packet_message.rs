use crate::message::{base_message::BaseMessage, position_vector_message::PositionVectorMessage, system_message::SystemMessage};
use super::message::Message;
use core::sync::atomic::AtomicU8;
use alloc::vec::Vec;
use alloc::format;
use alloc::string::String;
use core::sync::atomic::Ordering;

static RID_COUNTER: AtomicU8 = AtomicU8::new(1);

/// 以整包形式发送，其中包含了BaseMessage， SystemMessage, PositionVectorMessage，主要模仿收到大疆的结构类型
#[derive(Debug)]
pub struct PacketMessage {
    protocol_version: u8,          // 协议版本（1字节）
    message_counter: u8,          // 消息计数器（2字节）
    message_size: u8,             // 消息总大小（2字节）
    message_quantity: u8,          // 包含消息数量（1字节）
    base_message: BaseMessage,
    system_message: SystemMessage,
    position_message: PositionVectorMessage,
    checksum: u16,                 // CRC16校验和（2字节）
    reserved: [u8; 3],             // 3字节预留
}

impl PacketMessage {
    // 每一帧的大小
    const MESSAGE_SIZE:u8 = 25;
    // 每包一共3帧
    const MESSAGE_QUANTITY:u8 = 3;
    pub fn new(
        base: BaseMessage,
        system: SystemMessage,
        position: PositionVectorMessage
    ) -> Self {
        Self {
            protocol_version: 0xf1,
            message_counter: 3,
            message_size: Self::MESSAGE_SIZE,
            message_quantity: Self::MESSAGE_QUANTITY,
            base_message: base,
            system_message: system,
            position_message: position,
            checksum: 0,
            reserved: [0; 3],
        }
    }
    // 获取rid加前缀为ssid，仿大疆
    pub fn get_ssid(&self) -> String {
        return format!("RID-{}", self.base_message.uas_id.clone());
    }

    pub fn build_rid_package() -> Self {
        let fake_latitude = 1234844601;
        let fake_longitude = 417144677;
        let base = BaseMessage::new("1581F7FVC251A00CQ211");
        let system = SystemMessage::new(fake_latitude, fake_longitude);
        let position = PositionVectorMessage::new(fake_latitude, fake_longitude);
        let package = Self::new(base, system, position);
        package
    }
}

impl Message for PacketMessage {

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // 编码头部
        let rid_counter: u8 = RID_COUNTER.fetch_add(0x01, Ordering::SeqCst); // 序列号按802.11规范递增

        bytes.push(rid_counter);
        bytes.push(0xf1);
        
        bytes.push(0x19);
        bytes.push(self.message_quantity);
        
        // 编码子消息
        bytes.extend(self.base_message.encode());
        bytes.extend(self.position_message.encode());
        bytes.extend(self.system_message.encode());

        
        // 计算校验和
        let checksum = crc16::State::<crc16::XMODEM>::calculate(&bytes);
        bytes.extend_from_slice(&checksum.to_le_bytes());
        
        // 添加预留字段
        bytes.extend_from_slice(&self.reserved);
        
        bytes
    }
}
