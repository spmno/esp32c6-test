#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

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
    match controller.set_mode(esp_wifi::wifi::WifiMode::Ap) {
        Ok(_) => {
            info!("WiFi controller set mode successfully");
        }
        Err(e) => {
            error!("Failed to set WiFi mode: {:?}", e);
            loop {} // Halt on startup failure
        }
    }
    
    // Configure AP settings
    let ap_config = esp_wifi::wifi::Configuration::AccessPoint(esp_wifi::wifi::AccessPointConfiguration {
        ssid: "RID-DRONE123456789".try_into().unwrap(),
        channel: 6,
        secondary_channel: None,
        ..Default::default()
    });
    
    match controller.set_configuration(&ap_config) {
        Ok(_) => {
            info!("AP configuration set successfully (channel 6)");
        }
        Err(e) => {
            error!("Failed to set AP configuration: {:?}", e);
            loop {} // Halt on startup failure
        }
    }
    
        
    // Use the sniffer interface for raw frame transmission
    let mut wifi_device = interfaces.sniffer;
    
    // Create simple drone RID data
    let drone_id = b"DRONE-123456789";
    let ssid = format!("RID-{}", core::str::from_utf8(drone_id).unwrap());
    
    // Simple RID payload
    let rid_data = b"RID:DRONE123456789,LAT:39.9042,LON:116.4074,ALT:100";
    
    // Create beacon frame
    let beacon_frame = create_rid_beacon(&ssid,
        &MAC_ADDRESS,
        rid_data,
    );
    
    info!("Drone RID beacon transmitting on SSID: {}", ssid);
    info!("MAC Address: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}", 
          MAC_ADDRESS[0], MAC_ADDRESS[1], MAC_ADDRESS[2], 
          MAC_ADDRESS[3], MAC_ADDRESS[4], MAC_ADDRESS[5]);
    info!("Frame size: {} bytes", beacon_frame.len());
    
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

    //--------------------test data --------------------------------//
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
                        SSIDElement::new(ssid).unwrap(),
                        // These are known good values.
                        supported_rates![
                            1 B,
                            2 B,
                            5.5 B,
                            11 B,
                            6,
                            9,
                            12,
                            18
                        ],
                        DSSSParameterSetElement {
                            current_channel: 6,
                        },
                        // This contains the Traffic Indication Map(TIM), for which `ieee80211-rs` currently lacks support.
                        RawIEEE80211Element {
                            tlv_type: 5,
                            slice: [0x01, 0x02, 0x00, 0x00].as_slice(),
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
        
        // Create new beacon frame
        //let updated_beacon = create_rid_beacon(
        //    &ssid,
        //    &MAC_ADDRESS,
        //    rid_data,
        //);
        
        // Send raw beacon frame using sniffer mode
        match wifi_device.send_raw_frame(false, &beacon, false) {
            Ok(_) => {
                // Successfully sent beacon frame
                info!("send success.");
            }
            Err(e) => {
                error!("Failed to send beacon frame: {:?}", e);
            }
        }
        
        delay.delay_millis(1000); // 100ms interval
    }
}

fn create_rid_beacon(ssid: &str, bssid: &[u8; 6], rid_data: &[u8]) -> Vec<u8> {
    let mut frame = Vec::new();
    
    // Frame Control (2 bytes) - Beacon frame
    frame.extend_from_slice(&[0x80, 0x00]);
    
    // Duration (2 bytes)
    frame.extend_from_slice(&[0x00, 0x00]);
    
    // Destination Address (broadcast) (6 bytes)
    frame.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
    
    // Source Address (6 bytes)
    frame.extend_from_slice(bssid);
    
    // BSSID (6 bytes)
    frame.extend_from_slice(bssid);
    
    // Sequence Control (2 bytes)
    frame.extend_from_slice(&[0x00, 0x00]);
    
    // Beacon Frame Body
    // Timestamp (8 bytes) - will be updated by hardware
    frame.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    
    // Beacon Interval (2 bytes) - 100 TUs = 100ms
    frame.extend_from_slice(&[0x64, 0x00]);
    
    // Capability Information (2 bytes)
    frame.extend_from_slice(&[0x01, 0x04]); // Infrastructure mode, open network
    
    // SSID Element
    let ssid_bytes = ssid.as_bytes();
    let ssid_len = ssid_bytes.len().min(32);
    frame.push(0x00); // SSID element ID
    frame.push(ssid_len as u8);
    frame.extend_from_slice(&ssid_bytes[..ssid_len]);
    
    // Supported Rates Element
    frame.push(0x01); // Rates element ID
    frame.push(0x08); // Length
    frame.extend_from_slice(&[0x82, 0x84, 0x8b, 0x96, 0x0c, 0x12, 0x18, 0x24]);
    
    // DSS Parameter Set Element
    frame.push(0x03); // DSS element ID
    frame.push(0x01); // Length
    frame.push(0x06); // Channel 6
    
    // TIM Element
    frame.push(0x05); // TIM element ID
    frame.push(0x04); // Length
    frame.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]);
    
    // Vendor-Specific Element with RID data
    if !rid_data.is_empty() {
        let data_len = rid_data.len().min(255 - 6); // Leave room for OUI and type
        frame.push(0xdd); // Vendor-specific element ID
        frame.push((data_len + 6) as u8); // Length (OUI + type + data)
        frame.extend_from_slice(&[0xfa, 0x0b, 0xbc]); // Example OUI
        frame.push(0x0d); // OUI type
        frame.extend_from_slice(&rid_data[..data_len]);
    }
    
    frame
}