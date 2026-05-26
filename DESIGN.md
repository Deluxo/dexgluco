# Dexgluco Design Document

## Architecture: Functional Core / Imperative Shell

The application follows a **three-layer Functional Core / Imperative Shell** pattern:

```
┌─────────────────────────────────────────────────────────────────┐
│                     main.rs (WIRING LAYER)                      │
│   - Partial application: injects io::* into core::* closures   │
│   - Runs the composed Task pipeline                             │
│   - Only place that knows both core and io                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                  core/ (FUNCTIONAL CORE)                         │
│   - Pure workflows that compose Task<A> values                  │
│   - Declares its needs via impl Fn() -> Task<T> parameters      │
│   - Knows nothing about bluer, rusqlite, or any I/O crate       │
│   - Testable by injecting mock closures that return Task values │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    io/ (IMPERATIVE SHELL)                        │
│   - Concrete effect types — each struct does ONE side effect    │
│   - All hardware/network/filesystem deps live here              │
│   - Each type exposes .run(self) -> Task<A>                     │
│   - Independently swappable (real vs test implementations)      │
└─────────────────────────────────────────────────────────────────┘
```

### Rationale for Three Layers

The core insight is that **the structure of a program should communicate what it does**. When you open a file, the directory it lives in tells you immediately whether it touches hardware, performs business logic, or wires things together.

**Why not two layers?** A pure two-layer split (core vs shell) works for simple apps, but our shell has two distinct responsibilities:
1. Implementing side effects (BLE, SQL, camera)
2. Wiring those implementations into the core

By separating these into `io/` and `main.rs`, each file has a single, obvious purpose.

**Why not a framework (DI, actors, etc.)?** Because partial application (passing functions as parameters) is zero-overhead, zero-magic, and entirely type-checked by the compiler. There is no runtime wiring, no proc macros, no registration. The types tell you exactly what a function needs.

---

## The Task Monad

`Task<A>` is the bridge between all three layers.

```rust
pub struct Task<A>(Pin<Box<dyn Future<Output = Result<A, String>> + Send>>);
```

It is:
- **Lazy** — constructing a `Task` does nothing. Only `.run().await` executes the side effects.
- **Async** — wraps a `Future`, so I/O operations don't block the caller.
- **Composable** — `map` transforms values, `and_then` chains dependent operations.
- **Unified error type** — all failures are `String`, keeping things simple.

```rust
impl<A: Send + 'static> Task<A> {
    /// Build a Task from a future (used by io types)
    pub fn new(fut: impl Future<Output = Result<A, String>> + Send + 'static) -> Self

    /// Execute the task (only at the edge: main.rs or tests)
    pub async fn run(self) -> Result<A, String>

    /// Transform a success value synchronously
    pub fn map<B>(self, f: impl FnOnce(A) -> B + Send + 'static) -> Task<B>

    /// Chain to another Task-producing function
    pub fn and_then<B>(self, f: impl FnOnce(A) -> Task<B> + Send + 'static) -> Task<B>

    /// Lift a value into a Task (for mocks / pure computations)
    pub fn from_value(val: A) -> Task<A>

    /// Lift an error into a Task
    pub fn from_err(err: String) -> Task<Err>
}
```

### Why `map` and `and_then`?

These two combinators are sufficient to build any data pipeline:

- `map` is for infallible transformations: `ScanDataMatrix(path).run().map(|(s, p)| Sensor { serial: s, pin: p })`
- `and_then` is for fallible chaining: `LoadSensors.run().and_then(|sensors| connect(via_bt, sensors))`

This is the minimum viable monad interface. No `Applicative`, no `Monad` trait — just the two combinators we actually need.

### Execution Boundary

Tasks are constructed throughout the program but **never executed inside core or io**. Execution only happens at the very edge — `main.rs` calls `.run().await` on the final composed `Task`. This is the **dependency injection boundary**: before `.run()`, you're building a description of work. After `.run()`, side effects happen.

---

## io/ — Concrete Effect Types

Every I/O operation is represented as a named struct with a single `.run(self) -> Task<A>` method. The struct name IS the documentation.

### Why Structs Instead of Closures?

Closures are anonymous — you can't tell from `impl Fn() -> Result<X>` whether the caller is reading from SQLite, calling an HTTP API, or returning a hardcoded value. A struct with a descriptive name tells you exactly what it does:

```rust
// Clear: this reads from a database
LoadSensors.run()

// Clear: this decodes an image file
ScanDataMatrix("/path/to/image.png").run()

// Opaque: what does this closure do?
|| some_closure_returning_sensors()
```

Structs also give us a natural place for configuration. `ScanDataMatrix` takes a file path. `ConnectSensor` takes sensor credentials. These are fields, not magic.

### The Effect Types

#### `io/storage.rs` — SQLite Persistence

