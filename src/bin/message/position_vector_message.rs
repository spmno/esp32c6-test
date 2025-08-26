use crate::message::message::MessageType;

use super::message::{Message, MessageError};

/// 位置向量报文，强 制 动 态 报 文 ，主要包含位置 ， 高度 ， 速度 ， 时间等标识 UA 运行情况的报文 。
#[derive(Debug, Clone, PartialEq)]
pub struct PositionVectorMessage {
    // 第1字节 (运行状态和标志位)
    pub run_status: u8,         // 运行状态 (7-4位)
    #[serde(default)]
    pub reserved_flag: bool,     // 预留标志位 (3位)
    pub height_type: u8,        // 高度类型位 (2位) - 0-3
    pub track_direction: u8,   // 航迹角 E/W 方向标志 (1位)
    pub speed_multiplier: u8,  // 速度乘数 (0位)

    // 第2-4字节
    pub track_angle: u8,        // 航迹角 (1字节)
    pub ground_speed: i8,       // 地速 (1字节, 有正负)
    pub vertical_speed: i8,     // 垂直速度 (1字节, 有正负, 可选)

    // 第5-18字节
    pub latitude: i32,           // 纬度 (4字节小端序)
    pub longitude: i32,          // 经度 (4字节小端序)
    pub pressure_altitude: i16, // 气压高度 (2字节小端序, 可选)
    pub geometric_altitude: i16, // 几何高度 (2字节小端序, 可选)
    pub ground_altitude: i16,    // 距地高度 (2字节小端序)

    // 第19-22字节
    pub vertical_accuracy: u8,   // 垂直精度 (7-4位, 4 bits)
    pub horizontal_accuracy: u8, // 水平精度 (3-0位, 4 bits)
    pub speed_accuracy: u8,      // 速度精度 (3-0位, 4 bits)
    pub timestamp: u16,          // 时间戳 (2字节小端序)

    // 第23-24字节
    #[serde(default)]
    pub timestamp_accuracy: u8, // 时间戳精度 (3-0位, 4 bits)
    #[serde(default)]
    pub reserved: u8,           // 预留 (1字节)
}

impl PositionVectorMessage {
    pub const MESSAGE_TYPE: u8 = 0x01;
    const EXPECTED_LENGTH: usize = 24;

    fn calculate_full_track_angle(&self) -> u16 {
        if self.track_direction == 1 {
            self.track_angle as u16 + 180
        } else {
            self.track_angle as u16
        }
    }
    
    fn calculate_ground_speed_knots(&self) -> f32 {
        if self.speed_multiplier == 1{
            self.ground_speed as f32 * 10.0
        } else {
            self.ground_speed as f32
        }
    }

}

impl Message for PositionVectorMessage {

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        let message_type = MessageType::PositionVectorMessageType as u8;
        let message_protocol = (message_type << 4) | 0x01;
        bytes.push(message_protocol);
        // 第1字节编码
        let mut byte1 = (self.run_status << 4) as u8;
        byte1 |= (self.reserved_flag as u8) << 3;
        byte1 |= (self.height_type & 0x03) << 2;
        byte1 |= self.track_direction << 1;
        byte1 |= self.speed_multiplier;
        bytes.push(byte1);
        
        // 第2-4字节
        bytes.push(self.track_angle);
        bytes.push(self.ground_speed as u8);
        bytes.push(self.vertical_speed as u8);
        
        // 经纬度编码（小端序）
        bytes.extend_from_slice(&self.latitude.to_le_bytes());
        bytes.extend_from_slice(&self.longitude.to_le_bytes());
        
        // 高度字段
        bytes.extend_from_slice(&self.pressure_altitude.to_le_bytes());
        bytes.extend_from_slice(&self.geometric_altitude.to_le_bytes());
        bytes.extend_from_slice(&self.ground_altitude.to_le_bytes());
        
        // 精度和时间戳
        let accuracy_byte = (self.vertical_accuracy << 4) | (self.horizontal_accuracy & 0x0F);
        bytes.push(accuracy_byte);
        bytes.push(self.speed_accuracy & 0x0F);
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        
        // 最后2字节
        bytes.push((self.timestamp_accuracy << 4) | (self.reserved & 0x0F));
        bytes.push(self.reserved);
        
        bytes
    }

}
