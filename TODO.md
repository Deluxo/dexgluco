# Dexgluco Development TODO

## Phase 1: MVP - Dexcom ONE+ Support

### 1. Project Setup
- [x] Basic Rust project with relm4/GTK4
- [ ] Add required dependencies (bluer, rusqlite, chrono, tokio)

### 2. BLE Layer
- [ ] Add `bluer` crate to Cargo.toml
- [ ] Create BLE manager for device scanning
- [ ] Implement Dexcom ONE+ device discovery
- [ ] Implement BLE connection handling
- [ ] Subscribe to glucose notifications
- [ ] Parse glucose data packets

### 3. Protocol Implementation
- [ ] Understand Dexcom ONE+ BLE protocol
- [ ] Implement authentication/pairing
- [ ] Implement glucose data parsing
- [ ] Handle encryption (if applicable)

### 4. UI Layer (relm4/GTK4)
- [ ] Create main window layout
- [ ] Display current glucose value (large text)
- [ ] Display trend arrow (↑→↓)
- [ ] Display timestamp
- [ ] Show connection status
- [ ] Add settings panel (units: mg/dL or mmol/L)

### 5. Data Layer
- [ ] Set up SQLite database
- [ ] Store glucose readings
- [ ] Query historical data for charts

### 6. Historical Chart
- [ ] Display mini glucose graph (last 3 hours)
- [ ] Use GTK4 drawing or a chart library

## Phase 2: Future Enhancements

- [ ] Add more sensors (Libre 2/3, Dexcom G7)
- [ ] Web server for Nightscout export
- [ ] Alarm system
- [ ] Statistics (time in range, average, etc.)
- [ ] Data export functionality