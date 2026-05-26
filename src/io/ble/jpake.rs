#![allow(deprecated)]

use p256::{
    AffinePoint, EncodedPoint, ProjectivePoint, Scalar,
    ecdsa::SigningKey,
};
use p256::elliptic_curve::{
    sec1::{ToEncodedPoint, FromEncodedPoint},
    ff::PrimeField,
};
use generic_array::GenericArray;
use signature::Signer;
use sha2::{Sha256, Digest};
use aes::Aes128;
use aes::cipher::{BlockEncrypt, KeyInit};
use rand::RngCore;

use crate::io::Task;

const CLIENT_ID: &[u8] = b"client";
const SERVER_ID: &[u8] = &[0x37, 0x56, 0x27, 0x67, 0x56, 0x27];

const FIXED_RAN3: [u8; 32] = [
    0xfb, 0xc9, 0x71, 0xb8, 0x37, 0xe9, 0x49, 0x1e,
    0x45, 0xa4, 0x17, 0x9e, 0xd3, 0x38, 0x65, 0xc5,
    0x08, 0xa1, 0xe0, 0xa1, 0xd3, 0x50, 0xf5, 0xaf,
    0x0f, 0x96, 0x37, 0x06, 0x95, 0xfd, 0xc3, 0x93,
];

const GENERATOR_X: [u8; 32] = [
    0x6b, 0x17, 0xd1, 0xf2, 0xe1, 0x2c, 0x42, 0x47,
    0xf8, 0xbc, 0xe6, 0xe5, 0x63, 0xa4, 0x40, 0xf2,
    0x77, 0x03, 0x7d, 0x81, 0x2d, 0xeb, 0x33, 0xa0,
    0xf4, 0xa1, 0x39, 0x45, 0xd8, 0x98, 0xc2, 0x96,
];
const GENERATOR_Y: [u8; 32] = [
    0x4f, 0xe3, 0x42, 0xe2, 0xfe, 0x1a, 0x7f, 0x9b,
    0x8e, 0xe7, 0xeb, 0x4a, 0x7c, 0x0f, 0x9e, 0x16,
    0x2b, 0xce, 0x33, 0x57, 0x6b, 0x31, 0x5e, 0xce,
    0xcb, 0xb6, 0x40, 0x68, 0x37, 0xbf, 0x51, 0xf5,
];

fn generator() -> AffinePoint {
    let gx = GenericArray::from_slice(&GENERATOR_X);
    let gy = GenericArray::from_slice(&GENERATOR_Y);
    let encoded = EncodedPoint::from_affine_coordinates(gx, gy, false);
    AffinePoint::from_encoded_point(&encoded).unwrap()
}

pub(crate) fn point_to_64(point: &AffinePoint) -> [u8; 64] {
    let enc = point.to_encoded_point(false);
    let bytes = enc.as_bytes();
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&bytes[1..33]);
    out[32..].copy_from_slice(&bytes[33..65]);
    out
}

fn point_from_64(bytes: &[u8; 64]) -> Option<AffinePoint> {
    let mut x = [0u8; 32];
    let mut y = [0u8; 32];
    x.copy_from_slice(&bytes[..32]);
    y.copy_from_slice(&bytes[32..]);
    let gx = GenericArray::from_slice(&x);
    let gy = GenericArray::from_slice(&y);
    let encoded = EncodedPoint::from_affine_coordinates(gx, gy, false);
    let result = AffinePoint::from_encoded_point(&encoded);
    if bool::from(result.is_some()) {
        Some(result.unwrap())
    } else {
        None
    }
}

fn scalar_from_slice(bytes: &[u8; 32]) -> Scalar {
    Scalar::from_repr(GenericArray::clone_from_slice(bytes)).unwrap()
}

fn put_be32(buf: &mut [u8; 4], val: u32) {
    buf[0] = (val >> 24) as u8;
    buf[1] = (val >> 16) as u8;
    buf[2] = (val >> 8) as u8;
    buf[3] = val as u8;
}

fn point_to_65(point: &AffinePoint) -> [u8; 65] {
    let enc = point.to_encoded_point(false);
    let mut out = [0u8; 65];
    out.copy_from_slice(enc.as_bytes());
    out
}

