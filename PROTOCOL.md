# Dexcom ONE+ Protocol Documentation

> Derived from reverse-engineering of Juggluco, DiaBLE (Swift), and xDrip+ keks.
> Dexcom ONE+ is identical to Dexcom G7 at the protocol level.

## DataMatrix Code

The Dexcom ONE+ applicator has a **DataMatrix** code (not QR). It uses a GS1 format:

```
Prefix: 0100000000000001210000000000011126010117370101240
Code:   [prefix][4-char alphanumeric pairing code]
```

Example: `0100000000000001210000000000011126010117370101240ABCD`

The 4-character pairing code is also printed on the applicator as a fallback for manual entry.

**Note**: The pairing code is alphanumeric (A-Z, 0-9), not just digits. It is the shared secret for EC-JPAKE authentication.

## BLE Service

The Dexcom ONE+ does NOT use the Nordic UART Service. It uses a custom service:

| Attribute | Value |
|-----------|-------|
| **Service UUID** | `f8083532-849e-531c-c594-30f1f86a4ea5` |
| **Device name prefix** | `DXCM` (sensor serial follows) |

### Characteristics

| ID | UUID | Property | Purpose |
|----|------|----------|---------|
| 3534 | `f8083534-849e-531c-c594-30f1f86a4ea5` | Write, Notify | **Control** — glucose data, backfill commands, transmitter info |
| 3535 | `f8083535-849e-531c-c594-30f1f86a4ea5` | Write, Notify | **Authentication** — opcode commands, status replies |
| 3536 | `f8083536-849e-531c-c594-30f1f86a4ea5` | Notify | **Backfill** — raw backfill packets (20-byte chunks) |
| 3538 | `f8083538-849e-531c-c594-30f1f86a4ea5` | Write, Notify | **Authentication Data** — J-PAKE payloads, certificate data (20-byte MTU chunks) |

**Note on 20-byte MTU**: The authentication data characteristic (3538) exchanges data in 20-byte chunks, which is the BLE ATT MTU minus 3 bytes of ATT header. Larger payloads are split across multiple notifications/writes.

## Device Discovery

- BLE advertisement contains service UUID `f8083532-849e-531c-c594-30f1f86a4ea5`
- Device name format: `DXCM` followed by sensor serial number
- Example name: `DXCM123456`
- The official Dexcom app initiates bonding; this app can initiate bonding via J-PAKE

## Authentication Protocol

The Dexcom ONE+ uses **EC-JPAKE** (Elliptic Curve Password-Authenticated Key Exchange by Juggling) for pairing, followed by certificate exchange and proof of possession.

### Auth Opcodes

| Opcode | Name | Direction | Description |
|--------|------|-----------|-------------|
| `0x01` | TxIdChallenge | Phone→Sensor | Phone identifies itself |
| `0x02` | AppKeyChallenge | Phone→Sensor | Challenge with app key |
| `0x03` | ChallengeReply | Sensor→Phone | Response to challenge |
| `0x04` | HashFromDisplay | Phone→Sensor | Hash of pairing code |
| `0x05` | StatusReply | Sensor→Phone | Auth status result |
| `0x06` | KeepAlive | Phone→Sensor | Keep connection alive |
| `0x07` | BondRequest | Phone→Sensor | Request BLE bonding |
| `0x08` | BondResponse | Sensor→Phone | Bonding accepted |
| `0x0A` | PakeExchange | Both | EC-JPAKE round exchange |
| `0x0B` | CertificateExchange | Both | Certificate chain exchange |
| `0x0C` | ProofOfPossession | Both | Sign challenge with derived key |

### Auth State Machine

The authentication flows through these phases in order:

```
                    ┌──────────────┐
                    │    Init      │
                    └──────┬───────┘
                           ▼
                    ┌──────────────┐
                    │ PakeRound0   │  0x0A 00 — start J-PAKE
                    └──────┬───────┘
                           ▼
                    ┌──────────────┐
                    │ PakeRound1   │  0x0A 01 — EC-JPAKE write/read round 1
                    └──────┬───────┘
                           ▼
                    ┌──────────────┐
                    │ PakeRound2   │  0x0A 02 — EC-JPAKE write/read round 2
                    └──────┬───────┘           derive shared secret
                           ▼
                    ┌──────────────┐
                    │ Challenge    │  0x02 → 0x03 → 0x04 → 0x05
                    └──────┬───────┘           verify pairing code hash
                           ▼
                    ┌──────────────┐
                    │ CertExchange │  0x0B 00 → 01 → 02
                    └──────┬───────┘           exchange certificate chains
                           ▼
                    ┌──────────────┐
                    │ ProofOfPoss  │  0x0C — sign challenge with derived key
                    └──────┬───────┘
                           ▼
                    ┌──────────────┐
                    │ KeepAlive    │  0x06
                    └──────┬───────┘
                           ▼
                    ┌──────────────┐
                    │ BondRequest  │  0x07 → 0x08
                    └──────┬───────┘
                           ▼
                    ┌──────────────┐
                    │ Authenticated │  Ready for data exchange
                    └──────────────┘
```

