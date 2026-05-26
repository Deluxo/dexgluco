use crate::core::Sensor;
use crate::io::task::Task;
use rusqlite::Connection;
use std::sync::Arc;

fn open_db(path: &str) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(|e| format!("DB open: {}", e))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sensors (
            serial TEXT PRIMARY KEY,
            pin TEXT NOT NULL,
            address TEXT NOT NULL,
            shared_key BLOB
        );",
    )
    .map_err(|e| format!("DB init: {}", e))?;
    Ok(conn)
}

pub struct LoadSensors {
    pub path: Arc<String>,
}

impl LoadSensors {
    pub fn new(path: impl Into<String>) -> Self {
        LoadSensors {
            path: Arc::new(path.into()),
        }
    }

    pub fn run(self) -> Task<Vec<Sensor>> {
        Task::new(async move {
            let path = self.path.clone();
            tokio::task::spawn_blocking(move || -> Result<Vec<Sensor>, String> {
                let conn = open_db(&path)?;
                let mut stmt =
                    conn.prepare("SELECT serial, pin, address, shared_key FROM sensors")
                        .map_err(|e| format!("DB query: {}", e))?;
                let rows = stmt
                    .query_map([], |row| {
                        let shared_key_blob: Option<Vec<u8>> = row.get(3).ok();
                        let shared_key = shared_key_blob.and_then(|v| {
                            let mut arr = [0u8; 16];
                            if v.len() == 16 {
                                arr.copy_from_slice(&v);
                                Some(arr)
                            } else {
                                None
                            }
                        });
                        Ok(Sensor {
                            serial: row.get(0)?,
                            pin: row.get(1)?,
                            address: row.get(2)?,
                            shared_key,
                        })
                    })
                    .map_err(|e| format!("DB query: {}", e))?;
                let mut sensors = Vec::new();
                for row in rows {
                    sensors.push(row.map_err(|e| format!("DB row: {}", e))?);
                }
                Ok(sensors)
            })
            .await
            .map_err(|e| format!("DB join: {}", e))?
        })
    }
}

pub struct SaveSensor {
    pub path: Arc<String>,
    pub sensor: Sensor,
}

impl SaveSensor {
    pub fn new(path: impl Into<String>, sensor: Sensor) -> Self {
        SaveSensor {
            path: Arc::new(path.into()),
            sensor,
        }
    }

    pub fn run(self) -> Task<()> {
        Task::new(async move {
            let path = self.path.clone();
            let sensor = self.sensor;
            tokio::task::spawn_blocking(move || -> Result<(), String> {
                let conn = open_db(&path)?;
                let shared_key_blob: Option<Vec<u8>> = sensor.shared_key.map(|k| k.to_vec());
                conn.execute(
                    "INSERT OR REPLACE INTO sensors (serial, pin, address, shared_key) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![sensor.serial, sensor.pin, sensor.address, shared_key_blob],
                )
                .map_err(|e| format!("DB insert: {}", e))?;
                Ok(())
            })
            .await
            .map_err(|e| format!("DB join: {}", e))?
        })
    }
}
