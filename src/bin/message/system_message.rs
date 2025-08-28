use super::message::{Message, MessageType};
use alloc::vec::Vec;
use esp_hal::timer::timg::TimerGroup;

// SystemMessage 结构体，系统报文（报文类型 0x4）为周期性，强制静态报文，用于描述无人驾驶航空器控制站位置和高度 、 航空器组群及额外的系统信息
#[derive(Debug, Clone, PartialEq)]
pub struct SystemMessage {
    // 起始字节1 (1字节)
    pub coordinate_system: u8,     // 坐标系类型 (7位)
    pub reserved_bits: u8,         // 预留位 (6-5位)
    pub classification_region: u8, // 等级分类归属区域 (4-2位)
    pub station_type: u8,          // 控制站位置类型 (1-0位)

    // 起始字节2 (4字节)
    pub latitude: i32,             // 控制站纬度 (小端序)

    // 起始字节6 (4字节)
    pub longitude: i32,             // 控制站经度 (小端序)

    // 可选字段
    pub operation_count: u16, // 运行区域计数 (小端序)
    pub operation_radius: u8, // 运行区域半径 (*10)
    pub altitude_upper: u16,  // 运行区域高度上限 (几何高度, 小端序)
    pub altitude_lower: u16,  // 运行区域高度下限 (几何高度, 小端序)

    // 起始字节17 (1字节)
    pub ua_category: u8,           // UA运行类别

    // 起始字节18 (1字节)
    pub ua_level: u8,              // UA等级

    // 起始字节19 (2字节)
    pub station_altitude: u16,     // 控制站高度 (小端序)

    // 时间戳
    pub timestamp: u32,     // 时间戳 (Unix时间, 秒)
    pub reserved: u8,       // 预留
}

impl SystemMessage {
    pub const MESSAGE_TYPE: u8 = 0x04;
    const EXPECTED_LENGTH: usize = 24;

    pub fn new(latitude: i32, longitude: i32) -> Self {
        Self { 
            coordinate_system: 0, 
            reserved_bits: 0, 
            classification_region: 2, 
            station_type: 1, 
            latitude: latitude, 
            longitude: longitude, 
            operation_count: 1, 
            operation_radius: 0, 
            altitude_upper: 0, 
            altitude_lower: 0, 
            ua_category: 0, 
            ua_level: 0, 
            station_altitude: 0, 
            timestamp: 0, 
            reserved: 0
         }
    }
}


// 实现 Message trait
impl Message for SystemMessage {

    fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        let message_type = MessageType::SystemMessageType as u8;
        let message_protocol = (message_type << 4) | 0x01;
        bytes.push(message_protocol);

        // 第1字节编码
        let mut byte1 = (self.coordinate_system & 0x7F) << 1;
        byte1 |= (self.reserved_bits & 0x03) << 5;
        byte1 |= (self.classification_region & 0x07) << 2;
        byte1 |= self.station_type & 0x03;
        bytes.push(byte1 as u8);
        
        // 经纬度编码（小端序）
        bytes.extend_from_slice(&self.latitude.to_le_bytes());
        bytes.extend_from_slice(&self.longitude.to_le_bytes());
        
        // count and radius
        bytes.extend_from_slice(&self.operation_count.to_le_bytes());
        bytes.push(self.operation_radius);
        bytes.extend_from_slice(&self.altitude_upper.to_le_bytes());
        bytes.extend_from_slice(&self.altitude_lower.to_le_bytes());
        
        // UA类别和等级
        let ua_category_level = self.ua_category << 4 | self.ua_level;
        bytes.push(ua_category_level);
        
        // 控制站高度
        bytes.extend_from_slice(&self.station_altitude.to_le_bytes());
        
        // 时间戳和预留 - 使用ESP32-C6定时器获取系统时间
        let timestamp = {
            let timg0 = unsafe { &*esp_hal::peripherals::TIMG0::ptr() };
            let t = timg0.t(0); // 访问定时器0
            
            // 触发更新以捕获当前值
            t.update().write(|w| w.update().set_bit());
            while t.update().read().update().bit_is_set() {
                // 等待更新完成
            }
            
            // 读取定时器值
            let value_lo = t.lo().read().bits() as u64;
            let value_hi = t.hi().read().bits() as u64;
            let timer_value = (value_hi << 32) | value_lo;
            timer_value as u32
        };
        bytes.extend_from_slice(&timestamp.to_le_bytes());

        bytes.push(self.reserved);
        
        bytes
    }
}