### Already-Bonded Fast Path

If the sensor has been previously bonded (BLE bond exists and crypto keys are stored):

```
1. Write 3535: [0x01, 0x00]                    TxIdChallenge
2. Write 3535: [0x02, <8 bytes>, 0x02]         AppKeyChallenge
3. Notify 3535: [0x03, <16 bytes>]             ChallengeReply
4. Write 3535: [0x04, <8 bytes>]               HashFromDisplay
5. Notify 3535: [0x05, 0x01, 0x01]             StatusReply (authenticated)
→ Proceed to data exchange (0x4E glucose request)
```

### Full Pairing (J-PAKE + Certificate Exchange)

#### Phase 1: EC-JPAKE (Opcode 0x0A)

Enable notifications on 3535 and 3538 before starting.

```
Step 0: Write 3535  [0x0A, 0x00]               Start J-PAKE exchange

  Sensor → Notify 3538: 6 packets × 20 bytes   Sensor's round 1
  Sensor → Notify 3535: [0x0A, 0x00, 0x00]     Ack
  Sensor → Notify 3538: 2 packets × 20 bytes   Round 1 continuation
  Phone  → Write 3538:  8 packets × 20 bytes   Our round 1 (mbedtls_ecjpake_write_round_one)

Step 1: Write 3535  [0x0A, 0x01]               Continue J-PAKE

  Sensor → Notify 3538: 6 packets × 20 bytes   Sensor's round 2
  Sensor → Notify 3535: [0x0A, 0x00, 0x01]     Ack
  Sensor → Notify 3538: 2 packets × 20 bytes   Round 2 continuation
  Phone  → Write 3538:  8 packets × 20 bytes   Our round 2 (mbedtls_ecjpake_write_round_two)

Step 2: Write 3535  [0x0A, 0x02]               Finalize J-PAKE

  Sensor → Notify 3538: 6 packets × 20 bytes
  Sensor → Notify 3535: [0x0A, 0x00, 0x02]     Ack
  Sensor → Notify 3538: 2 packets × 20 bytes
  Phone  → Write 3538:  8 packets × 20 bytes   Derive shared secret
```

#### Phase 2: Authentication Challenge

```
Phone  → Write 3535: [0x02, <8 bytes>, 0x02]   AppKeyChallenge
Sensor → Notify 3535: [0x03, <16 bytes>]        ChallengeReply
Phone  → Write 3535: [0x04, <8 bytes>]          HashFromDisplay
Sensor → Notify 3535: [0x05, 0x01, 0x02]        StatusReply (0x02 = new pairing)
```

Status reply codes:
- `0x01` — Already authenticated (fast path)
- `0x02` — New pairing needed (proceed to certificate exchange)

#### Phase 3: Certificate Exchange (Opcode 0x0B)

```
Phone → Write 3535: [0x0B, 0x00, <4 bytes>]    Start cert exchange phase 0
Sensor → Notify 3538: 6 packets × 20 bytes
Sensor → Notify 3535: [0x0B, 0x00, 0x00, <4 bytes>]
Sensor → Notify 3538: 18 packets + 12 bytes
Phone  → Write 3538: 24 packets + 14 bytes     Our certs

Phone → Write 3535: [0x0B, 0x01, <4 bytes>]    Phase 1
Sensor → Notify 3538: 6 packets × 20 bytes
Sensor → Notify 3535: [0x0B, 0x00, 0x01, <4 bytes>]
Sensor → Notify 3538: 16 packets + 17/18 bytes
Phone  → Write 3538: 23/22 packets + 6 bytes

Phone → Write 3535: [0x0B, 0x02, 0x00, 0x00, 0x00, 0x00]  Phase 2
Sensor → Notify 3535: [0x0B, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00]
```

Certificate data is embedded from the xDrip+ keks project / Juggluco native library.

#### Phase 4: Proof of Possession (Opcode 0x0C)

```
Phone  → Write 3535: [0x0C, <16 bytes>]        Challenge
Sensor → Notify 3538: 3 packets × 20 bytes + 4 bytes
Sensor → Notify 3535: [0x0C, 0x00, <16 bytes>] Response
Phone  → Write 3538: 3 packets × 20 bytes + 4 bytes  Our signature
```

#### Phase 5: Keep Alive + Bond

```
Phone  → Write 3535: [0x06, <interval>]         Keep alive (e.g. 0x19 = 25)
Sensor → Notify 3535: [0x06, 0x00]              Ack
Phone  → Write 3535: [0x07]                     Bond request
Sensor → Notify 3535: [0x07, 0x00]              Ack
Sensor → Notify 3535: [0x08, 0x01]              Bond established
```

## Data Exchange (Post-Authentication)

Once authenticated, enable notifications on 3534 (control characteristic):

