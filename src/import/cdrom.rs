//! Read-only ISO 9660 access for GOG MODE1/2352 CD images and plain ISO images.
use std::io::{self, Read, Seek, SeekFrom, Write};

/// Validated root directory and sector layout, with a seekable image reader.
pub struct CdImage<R> {
    reader: R,
    sector: u64,
    offset: u64,
    root: (u32, u32),
}

/// One validated root-directory file in an ISO or MODE1 image.
/// The layout is retained so callers can seek without copying the preceding payload.
#[derive(Debug, Clone)]
pub struct CdFile {
    /// File payload length, excluding sector headers and padding.
    pub size: u64,
    extent: u32,
    sector: u64,
    offset: u64,
}
impl CdFile {
    /// Read a bounded payload range. Sector decoding stays inside the CD parser.
    pub fn read_range(
        &self,
        reader: &mut (impl Read + Seek),
        start: u64,
        length: usize,
    ) -> io::Result<Vec<u8>> {
        if start > self.size || length as u64 > self.size - start {
            return Err(invalid("CD file: range outside payload"));
        }
        if length == 0 {
            return Ok(Vec::new());
        }
        let first = start / 2048;
        let last = (start + length as u64 - 1) / 2048;
        let physical = (u64::from(self.extent) + first) * self.sector + self.offset;
        let span = (last - first) * self.sector + ((start + length as u64 - 1) % 2048) + 1;
        let mut sectors =
            vec![0; usize::try_from(span).map_err(|_| invalid("CD file: range too large"))?];
        reader.seek(SeekFrom::Start(physical))?;
        reader.read_exact(&mut sectors)?;
        let mut bytes = Vec::with_capacity(length);
        for index in first..=last {
            let begin = if index == first { start % 2048 } else { 0 };
            let count = (2048 - begin).min(length as u64 - bytes.len() as u64);
            let offset = usize::try_from((index - first) * self.sector + begin)
                .map_err(|_| invalid("CD file: range too large"))?;
            bytes.extend_from_slice(&sectors[offset..offset + count as usize]);
        }
        Ok(bytes)
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn le32(bytes: &[u8], start: usize) -> u32 {
    u32::from_le_bytes(bytes[start..start + 4].try_into().unwrap())
}

impl<R: Read + Seek> CdImage<R> {
    /// Detect a standard 2048-byte ISO or MODE1 raw sector layout.
    pub fn open(mut reader: R) -> io::Result<Self> {
        for (sector, offset) in [(2048, 0), (2352, 16)] {
            reader.seek(SeekFrom::Start(16 * sector + offset))?;
            let mut pvd = [0; 2048];
            if reader.read_exact(&mut pvd).is_ok() && pvd[..7] == *b"\x01CD001\x01" {
                if pvd[156] < 34 || u16::from_le_bytes([pvd[128], pvd[129]]) != 2048 {
                    return Err(invalid("CD image: invalid primary descriptor"));
                }
                let root = (le32(&pvd, 158), le32(&pvd, 166));
                return Ok(Self {
                    reader,
                    sector,
                    offset,
                    root,
                });
            }
        }
        Err(invalid("CD image: unsupported ISO 9660 sector layout"))
    }
    fn block(&mut self, index: u32) -> io::Result<[u8; 2048]> {
        self.reader.seek(SeekFrom::Start(
            u64::from(index) * self.sector + self.offset,
        ))?;
        let mut bytes = [0; 2048];
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    /// Locate a root-directory file without reading its payload.
    /// Version suffixes are ignored; directories and multi-extent records are unsupported.
    pub fn file(&mut self, name: &str) -> io::Result<CdFile> {
        if name.is_empty() || name.contains(['/', '\\', ':']) || name == "." || name == ".." {
            return Err(invalid("CD image: expected root filename"));
        }
        let (extent, size) = self.root;
        if size > 16 * 1024 * 1024 {
            return Err(invalid("CD image: root directory too large"));
        }
        let mut found = None;
        for index in 0..size.div_ceil(2048) {
            let block = self.block(
                extent
                    .checked_add(index)
                    .ok_or_else(|| invalid("CD extent overflow"))?,
            )?;
            let limit = (size - index * 2048).min(2048) as usize;
            let mut pos = 0;
            while pos < limit && block[pos] != 0 {
                let len = block[pos] as usize;
                if len < 34 || pos + len > limit {
                    return Err(invalid("CD image: invalid directory record"));
                }
                let record = &block[pos..pos + len];
                let n = record[32] as usize;
                if 33 + n > len {
                    return Err(invalid("CD image: invalid filename length"));
                }
                if let Ok(file) = std::str::from_utf8(&record[33..33 + n])
                    && file
                        .split(';')
                        .next()
                        .unwrap_or_default()
                        .eq_ignore_ascii_case(name)
                {
                    if record[25] & 0x82 != 0 {
                        return Err(invalid("CD image: directory/multi-extent file unsupported"));
                    }
                    found = Some((le32(record, 2), le32(record, 10)));
                    break;
                }
                pos += len;
            }
            if found.is_some() {
                break;
            }
        }
        let (extent, size) = found.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("CD image: missing {name}"))
        })?;
        Ok(CdFile {
            size: u64::from(size),
            extent,
            sector: self.sector,
            offset: self.offset,
        })
    }