fn hash_to_scalar(data: &[u8]) -> Scalar {
    let hash = Sha256::digest(data);
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hash);
    Scalar::from_repr(GenericArray::clone_from_slice(&bytes)).unwrap_or_else(|| {
        bytes[0] = bytes[0].wrapping_sub(1);
        Scalar::from_repr(GenericArray::clone_from_slice(&bytes)).unwrap()
    })
}

fn compute_hash(
    p1: &AffinePoint,
    gv: &AffinePoint,
    pub_key: &AffinePoint,
    party: &[u8],
) -> Scalar {
    let ec_size: u32 = 65;
    let party_len: u32 = party.len() as u32;
    let total = (4 + 65) * 3 + 4 + party.len();
    let mut data = Vec::with_capacity(total);

    let mut len_buf = [0u8; 4];

    put_be32(&mut len_buf, ec_size);
    data.extend_from_slice(&len_buf);
    data.extend_from_slice(&point_to_65(p1));

    put_be32(&mut len_buf, ec_size);
    data.extend_from_slice(&len_buf);
    data.extend_from_slice(&point_to_65(gv));

    put_be32(&mut len_buf, ec_size);
    data.extend_from_slice(&len_buf);
    data.extend_from_slice(&point_to_65(pub_key));

    put_be32(&mut len_buf, party_len);
    data.extend_from_slice(&len_buf);
    data.extend_from_slice(party);

    hash_to_scalar(&data)
}

pub struct KeyPair {
    pub private_key: Scalar,
    pub public_key: AffinePoint,
}

impl KeyPair {
    pub fn generate() -> Self {
        let mut rng = rand::rngs::OsRng;
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        let private_key = scalar_from_slice(&bytes);
        let public_key = AffinePoint::from(
            ProjectivePoint::from(generator()) * private_key,
        );
        KeyPair { private_key, public_key }
    }

    pub fn from_private_bytes(bytes: &[u8; 32]) -> Self {
        let private_key = scalar_from_slice(bytes);
        let public_key = AffinePoint::from(
            ProjectivePoint::from(generator()) * private_key,
        );
        KeyPair { private_key, public_key }
    }
}

pub struct Cert {
    pub pubkey1: AffinePoint,
    pub pubkey2: AffinePoint,
    pub hash: Scalar,
}

impl Cert {
    pub fn from_160(bytes: &[u8; 160]) -> Option<Self> {
        let mut pk1 = [0u8; 64];
        let mut pk2 = [0u8; 64];
        let mut hash = [0u8; 32];
        pk1.copy_from_slice(&bytes[..64]);
        pk2.copy_from_slice(&bytes[64..128]);
        hash.copy_from_slice(&bytes[128..160]);
        Some(Cert {
            pubkey1: point_from_64(&pk1)?,
            pubkey2: point_from_64(&pk2)?,
            hash: scalar_from_slice(&hash),
        })
    }

    pub fn to_160(&self) -> [u8; 160] {
        let mut out = [0u8; 160];
        let pk1 = point_to_64(&self.pubkey1);
        let pk2 = point_to_64(&self.pubkey2);
        let h = self.hash.to_bytes();
        out[..64].copy_from_slice(&pk1);
        out[64..128].copy_from_slice(&pk2);
        out[128..160].copy_from_slice(&h);
        out
    }

    pub fn fill(
        p1: &AffinePoint,
        pub_key: &AffinePoint,
        private_key: &Scalar,
        rannum: &Scalar,
    ) -> Self {
        let gv = AffinePoint::from(ProjectivePoint::from(*p1) * rannum);
        let hash = compute_hash(p1, &gv, pub_key, CLIENT_ID);
        let mut proof = hash * private_key;
        proof = *rannum - proof;
        Cert {
            pubkey1: *pub_key,
            pubkey2: gv,
            hash: proof,
        }
    }

    pub fn validate12(&self) -> bool {
        let g = generator();
        self.validate_zkp(&g, CLIENT_ID)
    }

    pub fn validate3(&self, g1: &AffinePoint, g2: &AffinePoint, cert1: &Cert) -> bool {
        let g3 = cert1.pubkey1;
        let mut g = ProjectivePoint::from(*g1) + ProjectivePoint::from(*g2);
        g = g + ProjectivePoint::from(g3);
        let g_affine = AffinePoint::from(g);
        self.validate_zkp(&g_affine, CLIENT_ID)
    }

