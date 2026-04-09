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

## Current State

- Empty project with basic relm4/GTK4 setup
- Cargo.toml configured with relm4 and relm4-components
- No BLE implementation yet
- No sensor protocol implementation

## Current Priority

Implement BLE communication with **Dexcom ONE+** sensor.

## Dexcom ONE+ Protocol

### BLE UUIDs
- Service UUID: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E` (Nordic UART)
- TX Characteristic: `6E400002-B5A3-F393-E0A9-E50E24DCCA9E` (Write)
- RX Characteristic: `6E400003-B5A3-F393-E0A9-E50E24DCCA9E` (Notify)

### Warmup Time
- Dexcom ONE+ warmup: **30 minutes**

## Implementation Steps

### Phase 1: Core Infrastructure

1. Create `src/consts.rs` - All hardcoded configuration values
2. Add dependencies to Cargo.toml (bluer, rusqlite, chrono, rqrr, tokio)

### Phase 2: QR Code Scanning

3. Create `src/ble/dexcom/qr_parser.rs`
   - Load image file
   - Parse QR code using rqrr
   - Extract: sensor serial, pairing PIN

### Phase 3: BLE Communication

4. Create `src/ble/mod.rs` - BLE module root
5. Implement `src/ble/scanner.rs` - BLE device discovery
   - Filter: device name contains "DEXCOM"
   - Timeout: 60 seconds
6. Implement `src/ble/pairing.rs` - BlueZ pairing agent
   - Register as default agent
   - Provide PIN automatically during pairing
7. Implement `src/ble/gatt.rs` - GATT client
   - Connect to sensor
   - Discover services/characteristics
   - Subscribe to RX notifications

### Phase 4: Data Layer

8. Create `src/data/mod.rs` - Data module
9. Create `src/data/sensor.rs` - Sensor storage (cached pairing info)
10. Create `src/data/glucose.rs` - Glucose readings storage

### Phase 5: UI Layer

11. Create main window with "Add Sensor" button
12. Implement file picker → QR scan flow
13. Display glucose value, trend arrow, timestamp
14. Show sensor state (Pairing, Warmup, Active, Disconnected)

### Multi-Sensor Architecture

```rust
struct App {
    sensors: HashMap<SensorId, SensorHandle>,
}

struct SensorHandle {
    state: SensorState,
    device: Option<Arc<SensorDevice>>,
    pairing_info: PairingInfo,
}

enum SensorState {
    Pairing,
    Warmup { started: DateTime },
    Active,
    Disconnected,
}
```

## Key Files

| File | Purpose |
|------|---------|
| `src/consts.rs` | All hardcoded constants |
| `src/main.rs` | Entry point |
| `Cargo.toml` | Dependencies |
| `src/ble/mod.rs` | BLE module |
| `src/ble/scanner.rs` | BLE device discovery |
| `src/ble/pairing.rs` | BLE pairing agent |
| `src/ble/gatt.rs` | GATT client |
| `src/ble/dexcom/qr_parser.rs` | QR code parsing |
| `src/ble/dexcom/parser.rs` | Glucose data parsing |
| `src/data/mod.rs` | Data module |
| `src/data/sensor.rs` | Sensor storage |
| `src/data/glucose.rs` | Glucose storage |

## Commands

```bash
# Run application
cargo run

# Check compilation
cargo check

# Build release
cargo build --release

# Linting
cargo clippy -- -D warnings
```

## Constraints

- Must work on Linux with BlueZ
- Desktop GTK4 application (not headless)
- Start with Dexcom ONE+ only (MVP)
- Support multiple sensors via HashMap (not limited to 1)

## Resources

- Juggluco repository: https://github.com/j-kaltes/Juggluco
- Dexcom ONE+ uses similar protocol to Dexcom G7
- bluer crate: https://docs.rs/bluer
- rqrr crate: https://crates.io/crates/rqrr