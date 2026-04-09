# Dexgluco Design Document

## Architecture: Functional Core / Imperative Shell

The application follows the **Functional Core / Imperative Shell** pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                     IMPERATIVE SHELL                         │
│   (main.rs - wires up the protocol, injects implementations)│
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      FUNCTIONAL CORE                        │
│   (Pure workflows - no IO, no side effects, testable)       │
└─────────────────────────────────────────────────────────────┘
```

---

## The Protocol

The application has three main operations:

```rust
fn main() {
    // Step 1: Get sensors - either from storage or add new
    let sensors = get_sensors(
        get_from_storage,
        connect_new,
    )?;

    // Step 2: Connect to all sensors
    let connections = connect(via_bt);

    // Step 3: Monitor incoming readings
    monitor(connections);
}
```

Each function is partially applied with real or mock implementations.

---

## get_sensors()

Meta function that retrieves sensors from storage OR adds new one:

```rust
fn get_sensors(
    get_from_storage: impl Fn() -> Result<Sensor, String>,    // read saved sensors
    connect_new: impl Fn() -> Result<Sensor, String>,          // add new: QR → BLE → sensor
) -> Result<Sensor, String>
```

Returns `Result<Sensor, String>` - Left is error string (for debugging), Right is sensor.

- Tries `get_from_storage` first
- If returns empty/none: tries `connect_new`
- Sensor contains everything needed: serial, pin, bonding info

---

## connect()

Simple connector abstraction:

```rust
fn connect(
    via_bt: impl Fn(Sensor) -> Result<Connection, String>,
) -> Result<Vec<Connection>, String>
```

- Takes a connector function (real bluer in production, mock in tests)
- Connector handles reconnection, re-auth, or fresh auth - that's implementation detail
- Returns connections for monitoring

---

## monitor()

Ongoing operation:

```rust
fn monitor(
    connections: Vec<Connection>,
) -> Result<Infallible, String>
```

- Processes incoming glucose readings
- Handles disconnects/reconnects
- Runs indefinitely (never returns normally)

---

## Partial Application Pattern

The same functions work in production and tests:

```rust
// Production - real implementations
let get_sensors_prod = get_sensors(
    sqlite_load_sensors,
    add_new_sensor_with_qr_and_ble,
);

let connect_prod = connect(bluer_connect);

// Test - mock implementations
let get_sensors_test = get_sensors(
    vec![],  // empty storage
    mock_new_sensor,  // returns prefilled sensor
);

let connect_test = connect(mock_connect);
```

---

## Error Handling

All functions return `Result<T, String>`:
- Left: error string (for developer debugging)
- Right: success value

No complex error types needed - simple string diagnostics.

---

## Implementation Details

### Action-Based Naming

Functions describe what they do, not what concept they represent:

```rust
fn get_from_storage()     // read saved sensors
fn connect_new()           // add new sensor via QR → BLE
fn via_bt()               // connect via Bluetooth
fn monitor()              // ongoing monitoring
```

### File Organization

The file structure will be defined after writing main.rs - files will find their rightful place after the protocol is outlined.

---

## Design Principles

1. **Explicit over implicit** - dependencies passed as params, not hidden in "context"
2. **Composable** - small functions composed into workflows
3. **Testable** - same code runs in production and tests via partial application
4. **Simple errors** - strings for debugging, not complex error types
5. **UI-agnostic core** - functional core has no UI knowledge; shell handles UI