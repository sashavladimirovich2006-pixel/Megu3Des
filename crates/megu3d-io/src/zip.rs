//! Minimal ZIP support for the `.megu3d` container.
//!
//! Only the *stored* method is written and accepted. The container holds JSON
//! and already-compressed assets, so a compression stack would buy little and
//! would add a dependency the offline toolchain cannot verify
//! (`docs/assumptions.md`, `D-95`).

use thiserror::Error;

const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const METHOD_STORED: u16 = 0;
const VERSION_NEEDED: u16 = 20;
/// 1980-01-01, the earliest date DOS can express. Fixed on purpose: the same
/// scene must produce the same bytes (`D-97`).
const DOS_DATE: u16 = 0x0021;
const DOS_TIME: u16 = 0;
const LOCAL_HEADER_LEN: usize = 30;
const CENTRAL_HEADER_LEN: usize = 46;
const EOCD_LEN: usize = 22;
const MAX_COMMENT: usize = 0xffff;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ZipError {
    #[error("archive ends before byte {0}")]
    Truncated(usize),
    #[error("not a zip archive: no end of central directory record")]
    NotZip,
    #[error("unsupported zip feature: {0}")]
    Unsupported(&'static str),
    #[error("entry `{0}` failed its crc32 check")]
    Crc(String),
    #[error("entry `{0}` is stored twice")]
    Duplicate(String),
    #[error("entry name is not valid utf-8")]
    Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZipEntry {
    pub name: String,
    pub data: Vec<u8>,
}

impl ZipEntry {
    pub fn new(name: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            name: name.into(),
            data: data.into(),
        }
    }
}

/// CRC-32 (IEEE, reflected), computed bitwise. Archives here are a few
/// megabytes at most, so a lookup table would be premature.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

/// Packs entries in the given order. Order is part of the format: readers that
/// stream the archive see the manifest before the scene.
pub fn write(entries: &[ZipEntry]) -> Result<Vec<u8>, ZipError> {
    let mut seen: Vec<&str> = Vec::with_capacity(entries.len());
    for entry in entries {
        if seen.contains(&entry.name.as_str()) {
            return Err(ZipError::Duplicate(entry.name.clone()));
        }
        seen.push(entry.name.as_str());
    }

    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for entry in entries {
        let offset = u32::try_from(out.len())
            .map_err(|_| ZipError::Unsupported("archive is larger than 4 GiB"))?;
        let size = u32::try_from(entry.data.len())
            .map_err(|_| ZipError::Unsupported("entry is larger than 4 GiB"))?;
        let name = entry.name.as_bytes();
        let name_len =
            u16::try_from(name.len()).map_err(|_| ZipError::Unsupported("entry name is too long"))?;
        let crc = crc32(&entry.data);

        push_u32(&mut out, LOCAL_SIGNATURE);
        push_u16(&mut out, VERSION_NEEDED);
        push_u16(&mut out, 0);
        push_u16(&mut out, METHOD_STORED);
        push_u16(&mut out, DOS_TIME);
        push_u16(&mut out, DOS_DATE);
        push_u32(&mut out, crc);
        push_u32(&mut out, size);
        push_u32(&mut out, size);
        push_u16(&mut out, name_len);
        push_u16(&mut out, 0);
        out.extend_from_slice(name);
        out.extend_from_slice(&entry.data);

        push_u32(&mut directory, CENTRAL_SIGNATURE);
        push_u16(&mut directory, VERSION_NEEDED);
        push_u16(&mut directory, VERSION_NEEDED);
        push_u16(&mut directory, 0);
        push_u16(&mut directory, METHOD_STORED);
        push_u16(&mut directory, DOS_TIME);
        push_u16(&mut directory, DOS_DATE);
        push_u32(&mut directory, crc);
        push_u32(&mut directory, size);
        push_u32(&mut directory, size);
        push_u16(&mut directory, name_len);
        push_u16(&mut directory, 0);
        push_u16(&mut directory, 0);
        push_u16(&mut directory, 0);
        push_u16(&mut directory, 0);
        push_u32(&mut directory, 0);
        push_u32(&mut directory, offset);
        directory.extend_from_slice(name);
    }

    let directory_offset = u32::try_from(out.len())
        .map_err(|_| ZipError::Unsupported("archive is larger than 4 GiB"))?;
    let directory_size = u32::try_from(directory.len())
        .map_err(|_| ZipError::Unsupported("directory is larger than 4 GiB"))?;
    let count =
        u16::try_from(entries.len()).map_err(|_| ZipError::Unsupported("too many entries"))?;
    out.extend_from_slice(&directory);
    push_u32(&mut out, EOCD_SIGNATURE);
    push_u16(&mut out, 0);
    push_u16(&mut out, 0);
    push_u16(&mut out, count);
    push_u16(&mut out, count);
    push_u32(&mut out, directory_size);
    push_u32(&mut out, directory_offset);
    push_u16(&mut out, 0);
    Ok(out)
}

/// Reads through the central directory, so a truncated tail is reported
/// instead of being silently accepted.
pub fn read(bytes: &[u8]) -> Result<Vec<ZipEntry>, ZipError> {
    let eocd = find_eocd(bytes).ok_or(ZipError::NotZip)?;
    let count = usize::from(read_u16(bytes, eocd + 10)?);
    let mut offset = read_u32(bytes, eocd + 16)? as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        if read_u32(bytes, offset)? != CENTRAL_SIGNATURE {
            return Err(ZipError::Unsupported("central directory is malformed"));
        }
        if read_u16(bytes, offset + 10)? != METHOD_STORED {
            return Err(ZipError::Unsupported("only stored entries are supported"));
        }
        let crc = read_u32(bytes, offset + 16)?;
        let compressed = read_u32(bytes, offset + 20)? as usize;
        let size = read_u32(bytes, offset + 24)? as usize;
        if compressed != size {
            return Err(ZipError::Unsupported("stored entry sizes disagree"));
        }
        let name_len = usize::from(read_u16(bytes, offset + 28)?);
        let extra_len = usize::from(read_u16(bytes, offset + 30)?);
        let comment_len = usize::from(read_u16(bytes, offset + 32)?);
        let local = read_u32(bytes, offset + 42)? as usize;
        let raw_name = slice(bytes, offset + CENTRAL_HEADER_LEN, name_len)?;
        let name = String::from_utf8(raw_name.to_vec()).map_err(|_| ZipError::Name)?;
        if read_u32(bytes, local)? != LOCAL_SIGNATURE {
            return Err(ZipError::Unsupported("local header is malformed"));
        }
        let local_name = usize::from(read_u16(bytes, local + 26)?);
        let local_extra = usize::from(read_u16(bytes, local + 28)?);
        let start = local + LOCAL_HEADER_LEN + local_name + local_extra;
        let data = slice(bytes, start, size)?;
        if crc32(data) != crc {
            return Err(ZipError::Crc(name));
        }
        entries.push(ZipEntry {
            name,
            data: data.to_vec(),
        });
        offset += CENTRAL_HEADER_LEN + name_len + extra_len + comment_len;
    }
    Ok(entries)
}

