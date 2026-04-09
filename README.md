# Dexgluco - Linux CGM Application

Dexgluco is a Linux desktop application that receives glucose readings via Bluetooth from CGM sensors and displays them in a GTK4 GUI.

## Architecture

Uses **Functional Core / Imperative Shell** pattern:

```
┌─────────────────────────────────────────────────┐
│            IMPERATIVE SHELL (main.rs)           │
│   Wires up protocol with partial applications   │
├─────────────────────────────────────────────────┤
│                FUNCTIONAL CORE                  │
│   Pure workflows: get_sensors, connect, monitor │
└─────────────────────────────────────────────────┘
```

### The Protocol

```rust
fn main() {
    // 1. Get sensors - from storage OR add new via QR+BLE
    let sensors = get_sensors(get_from_storage, connect_new)?;

    // 2. Connect to sensors via BT
    let connections = connect(via_bt);

    // 3. Monitor incoming readings
    monitor(connections);
}
```

## Supported Sensors

- **Dexcom ONE+** (initial target)
- Future: Freestyle Libre 2/3, Dexcom G7, Sibionics GS1, etc.

## Key Concepts

- **Partial Application** - Same workflow works in test and production
- **Action-based naming** - `get_from_storage`, `connect_new`, `via_bt`
- **Simple errors** - `Result<T, String>` for debugging

## Quick Start

```bash
cargo run
```

## Dependencies

- Rust (latest stable)
- GTK4 libraries
- BlueZ (Linux Bluetooth stack)

## Key Crates

| Crate | Purpose |
|-------|---------|
| relm4 | GTK4 UI framework |
| bluer | BLE communication |
| rusqlite | Local storage |
| tokio | Async runtime |
| chrono | Date/time |