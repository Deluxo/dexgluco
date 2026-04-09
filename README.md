# Dexgluco - Linux CGM Application

Dexgluco is a Linux desktop application that receives glucose readings via Bluetooth from CGM sensors and displays them in a GTK4 GUI.

## Supported Sensors

- **Dexcom ONE+** (initial target)
- Future: Freestyle Libre 2/3, Dexcom G7, Sibionics GS1, etc.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   UI Layer (GTK4/relm4)          │
├─────────────────────────────────────────────────┤
│                Business Logic                   │
├─────────────────────────────────────────────────┤
│              Sensor Protocols (BLE)             │
└─────────────────────────────────────────────────┘
```

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