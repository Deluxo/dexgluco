use dexgluco::io::qr::ScanDataMatrix;
use dexgluco::io::storage::{LoadSensors, SaveSensor};
use dexgluco::core::Sensor;

#[tokio::test]
async fn test_scan_real_sensor_datamatrix() {
    let result: Result<(String, String), String> =
        ScanDataMatrix("tests/data/sensor-qr.jpg".into()).run().await;

    match &result {
        Ok((serial, pin)) => {
            println!("=== DECODED DATA ===");
            println!("Serial: {}", serial);
            println!("PIN:    {}", pin);
            println!("=== END ===");

            assert!(!serial.is_empty(), "Serial should not be empty");
            assert!(!pin.is_empty(), "PIN should not be empty");

            assert_eq!(
                pin, "6044",
                "Expected pairing code 6044. Got: {}",
                pin
            );

            assert_eq!(
                serial, "667529744201",
                "Expected serial 667529744201. Got: {}",
                serial
            );
        }
        Err(e) => {
            panic!("Should decode the DataMatrix: {}", e);
        }
    }
}

#[tokio::test]
async fn test_storage_save_and_load() {
    let path = format!("/tmp/dexgluco-test-{}.db", std::process::id());

    let sensor = Sensor {
        serial: "TEST123".into(),
        pin: "6044".into(),
        address: "AA:BB:CC:DD:EE:FF".into(),
    };

    SaveSensor::new(&path, sensor.clone())
        .run()
        .await
        .expect("save should succeed");

    let sensors = LoadSensors::new(&path)
        .run()
        .await
        .expect("load should succeed");

    assert_eq!(sensors.len(), 1);
    assert_eq!(sensors[0].serial, "TEST123");
    assert_eq!(sensors[0].pin, "6044");
    assert_eq!(sensors[0].address, "AA:BB:CC:DD:EE:FF");

    let _ = std::fs::remove_file(&path);
}

#[tokio::test]
async fn test_storage_load_empty() {
    let path = format!("/tmp/dexgluco-test-empty-{}.db", std::process::id());

    let sensors = LoadSensors::new(&path)
        .run()
        .await
        .expect("load should succeed on missing DB");

    assert!(sensors.is_empty());

    let _ = std::fs::remove_file(&path);
}
