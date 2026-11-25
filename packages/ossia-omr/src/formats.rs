use crate::Result;
use ossia_types::{AppError, OmrSourceFormat};
use std::collections::HashSet;

const MAX_IMAGE_PIXELS: u64 = 100_000_000;
const MAX_IMAGE_EDGE: u32 = 20_000;

pub(crate) fn validate(
    format: OmrSourceFormat,
    filename: Option<&str>,
    content_type: Option<&str>,
    bytes: &[u8],
) -> Result<()> {
    if let Some(filename) = filename.filter(|name| !name.is_empty()) {
        if let Some(extension) = filename.rsplit_once('.').map(|(_, extension)| extension) {
            let actual = source_from_extension(extension)
                .ok_or_else(|| invalid("unsupported OMR file extension"))?;
            if actual != format {
                return Err(invalid("OMR file extension does not match sourceFormat"));
            }
        }
    }
    if let Some(content_type) = content_type {
        let mime = content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let expected = mime_for(format);
        if mime != expected && !(format == OmrSourceFormat::Jpeg && mime == "image/jpg") {
            return Err(invalid("OMR MIME type does not match sourceFormat"));
        }
    }
    match format {
        OmrSourceFormat::Pdf => validate_pdf(bytes),
        OmrSourceFormat::Png => validate_png(bytes),
        OmrSourceFormat::Jpeg => validate_jpeg(bytes),
        OmrSourceFormat::Tiff => validate_tiff(bytes),
    }
}

pub(crate) fn mime_for(format: OmrSourceFormat) -> &'static str {
    match format {
        OmrSourceFormat::Pdf => "application/pdf",
        OmrSourceFormat::Png => "image/png",
        OmrSourceFormat::Jpeg => "image/jpeg",
        OmrSourceFormat::Tiff => "image/tiff",
    }
}

fn source_from_extension(extension: &str) -> Option<OmrSourceFormat> {
    match extension.to_ascii_lowercase().as_str() {
        "pdf" => Some(OmrSourceFormat::Pdf),
        "png" => Some(OmrSourceFormat::Png),
        "jpg" | "jpeg" => Some(OmrSourceFormat::Jpeg),
        "tif" | "tiff" => Some(OmrSourceFormat::Tiff),
        _ => None,
    }
}

fn validate_pdf(bytes: &[u8]) -> Result<()> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(invalid("file header is not PDF"));
    }
    let tail = &bytes[bytes.len().saturating_sub(1024)..];
    if !tail.windows(5).any(|window| window == b"%%EOF") {
        return Err(invalid("PDF file is truncated or has no EOF marker"));
    }
    Ok(())
}

fn validate_png(bytes: &[u8]) -> Result<()> {
    const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
    if !bytes.starts_with(SIGNATURE) {
        return Err(invalid("file header is not PNG"));
    }
    let mut offset = SIGNATURE.len();
    let mut dimensions = None;
    let mut animated = false;
    let mut ended = false;
    while offset < bytes.len() {
        if offset + 12 > bytes.len() {
            return Err(invalid("PNG file is truncated"));
        }
        let length = u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let kind = &bytes[offset + 4..offset + 8];
        let end = offset
            .checked_add(12)
            .and_then(|value| value.checked_add(length))
            .ok_or_else(|| invalid("PNG chunk length overflow"))?;
        if end > bytes.len() {
            return Err(invalid("PNG chunk exceeds file size"));
        }
        let data = &bytes[offset + 8..offset + 8 + length];
        match kind {
            b"IHDR" if data.len() == 13 => {
                dimensions = Some((
                    u32::from_be_bytes(data[0..4].try_into().unwrap()),
                    u32::from_be_bytes(data[4..8].try_into().unwrap()),
                ));
            }
            b"acTL" | b"fcTL" => animated = true,
            b"IEND" => {
                ended = true;
                break;
            }
            _ => {}
        }
        offset = end;
    }
    if animated {
        return Err(invalid(
            "animated PNG is not supported; convert it to PDF or a single frame",
        ));
    }
    if !ended {
        return Err(invalid("PNG file has no IEND chunk"));
    }
    validate_dimensions(dimensions.ok_or_else(|| invalid("PNG has no IHDR dimensions"))?)
}