```rust
pub struct LoadSensors;
impl LoadSensors {
    /// Reads all known sensors from the SQLite database.
    /// Returns empty vec if no sensors have been saved yet.
    pub fn run(self) -> Task<Vec<Sensor>>
}

pub struct SaveSensor(pub Sensor);
impl SaveSensor {
    /// Persists a sensor to the database.
    /// Overwrites if sensor with same serial already exists.
    pub fn run(self) -> Task<()>
}
```

**Why SQLite?** It's a single file, no server, no configuration. The bundled feature means no system dependency. It's the simplest way to persist sensor info across restarts.

**Why named structs and not a trait?** A trait like `Storage { fn load() -> ..; fn save() -> .. }` would couple load and save into one abstraction. They are separate concerns. Using separate structs means we can mock one without mocking the other, and the type system prevents accidentally calling `save` when you meant `load`.

#### `io/qr.rs` — DataMatrix Scanning

```rust
pub struct ScanDataMatrix(pub String);  // file path to image
impl ScanDataMatrix {
    /// Decodes a DataMatrix barcode from the given image file.
    /// Returns (sensor_serial, pairing_code) extracted from the GS1 data.
    pub fn run(self) -> Task<(String, String)>
}
```

**Why DataMatrix and not QR?** The Dexcom ONE+ applicator uses a DataMatrix code, not a QR code. `rxing` is a Rust port of ZXing and supports both.

**Why accept a file path?** For MVP, the user saves a photo of the applicator and passes the path. Future versions can use a camera. This keeps the I/O boundary simple — reading a file is a well-understood operation with no hardware dependencies.

#### `io/ble/` — BLE Communication

The BLE module has the most complex implementation, split into three files:

**`io/ble/mod.rs`** — High-level operations:

```rust
pub struct ScanForSensor(pub String);  // sensor serial to find
impl ScanForSensor {
    /// Scans BLE for a device with "DXCM" prefix or matching serial.
    /// Returns the BLE MAC address as a string.
    pub fn run(self) -> Task<String>
}

pub struct ConnectSensor {
    pub serial: String,
    pub pin: String,
    pub address: String,
}
impl ConnectSensor {
    /// Connects to the sensor at the given address.
    /// Performs full EC-JPAKE authentication handshake.
    /// Returns a Connection with an active glucose stream.
    pub fn run(self) -> Task<Connection>
}
```

**`io/ble/jpake.rs`** — EC-JPAKE Cryptography:

Wraps mbedtls's EC-JPAKE implementation (via `mbedtls-sys-auto` FFI):

```rust
pub struct JPakeSession {
    ctx: mbedtls_sys::ecjpake_context,
}

impl JPakeSession {
    /// Initialize EC-JPAKE context with pairing code as shared secret.
    /// Role: MBEDTLS_ECJPAKE_CLIENT (we initiate pairing).
    /// Hash: SHA-256. Curve: secp256r1 (per Dexcom spec).
    pub fn new(pairing_code: &str) -> Result<Self, String>

    /// Generate and return round 1 message (to send to sensor).
    pub fn write_round1(&mut self) -> Result<Vec<u8>, String>

    /// Process sensor's round 1 message.
    pub fn read_round1(&mut self, data: &[u8]) -> Result<(), String>

    /// Generate and return round 2 message.
    pub fn write_round2(&mut self) -> Result<Vec<u8>, String>

    /// Process sensor's round 2 message.
    pub fn read_round2(&mut self, data: &[u8]) -> Result<(), String>

    /// Derive shared secret after both rounds complete.
    pub fn derive_secret(&mut self) -> Result<Vec<u8>, String>
}
```

**`io/ble/protocol.rs`** — BLE State Machine:

Models the full Dexcom G7/ONE+ authentication handshake as a state machine:

```rust
pub struct BleSession { /* bluer device, characteristics, phase, jpake */ }

impl BleSession {
    pub fn new(device: bluer::Device, pin: &str) -> Result<Self, String>
    pub async fn authenticate(&mut self) -> Result<(), String>
    pub async fn read_glucose(&mut self) -> Result<GlucoseReading, String>
}
```

The auth state machine has these phases:

```
Init → PakeRound0 → PakeRound1 → PakeRound2 → Challenge
→ CertificateExchange → ProofOfPossession → KeepAlive → BondRequest → Authenticated
```

Each phase corresponds to specific BLE writes/notifications on the auth characteristics.

---

## core/ — Pure Workflow Functions

The core defines the three main workflows:

