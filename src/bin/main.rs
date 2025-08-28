#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

pub mod message;
use core::marker::PhantomData;

use esp_hal::clock::CpuClock;
use esp_hal::main;
use esp_hal::delay::Delay;
use log::{info, error};
use esp_alloc as _;
extern crate alloc;

use esp_hal::timer::timg::TimerGroup;
use esp_hal::rng::Rng;
use alloc::vec::Vec;
use alloc::format;
use core::convert::TryInto;


use ieee80211::{
    common::{CapabilitiesInformation, FCFFlags},
    element_chain,
    elements::{DSSSParameterSetElement, RawIEEE80211Element, SSIDElement},
    mgmt_frame::{BeaconFrame, body::BeaconBody, ManagementFrameHeader},
    scroll::Pwrite,
    supported_rates,
};

use crate::message::{message::Message, packet_message::PacketMessage};


#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

const MAC_ADDRESS: [u8; 6] = [0x00, 0x80, 0x41, 0x13, 0x37, 0x42];

#[main]
fn main() -> ! {
    esp_alloc::heap_allocator!(size: 72 * 1024);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);
    esp_println::logger::init_logger(log::LevelFilter::Info);
    
    info!("Drone RID Beacon Transmitter Starting...");
    
    // Initialize WiFi
    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let init = esp_wifi::init(
        timg0.timer0,
        Rng::new(peripherals.RNG),
    )
    .unwrap();
    
    let wifi = peripherals.WIFI;
    let (mut controller, interfaces) = esp_wifi::wifi::new(&init, wifi).unwrap();
    match controller.set_mode(esp_wifi::wifi::WifiMode::ApSta) {
        Ok(_) => {
            info!("WiFi controller set mode successfully");
        }
        Err(e) => {
            error!("Failed to set WiFi mode: {:?}", e);
            loop {} // Halt on startup failure
        }
    }
    
    // Configure STA settings
    let sta_config = esp_wifi::wifi::Configuration::Client(esp_wifi::wifi::ClientConfiguration {
        ssid: "RID-DRONE123456789".try_into().unwrap(),
        channel: Some(6),
        ..Default::default()
    });
    
    match controller.set_configuration(&sta_config) {
        Ok(_) => {
            info!("STA configuration set successfully (channel 6)");
        }
        Err(e) => {
            error!("Failed to set STA configuration: {:?}", e);
            loop {} // Halt on startup failure
        }
    }
    
        
    // Use the sniffer interface for raw frame transmission
    let mut wifi_device = interfaces.sniffer;
        info!("MAC Address: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
          MAC_ADDRESS[0], MAC_ADDRESS[1], MAC_ADDRESS[2], 
          MAC_ADDRESS[3], MAC_ADDRESS[4], MAC_ADDRESS[5]);
    
    let delay = Delay::new();
    let mut counter = 0;
    
    // Start WiFi for raw frame transmission
    info!("Starting WiFi controller...");
    match controller.start() {
        Ok(_) => {
            info!("WiFi controller started successfully");
        }
        Err(e) => {
            error!("Failed to start WiFi controller: {:?}", e);
            loop {} // Halt on startup failure
        }
    }
    unsafe {
        let result = esp_wifi_sys::include::esp_wifi_set_channel(6,  0);
        info!("set channel result {:x}", result);
    };
    //--------------------test data --------------------------------//
    let package = PacketMessage::build_rid_package();
    let rid_data = package.encode();
    let mut rid_element = Vec::new();
    rid_element.extend_from_slice(&[0xfa, 0x0b, 0xbc]); // OUI
    rid_element.push(0x0d); // OUI type
    rid_element.extend_from_slice(rid_data.as_slice());
    let rid_slice = rid_element.as_slice();
    
    let mut beacon = [0u8; 300];
    let length = beacon
        .pwrite(
            BeaconFrame {
                header: ManagementFrameHeader {
                    fcf_flags: FCFFlags::new(),
                    duration: 0,
                    receiver_address: [0xff; 6].into(),
                    transmitter_address: MAC_ADDRESS.into(),
                    bssid: MAC_ADDRESS.into(),
                    ..Default::default()
                },
                body: BeaconBody {
                    timestamp: 0,
                    // We transmit a beacon every 100 ms/TUs
                    beacon_interval: 1000,
                    capabilities_info: CapabilitiesInformation::new().with_is_ess(true),
                    elements: element_chain! {
                        SSIDElement::new(package.get_ssid()).unwrap(),
                        // These are known good values.
                        supported_rates![
                            1 B
                        ],
                        DSSSParameterSetElement {
                            current_channel: 6,
                        },
                        // RID data element (vendor-specific)
                        RawIEEE80211Element {
                            tlv_type: 221, // Vendor-specific element
                            slice: rid_slice,
                            _phantom: PhantomData
                        }
                    },
                    _phantom: PhantomData,
                },
            },
            0,
        )
        .unwrap();
    // Only use the actually written bytes.
    let beacon = &beacon[..length];
    
    // Main beacon transmission loop
    info!("Entering main transmission loop");
    loop {
        counter += 1;
        
        if counter % 100 == 0 {
            info!("Transmitted {} beacon frames", counter);
        }
        
        // Send raw beacon frame using sniffer mode
        match wifi_device.send_raw_frame(true, &beacon, false) {
            Ok(_) => {
                // Successfully sent beacon frame
                info!("send success.");
            }
            Err(e) => {
                error!("Failed to send beacon frame: {:?}", e);
            }
        }
        
        delay.delay_millis(500); // 100ms interval
    }
}

