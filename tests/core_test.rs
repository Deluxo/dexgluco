use dexgluco::core::{get_sensors, connect, monitor, Sensor, Connection, AppResult};

#[test]
fn test_get_sensors_returns_stored_when_available() {
    let stored_sensor = Sensor {
        serial: "STORED123".to_string(),
        pin: "111111".to_string(),
        address: "AA:BB:CC:DD:EE:FF".to_string(),
    };

    let get_from_storage = || Ok(vec![stored_sensor.clone()]);
    let connect_new = || panic!("should not be called");

    let result = get_sensors(get_from_storage, connect_new).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].serial, "STORED123");
}

#[test]
fn test_get_sensors_calls_connect_new_when_storage_empty() {
    let get_from_storage = || Ok(vec![]);
    let connect_new = || {
        Ok(Sensor {
            serial: "NEW456".to_string(),
            pin: "222222".to_string(),
            address: "11:22:33:44:55:66".to_string(),
        })
    };

    let result = get_sensors(get_from_storage, connect_new).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(result[0].serial, "NEW456");
}

#[test]
fn test_get_sensors_returns_error_from_storage() {
    let get_from_storage = || Err("DB read failed".to_string());
    let connect_new = || panic!("should not be called");

    let result = get_sensors(get_from_storage, connect_new);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "DB read failed");
}

#[test]
fn test_connect_calls_via_bt_for_each_sensor() {
    let sensors = vec![
        Sensor { serial: "A".to_string(), pin: "1".to_string(), address: "A1".to_string() },
        Sensor { serial: "B".to_string(), pin: "2".to_string(), address: "B2".to_string() },
    ];

    let mut call_count = 0;
    let via_bt = |sensor: Sensor| -> AppResult<Connection> {
        call_count += 1;
        Ok(Connection {
            sensor,
            stream: vec![],
        })
    };

    let result = connect(via_bt, sensors).unwrap();

    assert_eq!(call_count, 2);
    assert_eq!(result.len(), 2);
}

#[test]
fn test_connect_returns_error_when_via_bt_fails() {
    let sensors = vec![
        Sensor { serial: "A".to_string(), pin: "1".to_string(), address: "A1".to_string() },
    ];

    let via_bt = |_: Sensor| -> AppResult<Connection> {
        Err("BLE connection failed".to_string())
    };

    let result = connect(via_bt, sensors);

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "BLE connection failed");
}

#[test]
fn test_connect_handles_empty_sensor_list() {
    let via_bt = |_: Sensor| -> AppResult<Connection> {
        panic!("should not be called");
    };

    let result = connect(via_bt, vec![]).unwrap();

    assert!(result.is_empty());
}

#[test]
#[ignore = "monitor runs indefinitely"]
fn test_monitor_runs_indefinitely() {
    let connections = vec![
        Connection {
            sensor: Sensor { serial: "X".to_string(), pin: "1".to_string(), address: "X1".to_string() },
            stream: vec![],
        }
    ];

    let _ = monitor(connections);
}
