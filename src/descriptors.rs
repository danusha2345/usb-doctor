//! Preserve every returned byte; decode standard descriptors, retain unknown/class blocks.
use crate::{
    fields::{Field, hex},
    wire,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Descriptor {
    pub kind: String,
    pub index: u8,
    pub language: u16,
    pub raw: Vec<u8>,
    pub complete: bool,
    pub notes: Vec<String>,
}
impl Descriptor {
    pub fn new(kind: &str, index: u8, language: u16, raw: Vec<u8>) -> Self {
        Self {
            kind: kind.into(),
            index,
            language,
            raw,
            complete: true,
            notes: Vec::new(),
        }
    }
}
pub fn u16_at(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(off..off.checked_add(2)?)?.try_into().ok()?,
    ))
}
pub fn text(raw: &[u8]) -> Option<String> {
    let n = *raw.first()? as usize;
    if raw.get(1) != Some(&3) || n < 2 || n > raw.len() || !n.is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = raw[2..n]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|x| u16::from_le_bytes(*x))
        .collect();
    String::from_utf16(&units).ok()
}
pub fn languages(raw: &[u8]) -> Vec<u16> {
    if raw.len() < 2 || raw[1] != 3 || raw[0] as usize > raw.len() || raw[0] < 2 {
        return Vec::new();
    }
    raw[2..raw[0] as usize]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|x| u16::from_le_bytes(*x))
        .collect()
}
pub fn device(raw: &[u8]) -> Vec<Field> {
    if raw.len() < 18 || raw[0] != 18 || raw[1] != 1 {
        return Vec::new();
    }
    let mut result = Vec::new();
    for (id, label, v) in [
        ("bLength", "Длина device descriptor", raw[0] as u16),
        ("bcdUSB", "Версия спецификации USB", u16_at(raw, 2).unwrap()),
        ("bDeviceClass", "Класс устройства", raw[4] as u16),
        ("bDeviceSubClass", "Подкласс устройства", raw[5] as u16),
        ("bDeviceProtocol", "Протокол устройства", raw[6] as u16),
        ("bMaxPacketSize0", "MaxPacketSize EP0 (raw)", raw[7] as u16),
        ("bcdDevice", "Версия устройства", u16_at(raw, 12).unwrap()),
        ("iManufacturer", "Индекс производителя", raw[14] as u16),
        ("iProduct", "Индекс продукта", raw[15] as u16),
        ("iSerialNumber", "Индекс серийного номера", raw[16] as u16),
        ("bNumConfigurations", "Число конфигураций", raw[17] as u16),
    ] {
        result.push(Field::new(
            format!("usb.{id}"),
            label,
            "USB · устройство",
            if id.starts_with("bcd") {
                bcd_text(v)
            } else {
                v.to_string()
            },
            "USB_DEVICE_DESCRIPTOR",
            false,
        ));
    }
    result
}
/// Returns fields and string references; rejects invalid lengths without stepping outside buffers.
pub fn configuration(desc: &mut Descriptor, superspeed: bool) -> (Vec<Field>, Vec<u8>) {
    let mut out = Vec::new();
    let mut strings = Vec::new();
    let b = &desc.raw;
    let base = format!(
        "usb.{}.{}",
        if desc.kind == "Other speed configuration" {
            "other_config"
        } else {
            "config"
        },
        desc.index
    );
    let group = format!(
        "{} {}",
        if desc.kind == "Other speed configuration" {
            "Другая скорость · конфигурация"
        } else {
            "Конфигурация"
        },
        desc.index
    );
    if b.len() < 9 || b[0] != 9 || !matches!(b[1], 2 | 7) || u16_at(b, 2).is_none_or(|n| n < 9) {
        desc.complete = false;
        desc.notes
            .push("Усечённый/неверный заголовок конфигурации".into());
        return (out, strings);
    }
    let expected = u16_at(b, 2).unwrap() as usize;
    if expected != b.len() {
        desc.complete = false;
        desc.notes
            .push(format!("wTotalLength={expected}, получено {}", b.len()));
    }
    let limit = expected.min(b.len());
    let mut off = 0;
    let mut interface = 0;
    let mut interface_class = 0;
    let mut interface_subclass = 0;
    let mut interface_protocol = 0;
    let mut alt = 0;
    while off < limit {
        if limit - off < 2 {
            desc.complete = false;
            desc.notes.push("Усечённый заголовок блока".into());
            break;
        }
        let len = b[off] as usize;
        let typ = b[off + 1];
        if len < 2 || off + len > limit {
            desc.complete = false;
            desc.notes.push(format!("Неверная bLength в позиции {off}"));
            break;
        }
        let x = &b[off..off + len];
        let block_group = if typ == 4 && len >= 9 {
            format!("{group} · интерфейс {} / alt {}", x[2], x[3])
        } else if typ == 5 && len >= 7 {
            format!(
                "{group} · интерфейс {interface} / alt {alt} · EP {:02X}",
                x[2]
            )
        } else if off > 0 {
            format!("{group} · интерфейс {interface} / alt {alt} · блок @{off}")
        } else {
            group.clone()
        };
        let id = format!("{base}.offset{off}");
        let mut add = |suffix: &str, label: &str, value: String| {
            out.push(Field::new(
                format!("{id}.{suffix}"),
                label,
                &block_group,
                value,
                "Configuration descriptor",
                false,
            ))
        };
        add("type", "Тип блока", format!("0x{typ:02X}"));
        add("length", "Длина блока", len.to_string());
        match typ {
            2 | 7 if len >= 9 => {
                add("interfaces", "Число интерфейсов", x[4].to_string());
                add("value", "Configuration value", x[5].to_string());
                add(
                    "attributes",
                    "Атрибуты питания",
                    format!(
                        "0x{:02X}; self-powered={}, remote-wakeup={}",
                        x[7],
                        x[7] & 64 != 0,
                        x[7] & 32 != 0
                    ),
                );
                strings.push(x[6]);
                out.push(Field::new(
                    format!("{base}.max_power"),
                    "Заявленное потребление",
                    &group,
                    format!(
                        "{} мА (дескриптор, не измерение)",
                        u16::from(x[8]) * if superspeed { 8 } else { 2 }
                    ),
                    "bMaxPower",
                    false,
                ));
            }
            4 if len >= 9 => {
                interface = x[2];
                interface_class = x[5];
                interface_subclass = x[6];
                interface_protocol = x[7];
                alt = x[3];
                add(
                    "interface",
                    "Интерфейс / alt",
                    format!("{} / {}", interface, alt),
                );
                add("endpoints", "Число endpoint", x[4].to_string());
                add(
                    "class",
                    "Класс / подкласс / протокол",
                    format!("{:02X} / {:02X} / {:02X}", x[5], x[6], x[7]),
                );
                strings.push(x[8]);
            }
            5 if len >= 7 => {
                add(
                    "endpoint",
                    "Endpoint",
                    format!(
                        "0x{:02X} · {} · interface {interface}, alt {alt}",
                        x[2],
                        if x[2] & 128 != 0 { "IN" } else { "OUT" }
                    ),
                );
                add(
                    "attributes",
                    "Transfer attributes",
                    format!("0x{:02X}", x[3]),
                );
                add(
                    "packet",
                    "wMaxPacketSize (raw)",
                    u16_at(x, 4).unwrap().to_string(),
                );
                add("interval", "bInterval", x[6].to_string());
            }
            11 if len >= 8 => {
                add(
                    "iad",
                    "Interface association",
                    format!(
                        "first={}, count={}, class={:02X}/{:02X}/{:02X}",
                        x[2], x[3], x[4], x[5], x[6]
                    ),
                );
                strings.push(x[7]);
            }
            0x21 if interface_class == 3 && len >= 6 => {
                add(
                    "hid_version",
                    "Версия HID",
                    format!("0x{:04X}", u16_at(x, 2).unwrap()),
                );
                add("country", "HID country", x[4].to_string());
                add("class_data", "HID subordinate descriptors", hex(&x[5..]));
            }
            0x21 if interface_class == 0xfe && interface_subclass == 1 && len >= 9 => {
                add("dfu_attributes", "Атрибуты DFU", format!("0x{:02X}", x[2]));
                add(
                    "dfu_timeout",
                    "DFU detach timeout (мс)",
                    u16_at(x, 3).unwrap().to_string(),
                );
                add(
                    "dfu_transfer",
                    "DFU transfer size",
                    u16_at(x, 5).unwrap().to_string(),
                );
                add(
                    "dfu_version",
                    "Версия DFU",
                    format!("0x{:04X}", u16_at(x, 7).unwrap()),
                );
            }
            0x24 | 0x25 => {
                if crate::class_descriptors::required_length(
                    x,
                    interface_class,
                    interface_subclass,
                    interface_protocol,
                )
                .is_some_and(|required| len < required)
                {
                    desc.complete = false;
                    desc.notes
                        .push(format!("Усечённый class-specific блок в позиции {off}"));
                }
                add(
                    "class_data",
                    "Audio / Video / CDC / MIDI class bytes",
                    hex(&x[2..]),
                );
                let (decoded, refs) = crate::class_descriptors::class_fields(
                    x,
                    interface_class,
                    interface_subclass,
                    interface_protocol,
                    &id,
                    &block_group,
                );
                out.extend(decoded);
                strings.extend(refs);
            }
            0x30 if len >= 6 => {
                add("burst", "Max burst", x[2].to_string());
                add("ss_attributes", "SS attributes", format!("0x{:02X}", x[3]));
                add(
                    "bytes_interval",
                    "Bytes per interval",
                    u16_at(x, 4).unwrap().to_string(),
                );
            }
            _ => add("raw", "Неразобранные байты", hex(x)),
        }
        off += len;
    }
    strings.retain(|x| *x != 0);
    strings.sort();
    strings.dedup();
    (out, strings)
}
pub fn bos(desc: &mut Descriptor) -> Vec<Field> {
    let b = &desc.raw;
    let mut fields = Vec::new();
    if b.len() < 5 || b[0] != 5 || b[1] != 15 || u16_at(b, 2).is_none_or(|n| n < 5) {
        desc.complete = false;
        desc.notes.push("Некорректный BOS header".into());
        return fields;
    }
    let total = u16_at(b, 2).unwrap() as usize;
    if total != b.len() {
        desc.complete = false;
        desc.notes
            .push("BOS wTotalLength отличается от длины ответа".into());
    }
    fields.push(Field::new(
        "usb.bos.count",
        "Число capabilities",
        "BOS",
        b[4],
        "BOS header",
        false,
    ));
    let mut off = 5;
    let mut count = 0;
    while off < total.min(b.len()) {
        let Some(h) = b.get(off..off + 3) else {
            desc.complete = false;
            break;
        };
        let len = h[0] as usize;
        if len < 3 || off + len > total.min(b.len()) {
            desc.complete = false;
            break;
        }
        if h[1] != 16 {
            desc.complete = false;
            desc.notes
                .push(format!("BOS: блок {off} не является Device Capability"));
        }
        count += 1;
        if h[1] == 16 {
            fields.extend(crate::class_descriptors::capability(
                &b[off..off + len],
                &format!("usb.bos.{off}"),
            ));
        }
        fields.push(Field::new(
            format!("usb.bos.{off}.type"),
            "Device capability type",
            "BOS",
            format!("0x{:02X}", h[2]),
            "BOS capability",
            false,
        ));
        fields.push(Field::new(
            format!("usb.bos.{off}.data"),
            "Capability bytes",
            "BOS",
            hex(&b[off..off + len]),
            "BOS capability",
            true,
        ));
        off += len;
    }
    if count != b[4] {
        desc.complete = false;
        desc.notes.push(format!(
            "BOS: заявлено {} capabilities, получено {count}",
            b[4]
        ));
    }
    fields
}
pub fn property_value(kind: u32, b: &[u8]) -> String {
    match kind {
        18 | 25 => wire::utf16z(b).unwrap_or_else(|| format!("UTF-16? {}", hex(b))),
        8210 => b
            .as_chunks::<2>()
            .0
            .iter()
            .map(|x| u16::from_le_bytes(*x))
            .collect::<Vec<_>>()
            .split(|x| *x == 0)
            .filter(|x| !x.is_empty())
            .map(String::from_utf16_lossy)
            .collect::<Vec<_>>()
            .join(" | "),
        3 if b.len() == 1 => b[0].to_string(),
        17 if b.len() == 1 => (b[0] != 0).to_string(),
        5 if b.len() == 2 => u16_at(b, 0).unwrap().to_string(),
        7 | 22 | 23 | 24 if b.len() == 4 => wire::u32_at(b, 0).unwrap().to_string(),
        16 if b.len() == 8 => {
            let ticks = u64::from_le_bytes(b.try_into().unwrap());
            let unix = (ticks / 10_000_000) as i64 - 11_644_473_600;
            time::OffsetDateTime::from_unix_timestamp(unix)
                .map(|t| t.date().to_string())
                .unwrap_or_else(|_| ticks.to_string())
        }
        9 if b.len() == 8 => u64::from_le_bytes(b.try_into().unwrap()).to_string(),
        13 if b.len() == 16 => format!(
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{}",
            wire::u32_at(b, 0).unwrap(),
            u16_at(b, 4).unwrap(),
            u16_at(b, 6).unwrap(),
            b[8],
            b[9],
            b[10..]
                .iter()
                .map(|x| format!("{x:02X}"))
                .collect::<String>()
        ),
        _ => hex(b),
    }
}