```rust
pub fn get_sensors(
    get_from_storage: impl Fn() -> Task<Vec<Sensor>>,
    connect_new: impl Fn() -> Task<Sensor>,
) -> Task<Vec<Sensor>> {
    get_from_storage().and_then(|stored| {
        if !stored.is_empty() {
            Task::from_value(stored)
        } else {
            connect_new().map(|s| vec![s])
        }
    })
}

pub fn connect(
    via_bt: impl Fn(Sensor) -> Task<Connection>,
    sensors: Vec<Sensor>,
) -> Task<Vec<Connection>> {
    // Sequentially connect each sensor, collecting results
    let mut chain = Task::from_value(vec![]);
    for sensor in sensors {
        chain = chain.and_then(move |mut conns| {
            via_bt(sensor).map(|c| {
                conns.push(c);
                conns
            })
        });
    }
    chain
}

pub fn monitor(
    connections: Vec<Connection>,
) -> Task<()> {
    // Loop: read glucose from each connection, sleep, repeat
    Task::new(async move {
        loop {
            for conn in &connections {
                // process conn.stream
            }
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    })
}
```

### Why These Signatures?

Each function receives its I/O dependencies as closures that return `Task<T>`:

- `get_sensors` needs two I/O capabilities: loading from storage, and creating a new sensor. These are independent concerns passed as separate closures.
- `connect` needs one capability per sensor: the ability to turn a `Sensor` into a `Connection`. The vec of sensors is provided separately because it comes from the output of `get_sensors`.
- `monitor` needs only the connections — once established, no further I/O capabilities are needed for the core.

This is **dependency injection without containers**. There is no global registry, no `Arc<dyn Trait>`, no builder pattern. Just functions taking functions.

---

## Partial Application Pattern

The wiring layer (`main.rs`) creates the final program by partially applying io types into core function holes:

```rust
// Real production pipeline
let program = get_sensors(
    || LoadSensors.run(),
    || scan_new_sensor(),  // combines ScanDataMatrix + ScanForSensor
)
.and_then(|sensors| connect(
    |s| ConnectSensor::new(s).run(),
    sensors,
))
.and_then(monitor);
```

Test mocks swap in trivial Tasks:

```rust
// Test: empty storage, fake sensor, mock connection
let program = get_sensors(
    || Task::from_value(vec![]),
    || Task::from_value(Sensor { serial: "TEST".into(), pin: "1234".into(), address: "AA:BB".into() }),
)
.and_then(|sensors| connect(
    |s| Task::from_value(Connection { sensor: s, stream: vec![] }),
    sensors,
))
.and_then(monitor);
```

The same `get_sensors`, `connect`, and `monitor` functions are used in both. Only the injected closures differ.

---

## Error Handling

All `Task<A>` values resolve to `Result<A, String>`. `String` is intentionally simple:

- **No custom error types**: Every failure is a human-readable string. This avoids the complexity of mapping between error types across the bluer/rusqlite/mbedtls boundaries.
- **No thiserror or anyhow**: Not needed for MVP. If debugging requires more structure, we can add it later without changing function signatures.
- **Error messages describe the problem, not blame**: "BLE connection failed: device not found" rather than "Error code -5".

---

## File Organization

```
src/
  core/
    mod.rs          # get_sensors, connect, monitor — pure, returns Task<A>
    types.rs        # Sensor, Connection, GlucoseReading
  io/
    mod.rs          # Re-exports Task and all effect types
    task.rs         # Task<A> monad
    storage.rs      # LoadSensors, SaveSensor — SQLite
    qr.rs           # ScanDataMatrix — rxing
    ble/
      mod.rs        # ScanForSensor, ConnectSensor — re-exports
      jpake.rs      # JPakeSession — mbedtls EC-JPAKE FFI wrapper
      protocol.rs   # BleSession — BLE auth state machine
  lib.rs            # pub mod core; pub mod io;
  main.rs           # Wiring layer: partial application + execution
tests/
  core_test.rs      # Core with mock Task closures
  io_test.rs        # Storage roundtrip, QR parsing
```

### Module Dependency Rules

```
main.rs  depends on: core, io
core/    depends on: io::task (Task type only), core::types
io/      depends on: io::task, rusqlite, bluer, rxing, mbedtls
```

These rules are not enforced by the compiler (except through `Cargo.toml` features), but they are enforced by convention. A PR that imports `bluer` in `core/mod.rs` should be rejected in review.

---

## Design Principles

1. **Explicit over implicit** — I/O dependencies are passed as function parameters, not hidden in global state or DI containers.
2. **Composable** — `Task<A>` with `map`/`and_then` lets you build complex pipelines from simple pieces.
3. **Testable** — The same core code runs in production and tests via partial application. Mocks are trivial `Task::from_value()` calls.
4. **Simple errors** — Strings for debugging. No complex error type hierarchies.
5. **UI-agnostic core** — The functional core has no knowledge of GTK, relm4, or any UI framework. It produces data; the shell displays it.
6. **Named effects** — Every I/O operation is a named struct, not an anonymous closure. You know what something does by its type.
7. **Single responsibility** — Each struct does one thing. `LoadSensors` loads. `SaveSensor` saves. They are not coupled into a `Storage` trait.
8. **Lazy execution** — `Task<A>` describes work but doesn't execute it. Construction and execution are separate phases.