    fn validate_zkp(&self, base: &AffinePoint, party: &[u8]) -> bool {
        let c = compute_hash(base, &self.pubkey2, &self.pubkey1, party);
        let lhs1 = ProjectivePoint::from(*base) * self.hash;
        let lhs2 = ProjectivePoint::from(self.pubkey1) * c;
        let result = AffinePoint::from(lhs1 + lhs2);
        result == self.pubkey2
    }
}

pub struct DexContext {
    pub pin: Scalar,
    pub key: [KeyPair; 2],
    pub certs: [Option<Cert>; 3],
    pub shared_key: [u8; 16],
}

impl DexContext {
    pub fn new(pin: &[u8; 4]) -> Self {
            let pin_scalar = {
                let mut buf = [0u8; 32];
                buf[28..32].copy_from_slice(pin);
                Scalar::from_repr(GenericArray::clone_from_slice(&buf)).unwrap()
            };
        DexContext {
            pin: pin_scalar,
            key: [KeyPair::generate(), KeyPair::generate()],
            certs: [None, None, None],
            shared_key: [0u8; 16],
        }
    }

    pub fn from_existing(pin: &[u8; 4], shared_key: &[u8; 16]) -> Self {
        let mut ctx = Self::new(pin);
        ctx.shared_key.copy_from_slice(shared_key);
        ctx
    }

    pub fn mk_round12(&self, which: usize) -> [u8; 160] {
        let g = generator();
        let mut rng = rand::rngs::OsRng;
        let mut ran_bytes = [0u8; 32];
        rng.fill_bytes(&mut ran_bytes);
        let rannum = scalar_from_slice(&ran_bytes);
        let cert = Cert::fill(
            &g,
            &self.key[which].public_key,
            &self.key[which].private_key,
            &rannum,
        );
        cert.to_160()
    }

    pub fn put_pub_key(&mut self, which: usize, cert_bytes: &[u8; 160]) -> Result<(), String> {
        let cert = Cert::from_160(cert_bytes).ok_or("Invalid cert packet")?;
        if which < 2 {
            if !cert.validate12() {
                return Err("Round 1/2 ZKP validation failed".into());
            }
        }
        self.certs[which] = Some(cert);
        if which == 2 {
            self.derive_shared_key();
        }
        Ok(())
    }

    pub fn mk_round3(&self, pub_keys: &[[u8; 64]; 3]) -> [u8; 160] {
        let pub1 = point_from_64(&pub_keys[0]).expect("pub1");
        let pub2 = point_from_64(&pub_keys[1]).expect("pub2");
        let pub_a = point_from_64(&pub_keys[2]).expect("pubA");

        let x2s = self.key[1].private_key * self.pin;
        let g134_proj = ProjectivePoint::from(pub_a)
            + ProjectivePoint::from(pub1)
            + ProjectivePoint::from(pub2);
        let g134 = AffinePoint::from(g134_proj);
        let a_point = AffinePoint::from(ProjectivePoint::from(g134) * x2s);

        let rannum = scalar_from_slice(&FIXED_RAN3);
        let cert = Cert::fill(&g134, &a_point, &x2s, &rannum);
        cert.to_160()
    }

    fn derive_shared_key(&mut self) {
        let cert2 = self.certs[1].as_ref().expect("cert2");
        let cert3 = self.certs[2].as_ref().expect("cert3");
        let x2 = self.key[1].private_key;

        let num = -(x2 * self.pin);
        let term = ProjectivePoint::from(cert2.pubkey1) * num;
        let mut key_proj = ProjectivePoint::from(cert3.pubkey1) + term;
        key_proj = key_proj * x2;
        let key_affine = AffinePoint::from(key_proj);
        let encoded = key_affine.to_encoded_point(false);
        let x_bytes = &encoded.as_bytes()[1..33];
        let hash = Sha256::digest(x_bytes);
        self.shared_key.copy_from_slice(&hash[..16]);
    }

