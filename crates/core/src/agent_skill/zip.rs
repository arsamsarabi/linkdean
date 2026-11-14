//! A minimal zip writer: stored (uncompressed) entries only, which every unzip tool reads.
//! A skill is a few small Markdown files, so compression is not worth a dependency.

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn u16le(out: &mut Vec<u8>, v: u16) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn u32le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// DOS date 2026-01-01 00:00: fixed, so the same skill zips to the same bytes.
const DOS_TIME: u16 = 0;
const DOS_DATE: u16 = ((2026 - 1980) << 9) | (1 << 5) | 1;
/// Bit 11: names are UTF-8.
const FLAGS: u16 = 1 << 11;

/// Zips `files` (path, bytes) with no compression. Paths use `/`. Sizes must fit zip32
/// (4 GiB), far above what a skill holds.
pub fn zip_stored(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let offset = out.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        let name_bytes = name.as_bytes();

        u32le(&mut out, 0x0403_4b50);
        u16le(&mut out, 20); // version needed
        u16le(&mut out, FLAGS);
        u16le(&mut out, 0); // stored
        u16le(&mut out, DOS_TIME);
        u16le(&mut out, DOS_DATE);
        u32le(&mut out, crc);
        u32le(&mut out, size);
        u32le(&mut out, size);
        u16le(&mut out, name_bytes.len() as u16);
        u16le(&mut out, 0); // extra
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);

        u32le(&mut central, 0x0201_4b50);
        u16le(&mut central, 0x031E); // made by: Unix, 3.0
        u16le(&mut central, 20);
        u16le(&mut central, FLAGS);
        u16le(&mut central, 0);
        u16le(&mut central, DOS_TIME);
        u16le(&mut central, DOS_DATE);
        u32le(&mut central, crc);
        u32le(&mut central, size);
        u32le(&mut central, size);
        u16le(&mut central, name_bytes.len() as u16);
        u16le(&mut central, 0); // extra
        u16le(&mut central, 0); // comment
        u16le(&mut central, 0); // disk
        u16le(&mut central, 0); // internal attributes
        u32le(&mut central, 0o100644 << 16); // -rw-r--r--
        u32le(&mut central, offset);
        central.extend_from_slice(name_bytes);
    }
    let central_offset = out.len() as u32;
    let central_size = central.len() as u32;
    out.extend_from_slice(&central);
    u32le(&mut out, 0x0605_4b50);
    u16le(&mut out, 0);
    u16le(&mut out, 0);
    u16le(&mut out, files.len() as u16);
    u16le(&mut out, files.len() as u16);
    u32le(&mut out, central_size);
    u32le(&mut out, central_offset);
    u16le(&mut out, 0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn writes_entries_and_a_central_directory() {
        let zip = zip_stored(&[
            ("a/SKILL.md".into(), b"hello".to_vec()),
            ("a/references/LINKS.md".into(), "é".as_bytes().to_vec()),
        ]);
        assert_eq!(&zip[..4], &[0x50, 0x4b, 0x03, 0x04]);
        // End of central directory: two entries.
        let end = zip.len() - 22;
        assert_eq!(&zip[end..end + 4], &[0x50, 0x4b, 0x05, 0x06]);
        assert_eq!(u16::from_le_bytes([zip[end + 10], zip[end + 11]]), 2);
        // The first entry's data follows its 30-byte header and name, uncompressed.
        let name_len = "a/SKILL.md".len();
        assert_eq!(&zip[30 + name_len..30 + name_len + 5], b"hello");
        // The same input zips to the same bytes.
        assert_eq!(
            zip,
            zip_stored(&[
                ("a/SKILL.md".into(), b"hello".to_vec()),
                ("a/references/LINKS.md".into(), "é".as_bytes().to_vec()),
            ])
        );
    }
}
