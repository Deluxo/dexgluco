// Dexcom ONE+ BLE protocol state machine.
// Handles the authentication and data streaming phases.
//
// Auth service UUID: f8083532-849e-531c-c594-30f1f86a4ea5
// Characteristics:
//   3534 - Control
//   3535 - Authentication
//   3536 - Backfill
//   3538 - Authentication Data (for long writes/reads)
//
// This module is a stub — filled in as the J-PAKE implementation progresses.

pub enum AuthPhase {
    Idle,
    Round1Write,
    Round1Read,
    Round2Write,
    Round2Read,
    ConfirmWrite,
    Complete,
}

pub struct BleSession;

impl BleSession {
    pub fn new() -> Self {
        BleSession
    }
}
