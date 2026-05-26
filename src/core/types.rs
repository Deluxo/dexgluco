#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Sensor {
    pub serial: String,
    pub pin: String,
    pub address: String,
    pub shared_key: Option<[u8; 16]>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Connection {
    pub sensor: Sensor,
    pub stream: Vec<GlucoseReading>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct GlucoseReading {
    pub value: f32,
    pub timestamp: i64,
    pub trend: i32,
}