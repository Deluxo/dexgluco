pub mod jpake;
pub mod protocol;

use crate::core::{Connection, Sensor};
use crate::io::task::Task;
use bluer::{
    AdapterEvent, Address, DiscoveryFilter, DiscoveryTransport, Session,
};
use futures::StreamExt;
use std::time::Duration;

const DEXCOM_SERVICE_UUID: &str = "f8083532-849e-531c-c594-30f1f86a4ea5";

pub struct ScanForSensor(pub String);

impl ScanForSensor {
    pub fn run(self) -> Task<String> {
        Task::new(async move {
            let session = Session::new().await.map_err(|e| format!("BLE session: {}", e))?;
            let adapter = session
                .default_adapter()
                .await
                .map_err(|e| format!("BLE adapter: {}", e))?;
            adapter
                .set_powered(true)
                .await
                .map_err(|e| format!("BLE power: {}", e))?;

            let filter = DiscoveryFilter {
                transport: DiscoveryTransport::Le,
                ..Default::default()
            };
            adapter
                .set_discovery_filter(filter)
                .await
                .map_err(|e| format!("BLE filter: {}", e))?;

            let device_events = adapter
                .discover_devices()
                .await
                .map_err(|e| format!("BLE discover: {}", e))?;
            let mut device_events = std::pin::pin!(device_events);

            let start = std::time::Instant::now();
            let timeout = Duration::from_secs(30);

            while start.elapsed() < timeout {
                match tokio::time::timeout(Duration::from_secs(5), device_events.next()).await {
                    Ok(Some(AdapterEvent::DeviceAdded(addr))) => {
                        let device =
                            adapter.device(addr).map_err(|e| format!("Device: {}", e))?;

                        if let Ok(Some(name)) = device.name().await {
                            if name.contains(&self.0) {
                                return Ok(addr.to_string());
                            }
                        }

                        if let Ok(Some(uuids)) = device.uuids().await {
                            if uuids.iter().any(|u| {
                                u.to_string().contains(DEXCOM_SERVICE_UUID)
                            }) {
                                return Ok(addr.to_string());
                            }
                        }
                    }
                    Ok(Some(_)) => {}
                    Ok(None) => break,
                    Err(_) => {}
                }
            }

            Err(format!("Sensor '{}' not found via BLE", self.0))
        })
    }
}

fn dexcom_uuid() -> bluer::Uuid {
    DEXCOM_SERVICE_UUID.parse().expect("valid Dexcom UUID")
}

pub struct ConnectSensor {
    pub sensor: Sensor,
}

impl ConnectSensor {
    pub fn new(sensor: Sensor) -> Self {
        ConnectSensor { sensor }
    }

    pub fn run(self) -> Task<Connection> {
        Task::new(async move {
            let session = Session::new().await.map_err(|e| format!("BLE session: {}", e))?;
            let adapter = session
                .default_adapter()
                .await
                .map_err(|e| format!("BLE adapter: {}", e))?;

            let addr: Address = self
                .sensor
                .address
                .parse()
                .map_err(|e| format!("Invalid address '{}': {}", self.sensor.address, e))?;
            let device = adapter.device(addr).map_err(|e| format!("Device: {}", e))?;

            device
                .connect()
                .await
                .map_err(|e| format!("BLE connect: {}", e))?;

            // Wait for services to resolve
            let wait_start = std::time::Instant::now();
            let wait_timeout = Duration::from_secs(10);
            while wait_start.elapsed() < wait_timeout {
                if device
                    .is_services_resolved()
                    .await
                    .map_err(|e| format!("Services resolved check: {}", e))?
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }

            // Verify the Dexcom service is present
            let has_dexcom = device
                .uuids()
                .await
                .map_err(|e| format!("UUID query: {}", e))?
                .map(|uuids| uuids.iter().any(|u| u == &dexcom_uuid()))
                .unwrap_or(false);

            if !has_dexcom {
                return Err("Dexcom service not found on device".into());
            }

            // TODO: Perform J-PAKE authentication handshake
            // 1. Find auth characteristic (3535) on the Dexcom service
            // 2. Perform J-PAKE round 1/2 with the pairing pin
            // 3. Authenticate and subscribe to glucose data

            Ok(Connection {
                sensor: self.sensor,
                stream: vec![],
            })
        })
    }
}