```
Phone  → Write 3534: [0x4E]                     Request EGV (glucose)
Sensor → Notify 3534: [0x4E, <18 bytes>]        Glucose reading

Phone  → Write 3534: [0x32]                     Request calibration bounds
Sensor → Notify 3534: [0x32, <19 bytes>]        Calibration data

Phone  → Write 3534: [0xEA, 0x00]               BLE whitelist control
Sensor → Notify 3534: [0xEA, <16 bytes>]

Phone  → Write 3534: [0x59, <8 bytes>]          Backfill request (start-end time)
Sensor → Notify 3536: 9-byte packets             Backfill data
Sensor → Notify 3534: [0x59, <18 bytes>]        Backfill done

Phone  → Write 3534: [0x51, <9 bytes>]          Diagnostic data
Sensor → Notify 3536: 20-byte packets
Sensor → Notify 3534: [0x51, <16 bytes>]
```

### Control Opcodes

| Opcode | Name | Description |
|--------|------|-------------|
| `0x22` | BatteryStatus | Get battery level |
| `0x28` | StopSession | End current sensor session |
| `0x32` | CalibrationBounds | Get calibration parameters |
| `0x34` | Calibrate | Send calibration value |
| `0x38` | EncryptionInfo | Get encryption parameters |
| `0x4A` | TransmitterVersion | Get firmware version |
| `0x4E` | EGV | Request glucose reading |
| `0x51` | DiagnosticData | Get diagnostic info |
| `0x52` | TransmitterVersionExt | Extended version info |
| `0x59` | Backfill | Request historical data |
| `0xEA` | BleControl | BLE parameter control |
| `0x0F` | EncryptionStatus | Encryption status |

## Glucose Reading Format

### EGV Packet (0x4E response, 18 bytes)

```
Byte 0:     Opcode (0x4E)
Byte 1:     Status (0x00 = ok)
Byte 2-3:   Glucose in mg/dL (little-endian)
  Special: 0xFFFF = low (< 40 mg/dL), 0xFFFE = high (> 400 mg/dL)
Byte 4:     Trend arrow (0=None, 1=RisingQuick, 2=Rising, 3=Steady, 4=Falling, 5=FallingQuick)
Byte 5-10:  Reserved / internal
Byte 11-12: System time (minutes since sensor start, LE)
Byte 13-17: Reserved
```

### Trend Values

| Value | Meaning |
|-------|---------|
| 0     | None / not computable |
| 1     | Rising quickly (> 3 mg/dL/min) |
| 2     | Rising (1-3 mg/dL/min) |
| 3     | Steady (-1 to 1 mg/dL/min) |
| 4     | Falling (-1 to -3 mg/dL/min) |
| 5     | Falling quickly (< -3 mg/dL/min) |

### Unit Conversion

- All glucose values are transmitted in **mg/dL**
- Conversion to mmol/L: `mmol/L = mg/dL / 18.0182`

## Sensor States

| State | Description |
|-------|-------------|
| NotPaired | Sensor info scanned but not yet paired |
| Pairing | J-PAKE handshake in progress |
| Warmup | 30-minute warmup period (sensor stabilizing) |
| Active | Receiving glucose readings every 5 minutes |
| Expired | Sensor session ended (10 day lifespan) |

## Warmup Time

- **Dexcom ONE+**: 30 minutes
- No readings are available during warmup

## EC-JPAKE Implementation Details

The J-PAKE protocol is implemented using **mbedtls EC-JPAKE**:

```
Setup:
  role    = MBEDTLS_ECJPAKE_CLIENT
  hash    = MBEDTLS_MD_SHA256
  curve   = MBEDTLS_ECP_DP_SECP256R1
  secret  = pairing_code bytes (ASCII)
  
Round 1: 
  mbedtls_ecjpake_write_round_one(ctx, buf, len, &olen, f_rng, p_rng)
  mbedtls_ecjpake_read_round_one(ctx, buf, len)

Round 2:
  mbedtls_ecjpake_write_round_two(ctx, buf, len, &olen, f_rng, p_rng)
  mbedtls_ecjpake_read_round_two(ctx, buf, len)

Derive:
  mbedtls_ecjpake_derive_secret(ctx, buf, len, &olen, f_rng, p_rng)
  mbedtls_ecjpake_write_shared_key(ctx, buf, len, &olen, f_rng, p_rng)
```

The pairing code (ASCII) is passed directly as the pre-shared secret. The RNG function is mbedtls's CTR_DRBG seeded from the OS entropy source.

## References

- Juggluco repository: https://github.com/j-kaltes/Juggluco
- DiaBLE (Swift): https://github.com/gui-dos/DiaBLE
- xDrip+ keks (C): https://github.com/NightscoutFoundation/xDrip/tree/master/libkeks
- LoopKit G7SensorKit: https://github.com/LoopKit/G7SensorKit
- mbedtls EC-JPAKE docs: https://mbed-tls.readthedocs.io/projects/api/en/v3.6.4/api/file/ecjpake_8h/
- RFC 8236 (J-PAKE): https://datatracker.ietf.org/doc/html/rfc8236
- bluer crate: https://docs.rs/bluer
- rxing crate: https://docs.rs/rxing