fn validate_jpeg(bytes: &[u8]) -> Result<()> {
    if !bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Err(invalid("file header is not JPEG"));
    }
    let mut offset = 2usize;
    let mut dimensions = None;
    let mut frames = 0usize;
    while offset < bytes.len() {
        if bytes[offset] != 0xff {
            return Err(invalid("JPEG marker stream is malformed"));
        }
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        let marker = *bytes
            .get(offset)
            .ok_or_else(|| invalid("JPEG file is truncated"))?;
        offset += 1;
        if marker == 0xd9 {
            break;
        }
        if marker == 0xda {
            break;
        }
        if marker == 0x01 || (0xd0..=0xd8).contains(&marker) {
            continue;
        }
        if offset + 2 > bytes.len() {
            return Err(invalid("JPEG segment is truncated"));
        }
        let length = u16::from_be_bytes(bytes[offset..offset + 2].try_into().unwrap()) as usize;
        if length < 2 || offset + length > bytes.len() {
            return Err(invalid("JPEG segment length is invalid"));
        }
        if is_sof(marker) {
            if length < 7 {
                return Err(invalid("JPEG frame header is truncated"));
            }
            frames += 1;
            if frames > 1 {
                return Err(invalid("multi-frame JPEG is not supported"));
            }
            let height =
                u16::from_be_bytes(bytes[offset + 3..offset + 5].try_into().unwrap()) as u32;
            let width =
                u16::from_be_bytes(bytes[offset + 5..offset + 7].try_into().unwrap()) as u32;
            dimensions = Some((width, height));
        }
        offset += length;
    }
    let dimensions = dimensions.ok_or_else(|| invalid("JPEG has no frame dimensions"))?;
    validate_dimensions(dimensions)
}

fn is_sof(marker: u8) -> bool {
    matches!(
        marker,
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf
    )
}

fn validate_tiff(bytes: &[u8]) -> Result<()> {
    if bytes.len() < 8 {
        return Err(invalid("TIFF file is truncated"));
    }
    let little = match &bytes[0..4] {
        b"II*\0" => true,
        b"MM\0*" => false,
        _ => return Err(invalid("file header is not TIFF")),
    };
    let read_u16 = |offset: usize| -> Result<u16> {
        let value = bytes
            .get(offset..offset + 2)
            .ok_or_else(|| invalid("TIFF entry is truncated"))?;
        Ok(if little {
            u16::from_le_bytes(value.try_into().unwrap())
        } else {
            u16::from_be_bytes(value.try_into().unwrap())
        })
    };
    let read_u32 = |offset: usize| -> Result<u32> {
        let value = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| invalid("TIFF offset is truncated"))?;
        Ok(if little {
            u32::from_le_bytes(value.try_into().unwrap())
        } else {
            u32::from_be_bytes(value.try_into().unwrap())
        })
    };
    let first = read_u32(4)? as usize;
    let mut offset = first;
    let mut dimensions = None;
    let mut frames = 0usize;
    let mut seen = HashSet::new();
    while offset != 0 {
        if !seen.insert(offset) {
            return Err(invalid("TIFF directory chain loops"));
        }
        frames += 1;
        if frames > 1 {
            return Err(invalid(
                "multi-frame TIFF is not supported; convert it to PDF instead",
            ));
        }
        let count = read_u16(offset)? as usize;
        let entries = offset
            .checked_add(2)
            .ok_or_else(|| invalid("TIFF directory offset overflow"))?;
        for index in 0..count {
            let entry = entries + index * 12;
            let tag = read_u16(entry)?;
            let kind = read_u16(entry + 2)?;
            let value_count = read_u32(entry + 4)?;
            if !matches!(tag, 256 | 257) || value_count == 0 {
                continue;
            }
            let value = match kind {
                3 => read_u16(entry + 8)? as u32,
                4 => read_u32(entry + 8)?,
                _ => return Err(invalid("TIFF dimensions use an unsupported value type")),
            };
            let pair = dimensions.get_or_insert((0, 0));
            if tag == 256 {
                pair.0 = value;
            } else {
                pair.1 = value;
            }
        }
        let next = entries + count * 12;
        offset = read_u32(next)? as usize;
    }
    if frames == 0 {
        return Err(invalid("TIFF has no image directory"));
    }
    validate_dimensions(dimensions.ok_or_else(|| invalid("TIFF has no dimensions"))?)
}

