//! A reader for the IVF container.
//!
//! An IVF file has a 32-byte header, then a sequence of packets. Each
//! packet has a 12-byte header (a size and a timestamp) and the data.
//! All numbers are little endian.

/// The header of an IVF file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    /// The four-character code of the codec, for example `AV01`.
    pub fourcc: [u8; 4],
    /// The width that the header declares.
    pub width: u16,
    /// The height that the header declares.
    pub height: u16,
    /// The time base numerator. One timestamp unit is `numerator /
    /// denominator` seconds.
    pub numerator: u32,
    /// The time base denominator.
    pub denominator: u32,
}

/// One packet: the data of one frame and its timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    /// The presentation timestamp, in time base units.
    pub pts: i64,
    /// The compressed data.
    pub data: Vec<u8>,
}

/// The reason that an IVF file does not parse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IvfError {
    /// The file does not start with `DKIF`.
    NotIvf,
    /// The file ends inside a header or a packet.
    Truncated,
    /// The time base has a zero in it.
    BadTimeBase,
    /// A timestamp does not fit an `i64`.
    BadTimestamp,
}

impl std::fmt::Display for IvfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            IvfError::NotIvf => "the file does not start with the IVF signature",
            IvfError::Truncated => "the file ends inside a header or a packet",
            IvfError::BadTimeBase => "the time base has a zero in it",
            IvfError::BadTimestamp => "a timestamp is too large",
        })
    }
}

/// Return true when `bytes` start with the IVF signature.
pub fn is_ivf(bytes: &[u8]) -> bool {
    bytes.starts_with(b"DKIF")
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// Parse a whole IVF file. Stop with an error at the first fault.
pub fn parse(bytes: &[u8]) -> Result<(Header, Vec<Packet>), IvfError> {
    if !is_ivf(bytes) {
        return Err(IvfError::NotIvf);
    }
    let header_size = u16_at(bytes, 6).ok_or(IvfError::Truncated)? as usize;
    let header = Header {
        fourcc: bytes
            .get(8..12)
            .and_then(|code| code.try_into().ok())
            .ok_or(IvfError::Truncated)?,
        width: u16_at(bytes, 12).ok_or(IvfError::Truncated)?,
        height: u16_at(bytes, 14).ok_or(IvfError::Truncated)?,
        denominator: u32_at(bytes, 16).ok_or(IvfError::Truncated)?,
        numerator: u32_at(bytes, 20).ok_or(IvfError::Truncated)?,
    };
    if header.numerator == 0 || header.denominator == 0 {
        return Err(IvfError::BadTimeBase);
    }

    let mut packets = Vec::new();
    let mut at = header_size.max(32);
    while at < bytes.len() {
        let size = u32_at(bytes, at).ok_or(IvfError::Truncated)? as usize;
        let pts = u64_at(bytes, at + 4).ok_or(IvfError::Truncated)?;
        let start = at + 12;
        let end = start.checked_add(size).ok_or(IvfError::Truncated)?;
        let data = bytes.get(start..end).ok_or(IvfError::Truncated)?.to_vec();
        packets.push(Packet {
            pts: i64::try_from(pts).map_err(|_| IvfError::BadTimestamp)?,
            data,
        });
        at = end;
    }
    Ok((header, packets))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(packets: &[(u64, &[u8])]) -> Vec<u8> {
        let mut out = b"DKIF".to_vec();
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(b"AV01");
        out.extend_from_slice(&64u16.to_le_bytes());
        out.extend_from_slice(&48u16.to_le_bytes());
        out.extend_from_slice(&30u32.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&(packets.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        for (pts, data) in packets {
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&pts.to_le_bytes());
            out.extend_from_slice(data);
        }
        out
    }

    #[test]
    fn a_file_parses_into_its_header_and_packets() {
        let (header, packets) = parse(&file(&[(0, b"ab"), (3, b"cde")])).expect("parse");
        assert_eq!(&header.fourcc, b"AV01");
        assert_eq!((header.width, header.height), (64, 48));
        assert_eq!((header.numerator, header.denominator), (1, 30));
        assert_eq!(packets.len(), 2);
        assert_eq!(packets[1].pts, 3);
        assert_eq!(packets[1].data, b"cde");
    }

    #[test]
    fn a_short_packet_is_refused() {
        let mut bytes = file(&[(0, b"abcd")]);
        bytes.pop();
        assert_eq!(parse(&bytes), Err(IvfError::Truncated));
    }

    #[test]
    fn a_file_without_the_signature_is_refused() {
        assert_eq!(parse(b"RIFF...."), Err(IvfError::NotIvf));
    }
}