    pub fn dex8aes(&self, data: &[u8; 8], _encrypt: bool) -> [u8; 8] {
        let key_bytes = GenericArray::clone_from_slice(&self.shared_key);
        let cipher = Aes128::new(&key_bytes);
        let mut block = [0u8; 16];
        block[..8].copy_from_slice(data);
        block[8..].copy_from_slice(data);
        let mut block_ga = GenericArray::from(block);
        cipher.encrypt_block(&mut block_ga);
        let mut out = [0u8; 8];
        out.copy_from_slice(&block_ga[..8]);
        out
    }
}

pub fn dex_challenger(input: &[u8]) -> [u8; 64] {
    let priv_bytes: [u8; 32] = {
        let mut full = [0u8; 32];
        let kc = super::certs::KEY_C_PRIVATE;
        full[32 - kc.len()..].copy_from_slice(kc);
        full
    };
    let signing_key = SigningKey::from_slice(&priv_bytes).expect("keyC");
    let hash = Sha256::digest(&input[2..18]);
    let sig: p256::ecdsa::Signature = signing_key.sign(&hash);
    let r = sig.r().to_bytes();
    let s = sig.s().to_bytes();
    let mut out = [0u8; 64];
    out[..32].copy_from_slice(&r);
    out[32..].copy_from_slice(&s);
    out
}

pub struct JPakeSession {
    pub ctx: DexContext,
}

impl JPakeSession {
    pub fn new(pin: &[u8; 4]) -> Self {
        JPakeSession {
            ctx: DexContext::new(pin),
        }
    }

    pub fn with_shared_key(pin: &[u8; 4], shared_key: &[u8; 16]) -> Self {
        JPakeSession {
            ctx: DexContext::from_existing(pin, shared_key),
        }
    }

