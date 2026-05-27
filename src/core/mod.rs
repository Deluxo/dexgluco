pub mod types;

pub use types::{Sensor, Connection, GlucoseReading};

use crate::io::Task;

pub type AppError = String;
pub type AppResult<T> = Result<T, AppError>;

pub fn get_sensors(
    get_from_storage: impl Fn() -> Task<Vec<Sensor>> + Send + 'static,
    connect_new: impl Fn() -> Task<Sensor> + Send + 'static,
) -> Task<Vec<Sensor>> {
    get_from_storage().and_then(move |stored| {
        if !stored.is_empty() {
            Task::from_value(stored)
        } else {
            connect_new().map(|s| vec![s])
        }
    })
}

pub fn connect(
    mut via_bt: impl FnMut(Sensor) -> Task<Connection> + Send + 'static,
    sensors: Vec<Sensor>,
) -> Task<Vec<Connection>> {
    Task::new(async move {
        let mut connections = Vec::with_capacity(sensors.len());
        for sensor in sensors {
            connections.push(via_bt(sensor).await?);
        }
        Ok(connections)
    })
}

pub fn monitor(
    sensors: Vec<Sensor>,
    run_sensor: impl Fn(Sensor) -> Task<()> + Send + 'static,
) -> Task<()> {
    Task::new(async move {
        let tasks: Vec<_> = sensors.into_iter()
            .map(|s| run_sensor(s).run())
            .collect();
        futures::future::join_all(tasks).await;
        Ok(())
    })
}
