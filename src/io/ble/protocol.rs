use std::pin::Pin;
use std::time::Duration;
use bluer::{Device, Uuid, gatt::remote::Characteristic};
use futures::StreamExt;
use rand::RngCore;

use crate::core::GlucoseReading;

use super::jpake::{self, DexContext};
use super::certs;

const DEXCOM_SERVICE: &str = "f8083532-849e-531c-c594-30f1f86a4ea5";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Init = -1,
    Round1 = 1,
    Round2 = 2,
    Round3 = 3,
    RequestAuth = 4,
    ChallengeReply = 5,
    SendCertificate1 = 7,
    SendCertificate2 = 8,
    SendKeyChallenge = 9,
    SendKeyChallengeOut = 10,
    GetData = 11,
}

#[allow(dead_code)]
pub struct BleSession {
    device: Device,
    ctx: DexContext,
    pub control_char: Option<Characteristic>,
    auth_char: Option<Characteristic>,
    cert_char: Option<Characteristic>,
    phase: Phase,
    cert_in_buf: Vec<u8>,
    packet_to_send: Vec<u8>,
    start_packet: usize,
    bonded: bool,
    new_certificates: bool,
    random8: [u8; 8],
    _control_notify: Option<Pin<Box<dyn futures::Stream<Item = Vec<u8>> + Send + Sync>>>,
}

impl BleSession {
    pub fn new(device: Device, pin: &[u8; 4], shared_key: Option<&[u8; 16]>) -> Self {
        let ctx = match shared_key {
            Some(k) => DexContext::from_existing(pin, k),
            None => DexContext::new(pin),
        };
        BleSession {
            device,
            ctx,
            control_char: None,
            auth_char: None,
            cert_char: None,
            phase: Phase::Init,
            cert_in_buf: Vec::new(),
            packet_to_send: Vec::new(),
            start_packet: 0,
            bonded: shared_key.is_some(),
            new_certificates: false,
            random8: [0u8; 8],
            _control_notify: None,
        }
    }

    pub async fn authenticate(&mut self) -> Result<[u8; 16], String> {
        self.discover_chars().await?;

        let cert_str = self.cert_char.as_ref().unwrap()
            .notify()
            .await
            .map_err(|e| format!("Cert notify: {}", e))?;
        let mut cert_stream = std::pin::pin!(cert_str);

        let auth_str = self.auth_char.as_ref().unwrap()
            .notify()
            .await
            .map_err(|e| format!("Auth notify: {}", e))?;
        let mut auth_stream = std::pin::pin!(auth_str);

        if self.bonded {
            self.phase = Phase::RequestAuth;
            self.do_request_auth().await?;
            loop {
                tokio::select! {
                    Some(data) = auth_stream.next() => {
                        self.handle_auth_notify(&data).await?;
                        if self.phase == Phase::GetData {
                            break;
                        }
                    }
                    Some(_) = cert_stream.next() => {}
                    else => break,
                }
            }
        } else {
            self.phase = Phase::Round1;
            self.cert_in_buf.clear();
            self.write_auth(&[0x0A, 0x00]).await?;

            loop {
                tokio::select! {
                    Some(data) = cert_stream.next() => {
                        self.handle_cert_notify(&data).await?;
                        if self.phase == Phase::GetData {
                            break;
                        }
                    }
                    Some(data) = auth_stream.next() => {
                        self.handle_auth_notify(&data).await?;
                        if self.phase == Phase::GetData {
                            break;
                        }
                    }
                    else => break,
                }
            }
        }

        debug_assert!(self.ctx.shared_key.iter().any(|&b| b != 0));
        Ok(self.ctx.shared_key)
    }

    async fn discover_chars(&mut self) -> Result<(), String> {
        let dexcom_uuid: Uuid = DEXCOM_SERVICE.parse().map_err(|_| "Bad UUID".to_string())?;
        let services = self.device.services().await
            .map_err(|e| format!("Get services: {}", e))?;

        for svc in &services {
            let svc_uuid: Uuid = svc.uuid().await
                .map_err(|e| format!("Service UUID: {}", e))?;
            if svc_uuid == dexcom_uuid {
                let chars = svc.characteristics().await
                    .map_err(|e| format!("Get chars: {}", e))?;
                for char in chars {
                    let char_uuid: Uuid = char.uuid().await
                        .map_err(|e| format!("Char UUID: {}", e))?;
                    let uuid_str = char_uuid.to_string();
                    if uuid_str.starts_with("f8083534") {
                        self.control_char = Some(char);
                    } else if uuid_str.starts_with("f8083535") {
                        self.auth_char = Some(char);
                    } else if uuid_str.starts_with("f8083538") {
                        self.cert_char = Some(char);
                    }
                }
                break;
            }
        }

        if self.auth_char.is_none() || self.cert_char.is_none() {
            return Err("Required characteristics not found".into());
        }
        Ok(())
    }

