# Dexgluco Development TODO

## Phase 1: MVP - Dexcom ONE+ Support

### 1. Project Setup
- [x] Basic Rust project with relm4/GTK4
- [ ] Add required dependencies (bluer, rusqlite, chrono, tokio, rqrr)

### 2. Constants & Configuration
- [ ] Create `src/consts.rs` with all hardcoded values
  - BLE timeout (60s)
  - Reconnect settings
  - Warmup duration (30 min)
  - Sensor duration (10 days)

### 3. QR Code Scanning
- [ ] Add `rqrr` crate for QR decoding
- [ ] Create `src/ble/dexcom/qr_parser.rs`
- [ ] Implement: load image → parse → extract (serial, PIN)
- [ ] Support image file input (file picker)

### 4. BLE Layer
- [ ] Add `bluer` crate to Cargo.toml
- [ ] Create `src/ble/mod.rs` - BLE module
- [ ] Implement `src/ble/scanner.rs` - device discovery
  - Filter: name contains "DEXCOM"
  - Timeout: 60 seconds
- [ ] Implement `src/ble/pairing.rs` - BlueZ pairing agent
  - Register agent with PIN callback
  - Auto-provide scanned PIN during pairing
- [ ] Implement `src/ble/gatt.rs` - GATT client
  - Connect to sensor
  - Discover Nordic UART service
  - Subscribe to RX characteristic (notify)

### 5. Protocol Implementation
- [ ] Create `src/ble/dexcom/mod.rs` - Dexcom protocol
- [ ] Implement `src/ble/dexcom/parser.rs` - glucose packet parsing
- [ ] Understand glucose data format from Juggluco
- [ ] Handle encryption (if applicable)

### 6. Data Layer
- [ ] Create `src/data/mod.rs` - data module
- [ ] Set up SQLite database with rusqlite
- [ ] Create `src/data/sensor.rs` - sensor storage (cached pairing info)
- [ ] Create `src/data/glucose.rs` - glucose readings storage

### 7. Multi-Sensor Architecture
- [ ] Implement HashMap-based sensor management
- [ ] Support N sensors concurrently (not just 1)
- [ ] Handle per-sensor state: NotPaired, Pairing, Warmup, Active, Expired

### 8. UI Layer (relm4/GTK4)
- [ ] Create main window layout
- [ ] Implement "Add Sensor" button → file picker flow
- [ ] Display current glucose value (large text)
- [ ] Display trend arrow (↑→↓)
- [ ] Display timestamp
- [ ] Show connection status per sensor
- [ ] Show warmup countdown (30 min)
- [ ] Add settings panel (units: mg/dL or mmol/L)

### 9. Historical Chart
- [ ] Display mini glucose graph (last 3 hours)
- [ ] Use GTK4 drawing or a chart library

## Phase 2: Future Enhancements

- [ ] Camera-based QR scanning (GtkCamera)
- [ ] Add more sensors (Libre 2/3, Dexcom G7)
- [ ] Web server for Nightscout export
- [ ] Alarm system
- [ ] Statistics (time in range, average, etc.)
- [ ] Data export functionality