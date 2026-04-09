# Dexgluco Design Document

## UI Design

### Main Window Layout

```
┌─────────────────────────────────────────┐
│  Dexgluco                        [─][□][×] │
├─────────────────────────────────────────┤
│                                         │
│            ┌─────────────┐              │
│            │   142      │  mg/dL       │
│            │     ↑      │              │
│            └─────────────┘              │
│                                         │
│         10:30 AM                        │
│                                         │
│  ┌────────────────────────────────┐    │
│  │  ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄  │    │
│  │  Graph (last 3 hours)         │    │
│  └────────────────────────────────┘    │
│                                         │
│  Status: Connected                      │
│  Sensor: DEXCOM123456                  │
│                                         │
│  [Scan] [Settings]                     │
└─────────────────────────────────────────┘
```

### Components

1. **Glucose Display**
   - Large numerical value (current glucose)
   - Unit label (mg/dL or mmol/L)
   - Trend arrow indicator
   - Timestamp of last reading

2. **Chart**
   - Line chart showing glucose over time
   - X-axis: time (last 3 hours)
   - Y-axis: glucose level
   - Target range shading (optional)

3. **Status Bar**
   - Connection status (Connected/Scanning/Disconnected)
   - Sensor identifier
   - Battery level (if available)

4. **Action Buttons**
   - Scan for sensors
   - Settings

### Color Scheme

- **Normal**: Green/Blue
- **High**: Orange/Red
- **Low**: Red
- **Background**: Dark theme preferred for battery savings on laptops

## Architecture

### Modules

```
src/
├── main.rs           # Entry point
├── lib.rs            # Library root
├── ble/
│   ├── mod.rs        # BLE module
│   ├── manager.rs    # BLE device management
│   ├── dexcom.rs     # Dexcom ONE+ protocol
│   └── gatt.rs       # GATT characteristic handling
├── data/
│   ├── mod.rs        # Data module
│   ├── glucose.rs    # Glucose reading struct
│   └── storage.rs    # SQLite storage
├── ui/
│   ├── mod.rs        # UI module
│   ├── app.rs        # Main application
│   ├── widgets.rs    # Custom widgets
│   └── chart.rs      # Glucose chart
└── config.rs         # Configuration
```

### Data Structures

```rust
struct GlucoseReading {
    value: f32,           // mg/dL or mmol/L
    timestamp: DateTime,
    trend: Trend,
    sensor_id: String,
}

enum Trend {
    None,
    RisingQuickly,
    Rising,
    Steady,
    Falling,
    FallingQuickly,
}
```

## Settings

- **Units**: mg/dL or mmol/L
- **Target Range**: Low/High thresholds for alarms
- **Alarms**: Enable/disable sound notifications
- **Data Export**: Format (CSV, JSON, etc.)