    async fn write_auth(&self, data: &[u8]) -> Result<(), String> {
        self.auth_char.as_ref().unwrap()
            .write(data)
            .await
            .map_err(|e| format!("Auth write: {}", e))
    }

    async fn write_cert(&self, data: &[u8]) -> Result<(), String> {
        self.cert_char.as_ref().unwrap()
            .write(data)
            .await
            .map_err(|e| format!("Cert write: {}", e))
    }

    async fn write_control(&self, data: &[u8]) -> Result<(), String> {
        if let Some(ref ctrl) = self.control_char {
            ctrl.write(data)
                .await
                .map_err(|e| format!("Control write: {}", e))
        } else {
            Ok(())
        }
    }

    async fn send_packet(&mut self, data: &[u8]) -> Result<(), String> {
        let chunks = data.chunks(20);
        for chunk in chunks {
            tokio::time::sleep(Duration::from_millis(40)).await;
            self.write_cert(chunk).await?;
        }
        Ok(())
    }

    async fn handle_cert_notify(&mut self, data: &[u8]) -> Result<(), String> {
        self.cert_in_buf.extend_from_slice(data);

        match self.phase {
            Phase::Round1 | Phase::Round2 | Phase::Round3 => {
                if self.cert_in_buf.len() >= 160 {
                    let mut cert_bytes = [0u8; 160];
                    cert_bytes.copy_from_slice(&self.cert_in_buf[..160]);
                    self.cert_in_buf.clear();

                    let from_round = self.phase as i32 - Phase::Round1 as i32;
                    self.ctx.put_pub_key(from_round as usize, &cert_bytes)
                        .map_err(|e| format!("J-PAKE round {}: {}", from_round, e))?;

                    if self.phase == Phase::Round3 {
                        let pub_keys: [[u8; 64]; 3] = [
                            jpake::point_to_64(&self.ctx.certs[0].as_ref().unwrap().pubkey1),
                            jpake::point_to_64(&self.ctx.certs[1].as_ref().unwrap().pubkey1),
                            jpake::point_to_64(&self.ctx.key[0].public_key),
                        ];
                        let our_r3 = self.ctx.mk_round3(&pub_keys);
                        self.send_packet(&our_r3).await?;
                        self.phase = Phase::RequestAuth;
                        self.do_request_auth().await?;
                    } else {
                        let our_round = self.ctx.mk_round12(from_round as usize);
                        self.send_packet(&our_round).await?;

                        let next_val = match self.phase {
                            Phase::Round1 => Phase::Round2,
                            Phase::Round2 => Phase::Round3,
                            _ => unreachable!(),
                        };
                        self.phase = next_val;
                        self.cert_in_buf.clear();
                        tokio::time::sleep(Duration::from_millis(40)).await;
                        let cmd_byte = (self.phase as i32 - Phase::Round1 as i32) as u8;
                        self.write_auth(&[0x0A, cmd_byte]).await?;
                    }
                }
            }
            Phase::SendCertificate1 | Phase::SendCertificate2 => {
                let idx = match self.phase {
                    Phase::SendCertificate1 => 0,
                    Phase::SendCertificate2 => 1,
                    _ => 0,
                };
                let our_cert = if idx == 0 { certs::KEKS_P1 } else { certs::KEKS_P2 };
                if self.cert_in_buf.len() >= our_cert.len() {
                    self.send_packet(our_cert).await?;
                    self.cert_in_buf.clear();
                    match idx {
                        0 => {
                            self.phase = Phase::SendCertificate2;
                            self.ask_certificate().await?;
                        }
                        1 => {
                            self.phase = Phase::SendKeyChallenge;
                            self.do_key_challenge().await?;
                        }
                        _ => {}
                    }
                }
            }
            Phase::SendKeyChallenge => {
                if self.cert_in_buf.len() >= 64 {
                    self.cert_in_buf.clear();
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_auth_notify(&mut self, data: &[u8]) -> Result<(), String> {
        if data.is_empty() {
            return Ok(());
        }

        match self.phase {
            Phase::RequestAuth => {
                let aes = self.ctx.dex8aes(&self.random8, true);
                let sensor_resp = if data.len() > 1 { &data[1..data.len().min(9)] } else { &[] };
                let verified = sensor_resp.len() >= 8 && &aes[..8] == &sensor_resp[..8];
                if !verified {
                    return Err("AES challenge verification failed".into());
                }
                self.phase = Phase::ChallengeReply;
                let mut reply = vec![0u8; 10];
                let aes_out = {
                    let mut d = [0u8; 8];
                    let copy_len = (data.len() - 1).min(8);
                    d[..copy_len].copy_from_slice(&data[1..1 + copy_len]);
                    self.ctx.dex8aes(&d, false)
                };
                reply[0] = 0x04;
                reply[1..9].copy_from_slice(&aes_out);
                reply[9] = 0x00;
                self.write_auth(&reply[..9]).await?;
            }
            Phase::ChallengeReply => {
                if data.len() >= 3 && data[0] == 0x05 {
                    let bond = data[2];
                    if bond == 3 {
                        return Err("Sensor rejected with bond==3".into());
                    }
                    if data[1] == 1 && (bond == 1 || (self.bonded && bond == 2)) {
                        self.phase = Phase::GetData;
                        self.do_get_data().await?;
                    } else {
                        self.phase = Phase::SendCertificate1;
                        self.new_certificates = true;
                        self.ask_certificate().await?;
                    }
                }
            }
            Phase::SendCertificate1 | Phase::SendCertificate2 => {}
            Phase::SendKeyChallenge => {
                if data[0] == 0x0C {
                    let sig = jpake::dex_challenger(data);
                    self.send_packet(&sig).await?;
                    self.phase = Phase::SendKeyChallengeOut;
                    self.write_auth(&[0x0d, 0x00, 0x02]).await?;
                }
            }
            Phase::SendKeyChallengeOut => {
                self.phase = Phase::GetData;
                self.write_auth(&[0x06, 0x19]).await?;
            }
            _ => {
                if data.len() >= 2 && data[0] == 0x06 && data[1] == 0x01 {
                    self.do_get_data().await?;
                }
            }
        }
        Ok(())
    }

    async fn do_request_auth(&mut self) -> Result<(), String> {
        let mut buf = [0u8; 10];
        let mut rng = rand::rngs::OsRng;
        rng.fill_bytes(&mut self.random8);
        buf[0] = 0x02;
        buf[1..9].copy_from_slice(&self.random8);
        buf[9] = 0x02;
        self.write_auth(&buf).await
    }

    async fn ask_certificate(&mut self) -> Result<(), String> {
        let idx = match self.phase {
            Phase::SendCertificate1 => 0,
            Phase::SendCertificate2 => 1,
            _ => return Err("Bad cert phase".into()),
        };
        let cert_data = if idx == 0 { certs::KEKS_P1 } else { certs::KEKS_P2 };
        self.cert_in_buf.clear();
        let mut cmd = vec![0x0B, idx as u8];
        cmd.extend_from_slice(&(cert_data.len() as u32).to_le_bytes());
        self.write_auth(&cmd).await
    }

    async fn do_key_challenge(&mut self) -> Result<(), String> {
        let mut buf = vec![0u8; 17];
        let mut rng = rand::rngs::OsRng;
        rng.fill_bytes(&mut buf[1..17]);
        buf[0] = 0x0C;
        self.cert_in_buf.clear();
        self.write_auth(&buf).await
    }

    async fn do_get_data(&mut self) -> Result<(), String> {
        self.phase = Phase::GetData;
        if let Some(ref ctrl) = self.control_char {
            let stream = ctrl.notify()
                .await
                .map_err(|e| format!("Control notify: {}", e))?;
            self._control_notify = Some(Box::pin(stream));
        }
        self.write_control(&[0x4E]).await
    }

    pub async fn read_glucose(
        &self,
        control_stream: &mut (impl futures::Stream<Item = Vec<u8>> + Unpin),
    ) -> Result<GlucoseReading, String> {
        tokio::time::timeout(Duration::from_secs(30), async {
            match control_stream.next().await {
                Some(data) => parse_egv(&data),
                None => Err("Control stream ended".into()),
            }
        })
        .await
        .map_err(|_| "Timeout waiting for glucose reading".to_string())?
    }
}

fn parse_egv(data: &[u8]) -> Result<GlucoseReading, String> {
    if data.len() < 19 || data[0] != 0x4E {
        return Err(format!("Invalid EGV packet: {:02x?}", data));
    }
    let raw_glucose = u16::from_le_bytes([data[2], data[3]]);
    let value = match raw_glucose {
        0xFFFE => 401.0,
        0xFFFF => 39.0,
        _ if raw_glucose > 400 => 400.0,
        _ if raw_glucose < 40 => 40.0,
        _ => raw_glucose as f32,
    };
    let trend = data[4] as i32;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    Ok(GlucoseReading { value, timestamp, trend })
}

impl std::fmt::Debug for BleSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BleSession")
            .field("phase", &self.phase)
            .field("bonded", &self.bonded)
            .finish()
    }
}
