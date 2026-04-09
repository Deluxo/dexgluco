use std::error::Error;

use dexgluco::{
    get_sensors, connect, monitor,
    Sensor, Connection, GlucoseReading, AppResult
};

fn main() -> Result<(), Box<dyn Error>> {
    let get_from_storage = || {
        Ok(vec![])
    };

    let connect_new = || {
        println!("User: scanning QR code from sensor packaging...");
        let serial = "DXCM123456".to_string();
        let pin = "123456".to_string();
        Ok(Sensor {
            serial,
            pin,
            address: "00:11:22:33:44:55".to_string(),
        })
    };

    let via_bt = |sensor: Sensor| -> AppResult<Connection> {
        println!("Connecting to sensor: {} at {}", sensor.serial, sensor.address);
        Ok(Connection {
            sensor,
            stream: vec![
                GlucoseReading {
                    value: 142.0,
                    timestamp: 1700000000,
                    trend: 3,
                },
            ],
        })
    };

    let sensors = get_sensors(get_from_storage, connect_new)?;
    println!("Got {} sensor(s)", sensors.len());

    let connections = connect(via_bt, sensors)?;
    println!("Connected to {} sensor(s)", connections.len());

    let _ = monitor(connections);

    Ok(())
}