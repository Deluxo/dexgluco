use dexgluco::{
    get_sensors, connect, monitor,
    Sensor, Connection, GlucoseReading,
    io::Task,
};

fn main() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Runtime: {}", e))?;

    let get_from_storage = || {
        Task::from_value(vec![])
    };

    let connect_new = || {
        println!("User: scanning QR code from sensor packaging...");
        let serial = "DXCM123456".to_string();
        let pin = "123456".to_string();
        Task::from_value(Sensor {
            serial,
            pin,
            address: "00:11:22:33:44:55".to_string(),
        })
    };

    let via_bt = |sensor: Sensor| -> Task<Connection> {
        let serial = sensor.serial.clone();
        let address = sensor.address.clone();
        println!("Connecting to sensor: {} at {}", serial, address);
        Task::from_value(Connection {
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

    rt.block_on(async move {
        get_sensors(get_from_storage, connect_new)
            .map(|sensors| {
                println!("Got {} sensor(s)", sensors.len());
                sensors
            })
            .and_then(move |sensors| connect(via_bt, sensors))
            .map(|connections| {
                println!("Connected {} sensor(s)", connections.len());
                connections
            })
            .and_then(monitor)
            .run()
            .await
    })
}
