# Agent Instructions

This file contains instructions for AI agents working on dexgluco.

## Project Overview

Dexgluco is a Linux desktop CGM (Continuous Glucose Monitoring) application written in Rust using GTK4 (relm4). It receives glucose readings via Bluetooth from Dexcom ONE+ sensors.

## Tech Stack

- **Language**: Rust (2018 edition)
- **UI Framework**: relm4 + GTK4
- **BLE**: bluer crate
- **Database**: rusqlite (bundled)
- **Auth**: mbedtls EC-JPAKE (system lib)
- **Barcode**: rxing crate (DataMatrix decoding)
- **Async**: tokio
- **Storage**: SQLite

## Architecture: Three-Layer Functional Core / Imperative Shell

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
│   - Pure workflows: get_sensors(), connect(), monitor()         │
│   - No I/O crate deps (no bluer, rusqlite, etc.)               │
│   - All parameters are impl Fn() -> Task<T>                     │
│   - All return types are Task<T>                                │
│   - Composes Task<A> with map/and_then                          │
│   - Testable with mock closures returning Task::from_value()    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    io/ (IMPERATIVE SHELL)                        │
│   - All side effects live here                                  │
│   - All crate deps live here (bluer, rusqlite, rxing, mbedtls)  │
│   - Concrete effect types: structs with .run(self) -> Task<A>   │
│   - Each type does ONE thing and does it well                   │
│   - Types: LoadSensors, SaveSensor, ScanDataMatrix,             │
│           ScanForSensor, ConnectSensor                           │
│   - Swappable: swap SQLite for in-memory, bluer for mock        │
└─────────────────────────────────────────────────────────────────┘
```

### Why Three Layers?

The separation exists to make every piece independently testable and swappable.

**io/** contains every crate that touches hardware, network, or filesystem. If you open `io/ble.rs`, you know immediately it depends on bluer and talks to a BLE radio. If you open `io/storage.rs`, you know it uses rusqlite and reads/writes a SQLite file. This means:

- To run tests without hardware: swap `io::ble::ConnectSensor` for a mock that returns `Task::from_value(connection)`.
- To test storage: swap the file path to `:memory:`.
- To audit security: only `io/` files touch the network.

**core/** never imports bluer, rusqlite, rxing, or mbedtls. It only knows about `Task<A>` and its own types (`Sensor`, `Connection`, `GlucoseReading`). The core declares *what* it needs via function parameter holes:

```rust
pub fn get_sensors(
    get_from_storage: impl Fn() -> Task<Vec<Sensor>>,
    connect_new: impl Fn() -> Task<Sensor>,
) -> Task<Vec<Sensor>>
```

The core doesn't know whether `get_from_storage` reads SQLite, reads a JSON file, or returns hardcoded test data. It just says "give me sensors somehow."

**main.rs** (the wiring layer) is the only place that knows about both sides. It's a thin file that:

1. Creates concrete io structs or test mocks
2. Wraps them in closures that match core's expected signatures
3. Calls core functions with those closures
4. Runs the final composed `Task` via `.run().await`

```rust
// main.rs — the only file that imports both core and io
fn main() {
    let pipeline = get_sensors(
        || LoadSensors.run(),               // io type injected into core hole
        || scan_new_sensor(),                // composed from multiple io types
    )
    .and_then(|sensors| connect(
        |s| ConnectSensor::new(s).run(),    // io type injected into core hole
        sensors,
    ))
    .and_then(monitor);

    rt.block_on(pipeline.run());
}
```

## The Task Monad

`Task<A>` is the glue that connects core and io. It is a lazy, async, composable wrapper:

```rust
pub struct Task<A>(Pin<Box<dyn Future<Output = Result<A, String>> + Send>>);
```

### Why Lazy?

When you call `LoadSensors.run()`, it does NOT execute the SQL query. It builds a `Task<Vec<Sensor>>` — a description of the work. The work only executes when you call `.run().await` on the final composed task.

This lets core functions compose whole pipelines without ever executing I/O:

```rust
pub fn connect(
    via_bt: impl Fn(Sensor) -> Task<Connection>,
    sensors: Vec<Sensor>,
) -> Task<Vec<Connection>> {
    // Returns a Task that, when run, will:
    // 1. Take each sensor
    // 2. Call via_bt for each (which returns a Task)
    // 3. Collect all results
    // But NONE of this executes until the caller runs the Task
}
```

### Why Async?

BLE communication inherently blocks for indeterminate periods (scanning, connecting, pairing). The `Task` monad wraps a `Future`, so io operations can suspend while waiting for the BLE radio without blocking the UI thread.

### Why Composable?

`Task<A>` provides `map` and `and_then`:

```rust
impl<A: Send + 'static> Task<A> {
    pub fn map<B>(self, f: impl FnOnce(A) -> B + Send + 'static) -> Task<B>
    pub fn and_then<B>(self, f: impl FnOnce(A) -> Task<B> + Send + 'static) -> Task<B>
}
```

- `map` transforms a successful value synchronously
- `and_then` chains to another Task-producing function

This lets you build pipelines declaratively:

```rust
ScanDataMatrix(path).run()                          // Task<(String, String)>
    .map(|(serial, pin)| Sensor { serial, pin })   // Task<Sensor>
    .and_then(|s| ConnectSensor::new(s).run())      // Task<Connection>
```

### The Lifecycle

1. **Construction**: Io structs are created (e.g. `LoadSensors`). Calling `.run()` returns a `Task<A>`.
2. **Composition**: Core functions take closures that return `Task<A>`, and compose them with `map`/`and_then` into larger `Task<A>` values.
3. **Execution**: At the edge (main.rs or a test), the final `Task` is run with `.run().await`. Only then does any I/O happen.

```
┌─────────────┐     .run()     ┌──────────┐   map/and_then   ┌──────────────┐
│ LoadSensors  │ ────────────→ │ Task<Vec>│ ──────────────→ │ Task<Vec>     │
└─────────────┘               └──────────┘                  │ (composed)    │
                                                             └──────┬───────┘
                                                                    │ .run().await
                                                                    ▼
                                                             ┌──────────────┐
                                                             │ Result<Vec>  │
                                                             │ (side effects│
                                                             │  happen here) │
                                                             └──────────────┘
```

## The Protocol

```rust
fn main() {
    // 1. Get sensors - from storage OR add new via QR+BLE
    let sensors = get_sensors(
        get_from_storage,  // returns Task<Vec<Sensor>>
        connect_new,       // returns Task<Sensor>
    );

    // 2. Connect to sensors via BT
    let connections = sensors.and_then(|sensors| connect(via_bt, sensors));

    // 3. Monitor incoming readings
    let program = connections.and_then(monitor);

    // 4. Execute everything
    rt.block_on(program.run());
}
```

### get_sensors()

```rust
fn get_sensors(
    get_from_storage: impl Fn() -> Task<Vec<Sensor>>,
    connect_new: impl Fn() -> Task<Sensor>,
) -> Task<Vec<Sensor>>
```

- Tries `get_from_storage` first (saved sensors from DB)
- If empty: tries `connect_new` (DataMatrix scan → BLE scan → pair)
- Returns sensor with: serial, pin, address

### connect()

```rust
fn connect(
    via_bt: impl Fn(Sensor) -> Task<Connection>,
    sensors: Vec<Sensor>,
) -> Task<Vec<Connection>>
```

- Takes connector (real `ConnectSensor` or mock)
- Connector handles full J-PAKE auth handshake
- Returns connections ready for monitoring

### monitor()

```rust
fn monitor(
    connections: Vec<Connection>,
) -> Task<()>
```

- Subscribes to glucose notifications
- Processes readings into the stream
- Handles disconnects/reconnects
- Runs indefinitely

## Error Handling

All `Task<A>` values resolve to `Result<A, String>` when run:
- `Ok(value)` — success
- `Err(reason)` — string diagnostics for debugging

## Partial Application Pattern

The same core functions work in production and tests — only the injected implementations differ:

```rust
// Production — real IO
let program = get_sensors(
    || LoadSensors.run(),
    || scan_new_sensor(),  // ScanDataMatrix → ScanForSensor → ConnectSensor
)
.and_then(|s| connect(|s| ConnectSensor::new(s).run(), s))
.and_then(monitor);

// Test — mock IO
let program = get_sensors(
    || Task::from_value(vec![]),              // empty storage
    || Task::from_value(mock_sensor()),       // fake sensor
)
.and_then(|s| connect(|_| Task::from_value(mock_conn()), s))
.and_then(monitor);
```

## Key Implementation Types

### io/ (concrete effect types — each is a struct)

- `LoadSensors` — `.run() -> Task<Vec<Sensor>>` — SQLite read
- `SaveSensor(Sensor)` — `.run() -> Task<()>` — SQLite write
- `ScanDataMatrix(String)` — `.run() -> Task<(String, String)>` — image → pairing data
- `ScanForSensor(String)` — `.run() -> Task<String>` — BLE scan → MAC address
- `ConnectSensor` — `.run() -> Task<Connection>` — BLE connect + J-PAKE auth

### core/ (pure workflow functions)

- `get_sensors(..) -> Task<Vec<Sensor>>`
- `connect(..) -> Task<Vec<Connection>>`
- `monitor(..) -> Task<()>`

## File Organization

```
src/
  core/
    mod.rs          # get_sensors, connect, monitor — all return Task<A>
    types.rs        # Sensor, Connection, GlucoseReading
  io/
    mod.rs          # pub use task::Task; pub use storage, qr, ble modules
    task.rs         # Task<A> monad: lazy async wrapper with map/and_then
    storage.rs      # LoadSensors, SaveSensor — SQLite via rusqlite
    qr.rs           # ScanDataMatrix — DataMatrix via rxing
    ble/
      mod.rs        # ScanForSensor, ConnectSensor — re-exports
      jpake.rs      # JPakeSession — mbedtls EC-JPAKE wrapper
      protocol.rs   # BleSession — BLE state machine (auth phases + data)
  lib.rs            # pub mod core; pub mod io
  main.rs           # Wire io into core, run final Task
tests/
  core_test.rs      # Core workflows with mock Task closures
  io_test.rs        # Storage roundtrip, QR parsing
```

### Why This Layout?

Files in `io/` can depend on bluer, rusqlite, rxing, mbedtls, etc. Files in `core/` cannot. This is enforced by humans reading the code and by the module system — `core/mod.rs` only imports `types.rs` and `Task` from `io/task.rs`. If someone adds an `use bluer` in core, it's instantly visible as wrong.

The `io/ble/` subdirectory exists because BLE has two non-trivial subsystems: the J-PAKE cryptography and the BLE state machine. Splitting them keeps each file focused.

## Commands

```bash
cargo run         # Run application
cargo check       # Check compilation
cargo build --release  # Build release
cargo clippy -- -D warnings  # Linting
cargo test        # Run all tests
```

### Build Prerequisites

**dbus development headers** are required for the `bluer` (bluetoothd feature) crate.
On NixOS, update `PKG_CONFIG_PATH` to include `dbus.dev`:
```bash
export PKG_CONFIG_PATH="$(nix-build --no-out-link -E 'with import <nixpkgs> {}; dbus.dev')/lib/pkgconfig:$PKG_CONFIG_PATH"
```
Or use `nix-shell` from the project root (shell.nix handles this).

## Constraints

- Must work on Linux with BlueZ
- Desktop GTK4 application (not headless)
- Start with Dexcom ONE+ only (MVP)
- No server/cloud — local only

## Resources

- Juggluco repository: https://github.com/j-kaltes/Juggluco
- Dexcom ONE+ uses similar protocol to Dexcom G7
- DiaBLE (Swift G7 implementation): https://github.com/gui-dos/DiaBLE
- xDrip+ keks (J-PAKE C library): https://github.com/NightscoutFoundation/xDrip/tree/master/libkeks
- bluer crate: https://docs.rs/bluer
- rxing crate: https://docs.rs/rxing
- mbedtls crate: https://docs.rs/mbedtls
- mbedtls EC-JPAKE docs: https://mbed-tls.readthedocs.io/projects/api/en/v3.6.4/api/file/ecjpake_8h/
