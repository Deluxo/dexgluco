pub mod jpake;
pub mod protocol;
pub mod certs;

use crate::core::{Connection, Sensor, GlucoseReading};
use crate::io::task::Task;
use bluer::{
    AdapterEvent, Address, DiscoveryFilter, DiscoveryTransport, Session,
};
use futures::StreamExt;
use std::time::Duration;

use self::protocol::BleSession;

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

pub struct ConnectSensor {
    pub sensor: Sensor,
    pub shared_key: Option<[u8; 16]>,
}

impl ConnectSensor {
    pub fn new(sensor: Sensor) -> Self {
        ConnectSensor {
            sensor,
            shared_key: None,
        }
    }

    pub fn with_shared_key(sensor: Sensor, shared_key: [u8; 16]) -> Self {
        ConnectSensor {
            sensor,
            shared_key: Some(shared_key),
        }
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

            let pin_bytes: [u8; 4] = {
                let b = self.sensor.pin.as_bytes();
                let mut p = [0u8; 4];
                let len = b.len().min(4);
                p[..len].copy_from_slice(&b[..len]);
                p
            };

            let mut ble_session = BleSession::new(
                device,
                &pin_bytes,
                self.shared_key.as_ref(),
            );

            let _shared_key = ble_session.authenticate().await?;

            Ok(Connection {
                sensor: self.sensor,
                stream: vec![],
            })
        })
    }
}

pub struct MonitorSensor {
    pub sensor: Sensor,
}

impl MonitorSensor {
    pub fn new(sensor: Sensor) -> Self {
        MonitorSensor { sensor }
    }

    pub fn run<F>(self, mut on_reading: F) -> Task<()>
    where
        F: FnMut(GlucoseReading) + Send + 'static,
    {
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

            let pin_bytes: [u8; 4] = {
                let b = self.sensor.pin.as_bytes();
                let mut p = [0u8; 4];
                let len = b.len().min(4);
                p[..len].copy_from_slice(&b[..len]);
                p
            };

            let mut ble_session = BleSession::new(
                device,
                &pin_bytes,
                self.sensor.shared_key.as_ref(),
            );

            ble_session.authenticate().await?;

            let mut control_stream = ble_session
                .take_control_stream()
                .ok_or("No control stream available after auth".to_string())?;

            loop {
                tokio::select! {
                    Some(data) = control_stream.next() => {
                        if data.len() >= 19 && data[0] == 0x4E {
                            match protocol::parse_egv(&data) {
                                Ok(reading) => {
                                    on_reading(reading);
                                }
                                Err(e) => {
                                    eprintln!("Parse error: {}", e);
                                }
                            }
                        }
                    }
                    else => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        })
    }
}