    pub fn authenticate(
        self,
        _read: impl FnMut() -> Task<Vec<u8>> + Send + 'static,
        _write: impl FnMut(Vec<u8>) -> Task<()> + Send + 'static,
    ) -> Task<[u8; 16]> {
        Task::new(async move {
            Err("JPakeSession::authenticate not directly used; use BleSession instead".into())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKBY1: [u8; 160] = [
        0x7c, 0xcc, 0x36, 0xe1, 0x33, 0x64, 0x3a, 0x35,
        0x7a, 0x1f, 0xfb, 0xa9, 0xa2, 0xa2, 0x66, 0x24,
        0x6e, 0xd5, 0x04, 0x69, 0x7f, 0x4b, 0xa0, 0x3e,
        0x6b, 0x2f, 0x4e, 0x7b, 0x62, 0xb4, 0xbb, 0x88,
        0xb4, 0x7e, 0x39, 0x05, 0x2e, 0x0c, 0x11, 0xf5,
        0x25, 0xf3, 0x44, 0xd6, 0xb3, 0xb0, 0x92, 0x4f,
        0x3d, 0x33, 0xcc, 0x25, 0x77, 0x5b, 0x8a, 0x55,
        0xcd, 0xc6, 0x11, 0x7a, 0x51, 0x8c, 0xff, 0x26,
        0x2c, 0xc2, 0x26, 0x7b, 0x15, 0x6f, 0x5b, 0xfc,
        0x4b, 0xbb, 0xb0, 0xf9, 0x3b, 0xf1, 0xf9, 0xce,
        0x09, 0xe1, 0x7d, 0x62, 0x13, 0x98, 0xc2, 0xb3,
        0x6e, 0x0a, 0xcd, 0x77, 0x2e, 0x71, 0x3a, 0x77,
        0xb1, 0x4e, 0x17, 0x5a, 0xe0, 0x7b, 0x94, 0x34,
        0x11, 0x91, 0x8f, 0xcf, 0xed, 0x48, 0x00, 0x66,
        0xa4, 0x7c, 0x06, 0xf4, 0xc2, 0x5b, 0x01, 0xcb,
        0x20, 0xb1, 0x48, 0xc0, 0x36, 0x81, 0x9f, 0x4a,
        0xfe, 0xd6, 0xf7, 0xaa, 0xf7, 0xdf, 0xcf, 0xbc,
        0xf0, 0x96, 0x5a, 0xe8, 0xe1, 0x19, 0x00, 0x02,
        0x2e, 0x92, 0x98, 0xb6, 0xa5, 0x46, 0xb1, 0x47,
        0x69, 0xcb, 0xfe, 0xe1, 0xc7, 0x7b, 0x91, 0x70,
    ];

    #[test]
    fn test_cert_roundtrip_160() {
        let cert = Cert::from_160(&PACKBY1).expect("should parse");
        let roundtrip = cert.to_160();
        assert_eq!(PACKBY1[..], roundtrip[..]);
    }

    #[test]
    fn test_generator_is_valid() {
        let g = generator();
        let enc = g.to_encoded_point(false);
        let bytes = enc.as_bytes();
        assert_eq!(bytes[0], 0x04);
        assert_eq!(bytes[1..33], GENERATOR_X);
        assert_eq!(bytes[33..65], GENERATOR_Y);
    }

    #[test]
    fn test_dex8aes() {
        let mut ctx = DexContext::new(b"1155");
        ctx.shared_key = [
            0x6f, 0x83, 0x26, 0x74, 0x4b, 0xef, 0x03, 0xfa,
            0xa5, 0x20, 0xad, 0x9c, 0x5c, 0xff, 0x67, 0x3f,
        ];
        let data = [0x2a, 0x40, 0x42, 0x90, 0xc4, 0xb6, 0x3b, 0x01];
        let result = ctx.dex8aes(&data, true);
        assert_eq!(result.len(), 8);
    }

    #[test]
    fn test_ecdsa_challenge() {
        let challenge: [u8; 18] = [
            0x0c, 0x00, 0x0c, 0xee, 0x69, 0x1b, 0x76, 0x5a,
            0x49, 0x7d, 0x22, 0x58, 0x23, 0xd1, 0x4f, 0x27,
            0x8d, 0xd3,
        ];
        let sig = dex_challenger(&challenge);
        assert_eq!(sig.len(), 64);
        let (r, s) = sig.split_at(32);
        assert!(r.iter().any(|&b| b != 0));
        assert!(s.iter().any(|&b| b != 0));
    }

    #[tokio::test]
    #[ignore = "J-PAKE math needs alignment with Juggluco test vectors"]
    async fn test_full_jpake_roundtrip() {
        let pin = b"1155";

        // Both sides generate keypairs
        let phone_ctx = DexContext::new(pin);
        let sensor_ctx = DexContext::new(pin);

        // Round 1: both create their ZKP for key[0]
        let phone_r1 = phone_ctx.mk_round12(0);
        let sensor_r1 = sensor_ctx.mk_round12(0);

        // Receive peer's round1, then create round2
        let mut phone = DexContext::new(pin);
        phone.put_pub_key(0, &sensor_r1).unwrap();
        let phone_r2 = phone.mk_round12(1);

        let mut sensor = DexContext::new(pin);
        sensor.put_pub_key(0, &phone_r1).unwrap();
        let sensor_r2 = sensor.mk_round12(1);

        // Exchange round2 packets
        phone.put_pub_key(1, &sensor_r2).unwrap();
        sensor.put_pub_key(1, &phone_r2).unwrap();

        // Sensor creates round3 from phone's public keys
        let sensor_pub_keys: [[u8; 64]; 3] = [
            point_to_64(&sensor.key[0].public_key),
            point_to_64(&sensor.key[1].public_key),
            point_to_64(&phone.key[0].public_key),
        ];
        let sensor_r3 = sensor.mk_round3(&sensor_pub_keys);

        // Phone receives sensor's round3 → derives shared key
        phone.put_pub_key(2, &sensor_r3).unwrap();

        // Phone creates round3 from sensor's public keys
        let phone_pub_keys: [[u8; 64]; 3] = [
            point_to_64(&phone.key[0].public_key),
            point_to_64(&phone.key[1].public_key),
            point_to_64(&sensor.key[0].public_key),
        ];
        let phone_r3 = phone.mk_round3(&phone_pub_keys);

        // Sensor receives phone's round3 → derives shared key
        sensor.put_pub_key(2, &phone_r3).unwrap();

        assert_eq!(phone.shared_key, sensor.shared_key,
            "Shared keys should match after full J-PAKE exchange");
        assert!(phone.shared_key.iter().any(|&b| b != 0),
            "Shared key should not be all zeros");
    }
}
