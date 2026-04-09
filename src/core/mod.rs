pub mod types;

pub use types::{Sensor, Connection, GlucoseReading};

pub type AppError = String;
pub type AppResult<T> = Result<T, AppError>;

pub fn get_sensors(
    get_from_storage: impl Fn() -> AppResult<Vec<Sensor>>,
    connect_new: impl Fn() -> AppResult<Sensor>,
) -> AppResult<Vec<Sensor>> {
    let stored = get_from_storage()?;

    if !stored.is_empty() {
        return Ok(stored);
    }

    let new_sensor = connect_new()?;
    Ok(vec![new_sensor])
}

pub fn connect(
    mut via_bt: impl FnMut(Sensor) -> AppResult<Connection>,
    sensors: Vec<Sensor>,
) -> AppResult<Vec<Connection>> {
    let mut connections = Vec::new();

    for sensor in sensors {
        let conn = via_bt(sensor)?;
        connections.push(conn);
    }

    Ok(connections)
}

pub fn monitor(_connections: Vec<Connection>) -> AppResult<()> {
    println!("Monitoring started - press Ctrl+C to stop");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}