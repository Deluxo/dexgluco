# Agent Instructions

This file contains instructions for AI agents working on dexgluco.

## Project Overview

Dexgluco is a Linux desktop CGM (Continuous Glucose Monitoring) application written in Rust using GTK4 (relm4). It receives glucose readings via Bluetooth from Dexcom ONE+ sensors.

## Tech Stack

- **Language**: Rust (2018 edition)
- **UI Framework**: relm4 + GTK4
- **BLE**: bluer crate
- **Database**: rusqlite
- **QR Scanning**: rqrr crate
- **Async**: tokio
- **Storage**: SQLite

## Architecture: Functional Core / Imperative Shell

```
┌─────────────────────────────────────────────────────────────┐
│                     IMPERATIVE SHELL (main.rs)              │
│   - Wires up the protocol with partial applications        │
│   - Injects real implementations                           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                      FUNCTIONAL CORE                        │
│   - Pure workflows (no IO, testable)                        │
│   - get_sensors(), connect(), monitor()                     │
└─────────────────────────────────────────────────────────────┘
```

## The Protocol

```rust
fn main() {
    // 1. Get sensors - from storage OR add new via QR+BLE
    let sensors = get_sensors(
        get_from_storage,
        connect_new,
    )?;

    // 2. Connect to sensors via BT
    let connections = connect(via_bt);

    // 3. Monitor incoming readings
    monitor(connections);
}
```

### get_sensors()

```rust
fn get_sensors(
    get_from_storage: impl Fn() -> Result<Sensor, String>,
    connect_new: impl Fn() -> Result<Sensor, String>,
) -> Result<Sensor, String>
```

- Tries `get_from_storage` first (saved sensors from DB)
- If none: tries `connect_new` (QR → BLE → new sensor)
- Returns sensor with: serial, pin, bonding info

### connect()

```rust
fn connect(
    via_bt: impl Fn(Sensor) -> Result<Connection, String>,
) -> Result<Vec<Connection>, String>
```

- Takes connector (real bluer or mock)
- Handles reconnect/new auth - that's implementation detail

### monitor()

```rust
fn monitor(connections: Vec<Connection>) -> Result<Infallible, String>
```

- Process glucose readings
- Handle disconnects
- Runs indefinitely

## Error Handling

All functions return `Result<T, String>`:
- Left: error string for debugging
- Right: success value

## Partial Application

Production vs test uses same code:

```rust
// Production
let get_sensors_prod = get_sensors(sqlite_load, add_new_sensor);

// Test  
let get_sensors_test = get_sensors(|| Ok(vec![]), mock_new_sensor);
```

## Key Implementation Traits

Use action-based naming (what it does, not what it is):
- `get_from_storage` - read saved sensors
- `connect_new` - add new sensor (QR → BLE)
- `via_bt` - connect via Bluetooth
- `via_qr` - decode QR image

## Commands

```bash
cargo run         # Run application
cargo check       # Check compilation
cargo build --release  # Build release
cargo clippy -- -D warnings  # Linting
```

## Constraints

- Must work on Linux with BlueZ
- Desktop GTK4 application (not headless)
- Start with Dexcom ONE+ only (MVP)
- No server/cloud - local only

## Resources

- Juggluco repository: https://github.com/j-kaltes/Juggluco
- Dexcom ONE+ uses similar protocol to Dexcom G7
- bluer crate: https://docs.rs/bluer
- rqrr crate: https://crates.io/crates/rqrr