    /// Copy a root-directory filename without buffering the movie.
    pub fn copy_file(&mut self, name: &str, output: &mut impl Write) -> io::Result<u64> {
        let file = self.file(name)?;
        let mut start = 0;
        while start < file.size {
            let length = usize::try_from((file.size - start).min(64 * 1024)).unwrap();
            output.write_all(&file.read_range(&mut self.reader, start, length)?)?;
            start += length as u64;
        }
        Ok(file.size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn reads_payload_without_sector_headers_and_rejects_truncation() {
        for (sector, offset) in [(2048, 0), (2352, 16)] {
            let mut bytes = vec![0; 23 * sector];
            let pvd = 16 * sector + offset;
            bytes[pvd..pvd + 7].copy_from_slice(b"\x01CD001\x01");
            bytes[pvd + 128..pvd + 130].copy_from_slice(&2048u16.to_le_bytes());
            bytes[pvd + 156] = 34;
            bytes[pvd + 158..pvd + 162].copy_from_slice(&20u32.to_le_bytes());
            bytes[pvd + 166..pvd + 170].copy_from_slice(&2048u32.to_le_bytes());
            let record = 20 * sector + offset;
            bytes[record] = 42;
            bytes[record + 2..record + 6].copy_from_slice(&21u32.to_le_bytes());
            bytes[record + 10..record + 14].copy_from_slice(&2051u32.to_le_bytes());
            bytes[record + 32] = 8;
            bytes[record + 33..record + 41].copy_from_slice(b"TEST.X;1");
            bytes[21 * sector + offset..21 * sector + offset + 2048].fill(b'a');
            bytes[22 * sector + offset..22 * sector + offset + 3].copy_from_slice(b"bcd");
            let mut cd = CdImage::open(Cursor::new(bytes.clone())).unwrap();
            let mut out = Vec::new();
            let file = cd.file("test.x").unwrap();
            assert_eq!(file.size, 2051);
            assert_eq!(
                file.read_range(&mut Cursor::new(bytes.clone()), 2047, 4)
                    .unwrap(),
                b"abcd"
            );
            assert_eq!(
                file.read_range(&mut Cursor::new(bytes.clone()), 2051, 0)
                    .unwrap(),
                b""
            );
            assert!(
                file.read_range(&mut Cursor::new(bytes.clone()), 2050, 2)
                    .is_err()
            );
            assert_eq!(cd.copy_file("test.x", &mut out).unwrap(), 2051);
            assert_eq!(out, [vec![b'a'; 2048], b"bcd".to_vec()].concat());
            assert_eq!(
                cd.copy_file("absent", &mut out).unwrap_err().kind(),
                io::ErrorKind::NotFound
            );
            bytes.truncate(21 * sector + offset + 2);
            let mut cd = CdImage::open(Cursor::new(bytes)).unwrap();
            assert!(cd.copy_file("test.x", &mut Vec::new()).is_err());
        }
    }
}
