# Agent Instructions

This file contains instructions for AI agents working on dexgluco.

## Project Overview

Dexgluco is a Linux desktop CGM (Continuous Glucose Monitoring) application written in Rust using GTK4 (relm4). It receives glucose readings via Bluetooth from Dexcom ONE+ sensors.

## Tech Stack

- **Language**: Rust
- **UI Framework**: relm4 + GTK4
- **BLE**: bluer crate
- **Database**: rusqlite
- **Async**: tokio
- **Storage**: SQLite

## Current State

- Empty project with basic relm4/GTK4 setup
- Cargo.toml configured with relm4 and relm4-components
- No BLE implementation yet
- No sensor protocol implementation

## Current Priority

Implement BLE communication with **Dexcom ONE+** sensor.

## Dexcom ONE+ Protocol Notes

Based on Juggluco code analysis:

### BLE UUIDs
- Service UUID: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E` (Nordic UART)
- TX Characteristic: `6E400002-B5A3-F393-E0A9-E50E24DCCA9E` (Write)
- RX Characteristic: `6E400003-B5A3-F393-E0A9-E50E24DCCA9E` (Notify)

### Sensor Information
- Device name: Typically contains "DEXCOM" or sensor serial
- Requires pairing with pairing code from sensor packaging
- Data format: Proprietary encrypted format

### Implementation Steps
1. Add `bluer` crate to Cargo.toml
2. Implement BLE scanning to find Dexcom ONE+ sensors
3. Implement connection and authentication
4. Subscribe to glucose notifications
5. Parse glucose data packets
6. Display in UI

## Important Files

- `src/main.rs` - Entry point
- `Cargo.toml` - Dependencies

## Key Constraints

- Must work on Linux with BlueZ
- Desktop GTK4 application (not headless)
- Start with Dexcom ONE+ only (MVP approach)

## Commands

```bash
# Run application
cargo run

# Check compilation
cargo check

# Build release
cargo build --release
```

## Linting

Run `cargo clippy -- -D warnings` before committing.

## Resources

- Juggluco repository: https://github.com/j-kaltes/Juggluco
- Dexcom ONE+ uses similar protocol to Dexcom G7
- bluer crate documentation: https://docs.rs/bluer