pub fn bcd_text(value: u16) -> String {
    let a = (value >> 12) & 15;
    let b = (value >> 8) & 15;
    let c = (value >> 4) & 15;
    let d = value & 15;
    if [a, b, c, d].iter().any(|x| *x > 9) {
        format!("0x{value:04X} (не BCD)")
    } else {
        format!("{}.{:02} (0x{value:04X})", a * 10 + b, c * 10 + d)
    }
}

pub fn power_fields(bytes: &[u8]) -> Vec<Field> {
    let Some(size) = wire::u32_at(bytes, 0) else {
        return Vec::new();
    };
    if size < 12 || size as usize > bytes.len() {
        return Vec::new();
    }
    let state = wire::u32_at(bytes, 4).unwrap();
    let label = match state {
        1 => "D0",
        2 => "D1",
        3 => "D2",
        4 => "D3",
        _ => "Не определено",
    };
    vec![
        Field::new(
            "windows.power.state",
            "Последнее D-состояние",
            "Питание",
            label,
            "CM_POWER_DATA.PD_MostRecentPowerState",
            false,
        ),
        Field::new(
            "windows.power.capabilities",
            "Power capabilities",
            "Питание",
            format!("0x{:08X}", wire::u32_at(bytes, 8).unwrap()),
            "CM_POWER_DATA",
            false,
        ),
    ]
}
