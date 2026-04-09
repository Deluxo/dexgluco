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

## Pairing/Authentication

- Requires pairing code from sensor packaging (found on sensor or in box)
- Pairing code is typically 6-10 digits

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

- Juggluco source: https://github.com/j-kaltes/Juggluco
- Similar to Dexcom G7 protocol
- bluer crate: https://docs.rs/bluer