fn validate_dimensions((width, height): (u32, u32)) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(invalid("image dimensions must be positive"));
    }
    if width > MAX_IMAGE_EDGE || height > MAX_IMAGE_EDGE {
        return Err(invalid("image edge exceeds 20,000 pixels"));
    }
    if u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS {
        return Err(invalid("image exceeds 100 megapixels"));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::invalid(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend(13u32.to_be_bytes());
        bytes.extend(b"IHDR");
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes.extend([8, 6, 0, 0, 0]);
        bytes.extend([0; 4]);
        bytes.extend(0u32.to_be_bytes());
        bytes.extend(b"IEND");
        bytes.extend([0; 4]);
        bytes
    }

    fn tiff(pages: usize) -> Vec<u8> {
        let mut bytes = b"II*\0".to_vec();
        bytes.extend(8u32.to_le_bytes());
        for page in 0..pages {
            bytes.extend(2u16.to_le_bytes());
            bytes.extend(256u16.to_le_bytes());
            bytes.extend(4u16.to_le_bytes());
            bytes.extend(1u32.to_le_bytes());
            bytes.extend(1u32.to_le_bytes());
            bytes.extend(257u16.to_le_bytes());
            bytes.extend(4u16.to_le_bytes());
            bytes.extend(1u32.to_le_bytes());
            bytes.extend(1u32.to_le_bytes());
            let next = if page + 1 < pages {
                (8 + (page + 1) * 30) as u32
            } else {
                0
            };
            bytes.extend(next.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn accepts_supported_headers() {
        assert!(validate(
            OmrSourceFormat::Pdf,
            Some("a.pdf"),
            Some("application/pdf"),
            b"%PDF-1.7\n%%EOF"
        )
        .is_ok());
        assert!(validate(
            OmrSourceFormat::Png,
            Some("a.png"),
            Some("image/png"),
            &png(10, 10)
        )
        .is_ok());
    }

    #[test]
    fn rejects_mime_extension_and_magic_mismatches() {
        assert!(validate(
            OmrSourceFormat::Png,
            Some("a.pdf"),
            Some("image/png"),
            &png(1, 1)
        )
        .is_err());
        assert!(validate(
            OmrSourceFormat::Png,
            Some("a.png"),
            Some("image/jpeg"),
            &png(1, 1)
        )
        .is_err());
        assert!(validate(OmrSourceFormat::Pdf, Some("a.pdf"), None, &png(1, 1)).is_err());
        assert!(validate(
            OmrSourceFormat::Pdf,
            Some("a.pdf"),
            Some("application/pdf"),
            b"%PDF-1.7"
        )
        .is_err());
    }

    #[test]
    fn rejects_oversized_images() {
        assert!(validate(OmrSourceFormat::Png, None, None, &png(20_001, 1)).is_err());
    }

    #[test]
    fn tiff_accepts_one_frame_and_rejects_multiple_frames() {
        assert!(validate(OmrSourceFormat::Tiff, None, None, &tiff(1)).is_ok());
        assert!(validate(OmrSourceFormat::Tiff, None, None, &tiff(2)).is_err());
    }
}
