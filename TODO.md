# Dexgluco Development TODO

## Protocol Overview

```
fn main() {
    let sensors = get_sensors(get_from_storage, connect_new);
    let connections = sensors.and_then(|s| connect(via_bt, s));
    let program = connections.and_then(monitor);
    rt.block_on(program.run());
}
```

---

## Phase 1: Define the Protocol in Code ✅

### 1.1 Types — `src/core/types.rs`
- [x] Define: `Sensor`, `Connection`, `GlucoseReading`
- [x] Error type: `Result<T, String>` => now `Task<A>` which resolves to `Result<A, String>`

### 1.2 get_sensors — `src/core/mod.rs`
- [x] Function: `get_sensors(get_from_storage, connect_new) -> Task<Vec<Sensor>>`
- [x] Tries storage first, falls back to connect_new

### 1.3 connect — `src/core/mod.rs`
- [x] Function: `connect(via_bt, sensors) -> Task<Vec<Connection>>`
- [x] Takes connector closure returning `Task<Connection>`

### 1.4 monitor — `src/core/mod.rs`
- [x] Function: `monitor(connections) -> Task<()>`
- [x] Async loop processing glucose readings

### 1.5 main.rs
- [x] Wire up protocol with mock implementations (stub)
- [x] Mock DataMatrix → mock BLE → mock storage

---

## Phase 2: Task Monad & IO Layer

### 2.1 Task Monad — `src/io/task.rs`
- [ ] Define `Task<A>` struct wrapping `Pin<Box<dyn Future<Output=Result<A,String>> + Send>>`
- [ ] Implement: `new()`, `run()`, `map()`, `and_then()`, `from_value()`, `from_err()`
- [ ] Tests: verify composition with map/and_then, verify laziness (no side effects without run)

### 2.2 Storage — `src/io/storage.rs`
- [ ] `LoadSensors.run() -> Task<Vec<Sensor>>` — SQLite read
- [ ] `SaveSensor(sensor).run() -> Task<()>` — SQLite write (upsert)
- [ ] DB schema: `sensors(serial TEXT PK, pin TEXT, address TEXT, created_at INTEGER)`
- [ ] DB path: `~/.dexgluco/sensors.db` (auto-create directory)
- [ ] Tests: in-memory SQLite roundtrip

### 2.3 DataMatrix Scanning — `src/io/qr.rs`
- [ ] `ScanDataMatrix(path).run() -> Task<(String, String)>` — image → (serial, code)
- [ ] Uses `rxing::datamatrix::DataMatrixReader` with `image` crate for loading
- [ ] Parse GS1 format: strip prefix, extract 4-char pairing code
- [ ] Tests: decode known test DataMatrix image

### 2.4 BLE: Scanning — `src/io/ble/mod.rs`
- [ ] `ScanForSensor(serial).run() -> Task<String>` — scan for `DXCM` prefix
- [ ] Uses `bluer` adapter for BLE scanning
- [ ] Filter by device name pattern or service UUID
- [ ] Return BLE address on match

### 2.5 BLE: J-PAKE — `src/io/ble/jpake.rs`
- [ ] `JPakeSession::new(pairing_code) -> Result<Self, String>` — mbedtls init
- [ ] `write_round1() -> Result<Vec<u8>, String>` — mbedtls_ecjpake_write_round_one
- [ ] `read_round1(data) -> Result<(), String>` — mbedtls_ecjpake_read_round_one
- [ ] `write_round2() -> Result<Vec<u8>, String>` — mbedtls_ecjpake_write_round_two
- [ ] `read_round2(data) -> Result<(), String>` — mbedtls_ecjpake_read_round_two
- [ ] `derive_secret() -> Result<Vec<u8>, String>` — mbedtls_ecjpake_derive_secret
- [ ] Requires: mbedtls system lib with `MBEDTLS_KEY_EXCHANGE_ECJPAKE` enabled
- [ ] Uses: `mbedtls-sys-auto` raw FFI for EC-JPAKE, `mbedtls` crate for RNG

### 2.6 BLE: State Machine — `src/io/ble/protocol.rs`
- [ ] `BleSession::new(device, pin)` — discover service + characteristics
- [ ] Auth state machine: Init → PakeRound0→1→2 → Challenge → CertExchange → ProofOfPossession → KeepAlive → BondRequest → Authenticated
- [ ] 20-byte MTU packet assembly/disassembly on char 3538
- [ ] Certificate exchange with embedded cert data
- [ ] Proof of possession signing
- [ ] `authenticate() -> Result<(), String>` — run full state machine
- [ ] `read_glucose() -> Result<GlucoseReading, String>` — send 0x4E, parse response
- [ ] `ConnectSensor.run() -> Task<Connection>` — scan + auth + return connection

### 2.7 Core Migration — `src/core/mod.rs`
- [ ] Update `get_sensors` signatures: `impl Fn() -> Task<T>` instead of `impl Fn() -> Result<T, String>`
- [ ] Update `connect` signatures: `impl Fn(Sensor) -> Task<Connection>`
- [ ] Update `monitor` to async loop using `Task::new()`
- [ ] Verify existing tests pass with new signatures

---

## Phase 3: UI Layer (relm4/GTK4)

- [ ] Create main window (GTK4 application window)
- [ ] Display glucose value, trend arrow, timestamp
- [ ] "Add Sensor" button → trigger DataMatrix scan flow
- [ ] Wire workflow to UI events (async Task execution in GTK event loop)
- [ ] Periodic glucose reading updates (5-minute interval)
- [ ] Sensor list view (stored sensors)
- [ ] Settings view (DB management, about)

---

## Phase 4: Polish & Hardening

- [ ] Handle BLE disconnects and auto-reconnect
- [ ] Sensor warmup state detection
- [ ] Graceful error messages in UI
- [ ] Graceful shutdown on Ctrl+C
- [ ] Logging (tracing or log crate)
- [ ] System tray icon
- [ ] Desktop notifications for alarms

---

## Testing Strategy

```rust
// Core is tested with mock Task closures:
let sensors = get_sensors(
    || Task::from_value(vec![]),               // empty storage
    || Task::from_value(test_sensor),           // mock new sensor
);

let conns = sensors.and_then(|s| connect(
    |_| Task::from_value(test_connection()),    // mock BLE
    s,
));

// IO modules have unit tests with real deps (SQLite in-memory):
// - storage: LoadSensors ↔ SaveSensor roundtrip
// - qr: decode known DataMatrix image
// - ble/jpake: J-PAKE round-trip self-test

// E2E test: full pipeline with mocked io
let result = get_sensors(
    || Task::from_value(vec![]),
    || Task::from_value(Sensor { serial: "TEST".into(), pin: "ABCD".into(), address: "00:00".into() }),
)
.and_then(|s| connect(|s| Task::from_value(Connection { sensor: s, stream: vec![] }), s))
.and_then(|_| Task::from_value(()))
.run().await;

assert!(result.is_ok());
```

## Known Open Questions

1. Certificate data for 0x0B exchange — needs to be extracted from Juggluco/xDrip+ keks
2. BLE MTU negotiation — 20-byte chunks assumed, may need to negotiate larger MTU
3. mbedtls `MBEDTLS_KEY_EXCHANGE_ECJPAKE` compile flag — must verify Nix package has it enabled
4. Reconnection flow — how to resume authenticated session without full re-pairing
