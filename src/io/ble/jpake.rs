// EC-JPAKE authentication for Dexcom ONE+ pairing.
// Uses mbedtls for the cryptographic handshake.
// This module is a stub — full implementation pending mbedtls crate integration.

use crate::io::Task;

pub struct JPakeSession;

impl JPakeSession {
    pub fn new(_pin: &str) -> Self {
        JPakeSession
    }

    /// Perform the full J-PAKE handshake by reading/writing auth characteristic.
    /// Takes closures for BLE I/O so this module stays pure (no bluer dep).
    pub fn authenticate(
        self,
        _read: impl Fn() -> Task<Vec<u8>> + Send + 'static,
        _write: impl Fn(Vec<u8>) -> Task<()> + Send + 'static,
    ) -> Task<()> {
        Task::new(async move {
            Err("J-PAKE not yet implemented".into())
        })
    }
}
