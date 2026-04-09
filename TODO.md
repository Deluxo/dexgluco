# Dexgluco Development TODO

## Protocol Overview

```
fn main() {
    let sensors = get_sensors(get_from_storage, connect_new)?;
    let connections = connect(via_bt);
    monitor(connections);
}
```

---

## Phase 1: Define the Protocol in Code

### 1.1 Types (src/types.rs)
- [ ] Define: Sensor, Connection, GlucoseReading, SerialNumber, PairingPin
- [ ] Error type: `String` (simple diagnostics)

### 1.2 get_sensors (src/workflows/get_sensors.rs)
- [ ] Function: `get_sensors(get_from_storage, connect_new) -> Result<Sensor, String>`
- [ ] Sub-function: `get_from_storage()` - loads from DB
- [ ] Sub-function: `connect_new()` - adds new via QR → BLE

### 1.3 connect (src/workflows/connect.rs)
- [ ] Function: `connect(via_bt) -> Result<Vec<Connection>, String>`
- [ ] Takes connector implementation

### 1.4 monitor (src/workflows/monitor.rs)
- [ ] Function: `monitor(connections) -> Result<Infallible, String>`
- [ ] Process glucose readings
- [ ] Handle disconnects

### 1.5 main.rs (src/main.rs)
- [ ] Wire up the protocol with partial applications
- [ ] Inject real implementations

---

## Phase 2: Implementations (Injected via Partial Application)

### 2.1 QR Implementation
- [ ] `via_qr()` - decode QR from image file (rqrr)

### 2.2 BLE Implementation
- [ ] `via_bt()` - connect via Bluetooth (bluer)

### 2.3 Storage Implementation
- [ ] `get_from_storage()` - read from SQLite

---

## Phase 3: UI Layer (relm4/GTK4)

- [ ] Create main window
- [ ] Wire up workflow to UI events
- [ ] Display glucose values

---

## Testing Strategy

```rust
// Test uses same protocol, different implementations
let get_sensors_test = get_sensors(
    || Ok(vec![]),           // empty storage
    || Ok(sensor),          // mock new sensor
);
let connect_test = connect(|_| Ok(conn));  // mock connection
```