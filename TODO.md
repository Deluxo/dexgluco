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
- [x] `Sensor.shared_key: Option<[u8; 16]>` for bonded fast path

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

## Phase 2: Task Monad & IO Layer ✅

### 2.1 Task Monad — `src/io/task.rs`
- [x] Define `Task<A>` struct wrapping `Pin<Box<dyn Future<Output=Result<A,String>> + Send>>`
- [x] Implement: `new()`, `run()`, `map()`, `and_then()`, `from_value()`, `From<Result<A, String>>`
- [x] Tests: verify composition with map/and_then, verify laziness (no side effects without run)

### 2.2 Storage — `src/io/storage.rs`
- [x] `LoadSensors.run() -> Task<Vec<Sensor>>` — SQLite read
- [x] `SaveSensor(sensor).run() -> Task<()>` — SQLite write (upsert)
- [x] DB schema: `sensors(serial TEXT PK, pin TEXT, address TEXT, shared_key BLOB)`
- [x] DB path: `~/.dexgluco/sensors.db` (auto-create directory)
- [x] Tests: in-memory SQLite roundtrip

### 2.3 DataMatrix Scanning — `src/io/qr.rs`
- [x] `ScanDataMatrix(path).run() -> Task<(String, String)>` — image → (serial, code)
- [x] Uses `rxing::datamatrix::DataMatrixReader` with `image` crate for loading
- [x] Parse GS1 format: strip prefix, extract 4-char pairing code
- [x] Tests: decode known test DataMatrix image

### 2.4 BLE: Scanning — `src/io/ble/mod.rs`
- [x] `ScanForSensor(serial).run() -> Task<String>` — scan for Dexcom devices
- [x] Uses `bluer` adapter for BLE LE scanning
- [x] Filter by device name pattern or service UUID
- [x] Return BLE address on match

### 2.5 BLE: J-PAKE — `src/io/ble/jpake.rs`
- [x] Custom J-PAKE protocol (not RFC 8236), matching Juggluco exactly
- [x] Pure Rust: `p256`, `sha2`, `aes`, `ecdsa`, `signature`, `elliptic-curve`
- [x] `DexContext` — party state with 2 keypairs, 3 certs, Schnorr ZKP
- [x] `Cert::fill()` + `validate12()` + `validate3()` — Schnorr ZKP creation & verification
- [x] `derive_shared_key()` — SHA-256 of affine x coordinate
- [x] `dex8aes()` — AES-128-ECB on 8-byte blocks
- [x] `dex_challenger()` — ECDSA signing with fixed keyC
- [x] Fixed ZKP exponent `FIXED_RAN3` from Juggluco
- [x] Tests: cert roundtrip, generator, AES, ECDSA, J-PAKE roundtrip (ignored, needs vector alignment)
- [x] DER certificate constants: `certs.rs` (keks_p1, keks_p2, keyC)

### 2.6 BLE: State Machine — `src/io/ble/protocol.rs`
- [x] `BleSession::new(device, pin, shared_key)` — discover service + characteristics
- [x] Auth state machine: Init → Round1→2→3 → RequestAuth → ChallengeReply → CertExchange → ProofOfPossession → GetData
- [x] 20-byte MTU packet assembly/disassembly on char 3538
- [x] Certificate exchange with embedded cert data (keks_p1, keks_p2)
- [x] Bonded fast path: skip J-PAKE when shared_key exists
- [x] `authenticate() -> Result<[u8; 16], String>` — run full state machine, return shared key
- [x] `read_glucose() -> Result<GlucoseReading, String>` — parse EGV packet
- [x] `ConnectSensor.run() -> Task<Connection>` — BLE connect + auth + return connection

### 2.7 Core Migration — `src/core/mod.rs`
- [x] Update `get_sensors` signatures: `impl Fn() -> Task<T>` instead of `impl Fn() -> Result<T, String>`
- [x] Update `connect` signatures: `impl Fn(Sensor) -> Task<Connection>`
- [x] Update `monitor` to async loop using `Task::new()`
- [x] Verify existing tests pass with new signatures

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

## Known Open Issues

1. **J-PAKE roundtrip test** (`test_full_jpake_roundtrip`) ignored — shared key derivation math needs alignment with Juggluco test vectors from `testmulti()` in ecJPake.cpp
2. **BLE MTU negotiation** — 20-byte chunks assumed, may need to negotiate larger MTU
3. **Reconnection flow** — how to resume authenticated session without full re-pairing
4. **GTK4 integration** — wiring async Task execution into relm4 event loop

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

// IO modules have unit tests with real deps (SQLite in-memory, J-PAKE self-test):
// - storage: LoadSensors ↔ SaveSensor roundtrip
// - qr: decode known DataMatrix image
// - ble/jpake: cert roundtrip, AES, ECDSA, generator validation

// E2E test: full pipeline with mocked io
let result = get_sensors(
    || Task::from_value(vec![]),
    || Task::from_value(Sensor { serial: "TEST".into(), pin: "ABCD".into(), address: "00:00".into(), shared_key: None }),
)
.and_then(|s| connect(|s| Task::from_value(Connection { sensor: s, stream: vec![] }), s))
.and_then(|_| Task::from_value(()))
.run().await;

assert!(result.is_ok());
```
