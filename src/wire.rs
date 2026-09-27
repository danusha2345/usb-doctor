//! Bounded decoding of USB IOCTL payloads; no references into packed FFI structures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub status: i32,
    pub address: u16,
    pub speed: u8,
    pub is_hub: bool,
    pub vendor: Option<u16>,
    pub product: Option<u16>,
    pub class: Option<u8>,
}
pub fn u32_at(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}
pub fn connection(bytes: &[u8]) -> Option<Connection> {
    if bytes.len() < 35 {
        return None;
    }
    let descriptor_ok = bytes[4] == 18 && bytes[5] == 1;
    Some(Connection {
        status: u32_at(bytes, 31)? as i32,
        address: u16::from_le_bytes([bytes[25], bytes[26]]),
        speed: bytes[23],
        is_hub: bytes[24] != 0,
        vendor: descriptor_ok.then(|| u16::from_le_bytes([bytes[12], bytes[13]])),
        product: descriptor_ok.then(|| u16::from_le_bytes([bytes[14], bytes[15]])),
        class: descriptor_ok.then_some(bytes[8]),
    })
}
pub fn v2(bytes: &[u8], port: u32) -> Option<(u32, u32)> {
    if bytes.len() < 16 || u32_at(bytes, 0)? != port || u32_at(bytes, 4)? != 16 {
        return None;
    }
    Some((u32_at(bytes, 8)?, u32_at(bytes, 12)?))
}
pub fn utf16z(bytes: &[u8]) -> Option<String> {
    if !bytes.len().is_multiple_of(2) {
        return None;
    }
    let units: Vec<_> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|x| u16::from_le_bytes([x[0], x[1]]))
        .collect();
    let end = units.iter().position(|x| *x == 0)?;
    String::from_utf16(&units[..end]).ok()
}
pub fn driver_key(bytes: &[u8], port: u32) -> Option<String> {
    if u32_at(bytes, 0)? != port {
        return None;
    }
    let length = u32_at(bytes, 4)? as usize;
    if !(10..=bytes.len()).contains(&length) {
        return None;
    }
    utf16z(&bytes[8..length])
}

/// USB SSP mantissa/exponent and lane_count_minus_one, from Windows SDK usbspec.h.
pub fn ssp_bps(attributes: u32, lane_count_minus_one: u32) -> Option<u64> {
    if attributes & 0x3f00 != 0 || lane_count_minus_one > 15 {
        return None;
    }
    let mantissa = u64::from(attributes >> 16);
    if mantissa == 0 {
        return None;
    }
    mantissa
        .checked_mul(1000u64.checked_pow((attributes >> 4) & 3)?)?
        .checked_mul(u64::from(lane_count_minus_one) + 1)
}

/// Some Windows hub drivers zero ConnectionIndex for an empty port (observed on Home-PC).
/// Never accept a mismatching index for a connected/failed device.
pub fn connection_for_port(bytes: &[u8], port: u32) -> Option<Connection> {
    let c = connection(bytes)?;
    let echo = u32_at(bytes, 0)?;
    if echo == port || (echo == 0 && c.status == 0) {
        Some(c)
    } else {
        None
    }
}
