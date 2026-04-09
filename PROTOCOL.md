# Dexcom ONE+ Protocol Documentation

## BLE Service

### Nordic UART Service (NUS)
- **Service UUID**: `6E400001-B5A3-F393-E0A9-E50E24DCCA9E`

### Characteristics
| Name | UUID | Properties |
|------|------|------------|
| TX | `6E400002-B5A3-F393-E0A9-E50E24DCCA9E` | Write |
| RX | `6E400003-B5A3-F393-E0A9-E50E24DCCA9E` | Notify |

## Device Discovery

- Device name pattern: Contains "DEXCOM" or sensor serial number
- Advertisement includes the NUS service UUID

## Pairing Flow

### Step 1: QR Code Scanning
The data matrix on Dexcom ONE+ applicator packaging contains:
- **Sensor serial number** - Unique identifier
- **Pairing PIN** - 6-digit code for BLE bonding

**Library**: `rqrr` (pure Rust QR decoder)

**Input Methods**:
- MVP: Image file (file picker)
- Production: Camera via GTK4

### Step 2: BLE Pairing
1. App starts BLE scanning (filter: DEXCOM name)
2. When sensor found, registers as BlueZ pairing agent
3. When pairing request arrives, provides PIN automatically via agent
4. No user interaction needed for PIN entry

### Step 3: Connection
1. Connect to sensor (GATT)
2. Discover Nordic UART service
3. Subscribe to RX characteristic notifications
4. Wait for glucose data

## Sensor States

| State | Description |
|-------|-------------|
| NotPaired | Sensor info scanned but not paired |
| Pairing | In process of BLE bonding |
| Warmup | 30-minute warmup period (sensor starting up) |
| Active | Receiving glucose readings |
| Expired | Sensor session ended (10 days) |

## Warmup Time

- **Dexcom ONE+**: 30 minutes
- **Dexcom G7 (10-day)**: 30 minutes  
- **Dexcom G7 (15-day)**: 60 minutes

The warmup is managed internally by the sensor. The app waits for glucose data to arrive.

## Data Format

### Glucose Packet Structure
```
[To be documented as implementation progresses]
```

### Glucose Value Encoding
- Glucose values are transmitted in mg/dL internally
- Conversion to mmol/L: `mmol/L = mg/dL / 18.0182`

### Trend Values
| Value | Meaning |
|-------|---------|
| 0     | None    |
| 1     | Rising quickly |
| 2     | Rising |
| 3     | Steady |
| 4     | Falling |
| 5     | Falling quickly |

## Implementation Files

| Component | File | Description |
|-----------|------|-------------|
| QR Parser | `src/ble/dexcom/qr_parser.rs` | Parse QR image → (serial, PIN) |
| BLE Scanner | `src/ble/scanner.rs` | Device discovery |
| Pairing Agent | `src/ble/pairing.rs` | BlueZ agent with PIN |
| GATT Client | `src/ble/gatt.rs` | Connect, subscribe |
| Glucose Parser | `src/ble/dexcom/parser.rs` | Decode glucose packets |

## References

- Juggluco source: https://github.com/j-kaltes/Juggluco
- Similar to Dexcom G7 protocol
- bluer crate: https://docs.rs/bluer
- rqrr crate: https://crates.io/crates/rqrr