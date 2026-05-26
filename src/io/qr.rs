use crate::io::task::Task;
use image::imageops::FilterType;
use rxing::common::{GlobalHistogramBinarizer, HybridBinarizer};
use rxing::datamatrix::DataMatrixReader;
use rxing::qrcode::QRCodeReader;
use rxing::{BinaryBitmap, DecodeHints, ImmutableReader, Luma8LuminanceSource};

pub struct ScanDataMatrix(pub String);

impl ScanDataMatrix {
    pub fn run(self) -> Task<(String, String)> {
        Task::new(async move {
            let raw = decode_image(&self.0).await?;
            extract_pairing_data(&raw)
        })
    }
}

fn extract_pairing_data(text: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = text.split('\x1d').collect();

    let serial = parts
        .first()
        .and_then(|p| {
            let si = p.find("21")?;
            Some(p[si + 2..].to_string())
        })
        .ok_or_else(|| "No serial (AI 21) in DataMatrix".to_string())?;

    let pairing_code = parts
        .get(1)
        .and_then(|p| {
            let si = p.find("240")?;
            Some(p[si + 3..].to_string())
        })
        .ok_or_else(|| "No pairing code (AI 240) in DataMatrix".to_string())?;

    if serial.is_empty() {
        return Err("Empty serial in DataMatrix".into());
    }
    if pairing_code.is_empty() {
        return Err("Empty pairing code in DataMatrix".into());
    }

    Ok((serial, pairing_code))
}

async fn decode_image(path: &str) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("Failed to open image: {}", e))?;
    let gray = img.to_luma8();
    let (orig_w, orig_h) = gray.dimensions();
    let max_dim = orig_w.max(orig_h);

    let target_sizes: Vec<u32> = if max_dim > 4000 {
        vec![max_dim / 8, max_dim / 4, max_dim / 2]
    } else {
        vec![max_dim / 4, max_dim / 2, max_dim]
    };

    for &target in &target_sizes {
        if target < 200 {
            continue;
        }

        let scale = target as f64 / max_dim as f64;
        let dw = (orig_w as f64 * scale) as u32;
        let dh = (orig_h as f64 * scale) as u32;

        let resized = if (dw, dh) == (orig_w, orig_h) {
            gray.clone()
        } else {
            image::imageops::resize(&gray, dw, dh, FilterType::Triangle)
        };

        let pixels = resized.into_raw();
        let source = Luma8LuminanceSource::new(pixels, dw, dh);

        let hints = DecodeHints::default();

        let dm_reader = DataMatrixReader;
        let qr_reader = QRCodeReader;

        let result = dm_reader
            .immutable_decode_with_hints(
                &mut BinaryBitmap::new(HybridBinarizer::new(source.clone())),
                &hints,
            )
            .or_else(|_| {
                qr_reader.immutable_decode_with_hints(
                    &mut BinaryBitmap::new(HybridBinarizer::new(source.clone())),
                    &hints,
                )
            })
            .or_else(|_| {
                dm_reader.immutable_decode_with_hints(
                    &mut BinaryBitmap::new(GlobalHistogramBinarizer::new(source.clone())),
                    &hints,
                )
            })
            .or_else(|_| {
                qr_reader.immutable_decode_with_hints(
                    &mut BinaryBitmap::new(GlobalHistogramBinarizer::new(source)),
                    &hints,
                )
            });

        if let Ok(decoded) = result {
            return Ok(decoded.getText().to_owned());
        }
    }

    Err("Decode failed after trying multiple scales".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_pairing_data_standard_format() {
        let text = "010038627000488821667529744201\x1d11251101172704302406044";
        let (serial, pin) = extract_pairing_data(text).unwrap();
        assert_eq!(serial, "667529744201");
        assert_eq!(pin, "6044");
    }

    #[test]
    fn test_extract_pairing_data_missing_serial() {
        let text = "0100386270004888\x1d2406044";
        assert!(extract_pairing_data(text).is_err());
    }

    #[test]
    fn test_extract_pairing_data_missing_pin() {
        let text = "010038627000488821667529744201\x1d1125110117270430";
        assert!(extract_pairing_data(text).is_err());
    }
}