pub fn entry<'a>(entries: &'a [ZipEntry], name: &str) -> Option<&'a ZipEntry> {
    entries.iter().find(|entry| entry.name == name)
}

fn find_eocd(bytes: &[u8]) -> Option<usize> {
    let last = bytes.len().checked_sub(EOCD_LEN)?;
    let floor = last.saturating_sub(MAX_COMMENT);
    let mut at = last;
    loop {
        if read_u32(bytes, at).ok() == Some(EOCD_SIGNATURE) {
            return Some(at);
        }
        if at == floor {
            return None;
        }
        at -= 1;
    }
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn slice(bytes: &[u8], at: usize, len: usize) -> Result<&[u8], ZipError> {
    let end = at.checked_add(len).ok_or(ZipError::Truncated(at))?;
    bytes.get(at..end).ok_or(ZipError::Truncated(end))
}

fn read_u16(bytes: &[u8], at: usize) -> Result<u16, ZipError> {
    let raw = slice(bytes, at, 2)?;
    Ok(u16::from_le_bytes([raw[0], raw[1]]))
}

fn read_u32(bytes: &[u8], at: usize) -> Result<u32, ZipError> {
    let raw = slice(bytes, at, 4)?;
    Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn archives_round_trip() {
        let entries = vec![
            ZipEntry::new("manifest.json", b"{\"a\":1}".to_vec()),
            ZipEntry::new("scene.json", vec![7u8; 5000]),
        ];
        let bytes = write(&entries).expect("write");
        let read_back = read(&bytes).expect("read");
        assert_eq!(read_back, entries);
        assert_eq!(
            entry(&read_back, "scene.json").map(|found| found.data.len()),
            Some(5000)
        );
        assert!(entry(&read_back, "missing.json").is_none());
    }

    #[test]
    fn an_empty_archive_round_trips() {
        let bytes = write(&[]).expect("write");
        assert_eq!(bytes.len(), EOCD_LEN);
        assert!(read(&bytes).expect("read").is_empty());
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let entries = vec![
            ZipEntry::new("scene.json", b"1".to_vec()),
            ZipEntry::new("scene.json", b"2".to_vec()),
        ];
        assert_eq!(
            write(&entries),
            Err(ZipError::Duplicate("scene.json".to_owned()))
        );
    }

    #[test]
    fn corrupted_payloads_are_caught() {
        let entries = vec![ZipEntry::new("scene.json", b"hello".to_vec())];
        let mut bytes = write(&entries).expect("write");
        let at = LOCAL_HEADER_LEN + "scene.json".len();
        bytes[at] = b'H';
        assert_eq!(read(&bytes), Err(ZipError::Crc("scene.json".to_owned())));
    }

    #[test]
    fn plain_bytes_are_not_an_archive() {
        assert_eq!(read(b"not a zip file at all"), Err(ZipError::NotZip));
    }
}
