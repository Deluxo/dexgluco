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

## Application Protocol

The application follows this protocol:

```rust
fn main() {
    // 1. Get sensors - from storage OR add new via QR+BLE
    let sensors = get_sensors(
        get_from_storage,  // load from DB
        connect_new,       // QR → BLE → sensor
    )?;

    // 2. Connect to sensors via BT
    let connections = connect(via_bt);

    // 3. Monitor incoming readings
    monitor(connections);
}
```

### get_sensors()

Returns sensors either from storage or newly added:
- If storage has sensors: returns them
- If storage empty: runs `connect_new` (QR scan → BLE pair)
- Returns `Result<Sensor, String>` - Left is error string, Right is sensor

### connect()

Connects to sensors:
- Takes connector function (real bluer or mock)
- Handles reconnection/re-auth internally - that's implementation detail
- Returns vector of connections

### monitor()

Ongoing:
- Subscribes to glucose notifications
- Processes readings
- Handles disconnects
- Runs indefinitely

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

## References

- Juggluco repository: https://github.com/j-kaltes/Juggluco
- Dexcom ONE+ uses similar protocol to Dexcom G7
- bluer crate: https://docs.rs/bluer
- rqrr crate: https://crates.io/crates/rqrr