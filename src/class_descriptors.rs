//! Read-only semantic views over preserved configuration/BOS bytes.
//! Layout references and supported versions: docs/research/DESCRIPTOR_DECODERS.md.
use crate::{
    descriptors::{bcd_text, u16_at},
    fields::{Field, hex},
    wire::u32_at,
};

struct View<'a> {
    raw: &'a [u8],
    id: &'a str,
    group: &'a str,
    out: Vec<Field>,
    strings: Vec<u8>,
}
impl<'a> View<'a> {
    fn add(&mut self, key: &str, label: &str, value: impl ToString) {
        self.out.push(Field::new(
            format!("{}.{}", self.id, key),
            label,
            self.group,
            value,
            "USB descriptor",
            false,
        ));
    }
    fn n(&mut self, key: &str, label: &str, offset: usize, size: usize) {
        if let Some(b) = self.raw.get(offset..offset + size) {
            let v = b
                .iter()
                .enumerate()
                .fold(0u64, |a, (i, x)| a | (u64::from(*x) << (i * 8)));
            self.add(key, label, v);
        }
    }
    fn bytes(&mut self, key: &str, label: &str, start: usize, len: usize) {
        if let Some(b) = self.raw.get(start..start + len) {
            self.add(key, label, hex(b));
        }
    }
    fn string(&mut self, offset: usize) {
        if let Some(&v) = self.raw.get(offset)
            && v != 0
        {
            self.strings.push(v);
        }
    }
    fn version(&mut self, offset: usize) {
        if let Some(v) = u16_at(self.raw, offset) {
            self.add("version", "Версия спецификации", bcd_text(v));
        }
    }
}

/// Unknown versions remain raw; never interpret UAC2/3 as UAC1.
pub fn class_fields(
    x: &[u8],
    class: u8,
    subclass: u8,
    protocol: u8,
    id: &str,
    group: &str,
) -> (Vec<Field>, Vec<u8>) {
    let mut v = View {
        raw: x,
        id,
        group,
        out: vec![],
        strings: vec![],
    };
    if x.len() < 3 {
        return (v.out, v.strings);
    }
    let st = x[2];
    v.add("subtype", "Подтип дескриптора", format!("0x{st:02X}"));
    match (class, subclass, x[1]) {
        (2, _, 0x24) => {
            let name = match st {
                0 => "CDC Header",
                1 => "CDC Call Management",
                2 => "CDC ACM",
                6 => "CDC Union",
                7 => "CDC Country",
                0x0f => "CDC Ethernet",
                0x1a => "CDC NCM",
                0x1b => "CDC MBIM",
                0x1c => "CDC MBIM Extended",
                _ => "CDC · неизвестный подтип",
            };
            v.add("kind", "Назначение", name);
            match st {
                0 => v.version(3),
                1 => {
                    v.n("capabilities", "Call Management capabilities", 3, 1);
                    v.n("data_interface", "Интерфейс данных", 4, 1);
                }
                2 => v.n("capabilities", "ACM capabilities", 3, 1),
                6 => {
                    v.n("control_interface", "Управляющий интерфейс", 3, 1);
                    if x.len() > 4 {
                        v.bytes("interfaces", "Связанные интерфейсы", 4, x.len() - 4);
                    }
                }
                7 => {
                    v.string(3);
                    if x.len() > 4 {
                        v.bytes("countries", "Коды стран (LE16)", 4, x.len() - 4);
                    }
                }
                0x0f => {
                    v.string(3);
                    v.n("statistics", "Ethernet statistics bitmap", 4, 4);
                    v.n("segment_bytes", "Максимальный Ethernet-сегмент, байт", 8, 2);
                    v.n("multicast_filters", "Multicast filters", 10, 2);
                    v.n("power_filters", "Power filters", 12, 1);
                }
                0x1a => {
                    v.version(3);
                    v.n("capabilities", "NCM capabilities", 5, 1);
                }
                0x1b => {
                    v.version(3);
                    v.n(
                        "control_bytes",
                        "Максимальное управляющее сообщение, байт",
                        5,
                        2,
                    );
                    v.n("filters", "Число фильтров", 7, 1);
                    v.n("filter_bytes", "Размер фильтра", 8, 1);
                    v.n("segment_bytes", "Максимальный сегмент, байт", 9, 2);
                    v.n("capabilities", "MBIM capabilities", 11, 1);
                }
                0x1c => {
                    v.version(3);
                    v.n("commands", "Одновременных команд", 5, 1);
                    v.n("mtu", "MTU, байт", 6, 2);
                }
                _ => {}
            }
        }
        (1, 3, 0x24) if protocol == 0 => {
            v.add(
                "kind",
                "Назначение",
                match st {
                    1 => "MIDI Streaming Header",
                    2 => "MIDI IN Jack",
                    3 => "MIDI OUT Jack",
                    _ => "MIDI · неизвестный подтип",
                },
            );
            match st {
                1 => {
                    v.version(3);
                    v.n("total_length", "Длина MIDI-описания", 5, 2);
                }
                2 => {
                    v.n("jack_type", "Тип MIDI-разъёма", 3, 1);
                    v.n("jack_id", "ID MIDI-разъёма", 4, 1);
                    v.string(5);
                }
                3 => {
                    v.n("jack_type", "Тип MIDI-разъёма", 3, 1);
                    v.n("jack_id", "ID MIDI-разъёма", 4, 1);
                    if let Some(&n) = x.get(5) {
                        v.n("pins", "Входных связей", 5, 1);
                        v.bytes("sources", "Пары source ID / pin", 6, n as usize * 2);
                        v.string(6 + n as usize * 2);
                    }
                }
                _ => {}
            }
        }
        (1, 3, 0x25) if protocol == 0 && st == 1 => {
            v.add("kind", "Назначение", "MIDI endpoint");
            if let Some(&n) = x.get(3) {
                v.bytes("jacks", "Связанные MIDI Jack IDs", 4, n as usize);
            }
        }
        (1, 1, 0x24) if matches!(protocol, 0 | 0x20) => {
            let uac2 = protocol == 0x20;
            v.add(
                "kind",
                "Назначение",
                format!(
                    "UAC{} · {}",
                    if uac2 { 2 } else { 1 },
                    match st {
                        1 => "Header",
                        2 => "Input Terminal",
                        3 => "Output Terminal",
                        4 => "Mixer",
                        5 => "Selector",
                        6 => "Feature Unit",
                        0x0a if uac2 => "Clock Source",
                        0x0b if uac2 => "Clock Selector",
                        0x0c if uac2 => "Clock Multiplier",
                        _ => "другой блок",
                    }
                ),
            );
            match st {
                1 => {
                    v.version(3);
                    v.n(
                        "total_length",
                        "Длина AudioControl-описания",
                        if uac2 { 6 } else { 5 },
                        2,
                    );
                    if !uac2 && let Some(&n) = x.get(7) {
                        v.bytes("interfaces", "Аудиоинтерфейсы", 8, n as usize);
                    }
                }
                2 => {
                    v.n("terminal_id", "ID терминала", 3, 1);
                    v.n("terminal_type", "Тип терминала", 4, 2);
                    v.n("associated", "Связанный терминал", 6, 1);
                    v.n("channels", "Число каналов", if uac2 { 8 } else { 7 }, 1);
                    v.n(
                        "channel_map",
                        "Карта каналов",
                        if uac2 { 9 } else { 8 },
                        if uac2 { 4 } else { 2 },
                    );
                    v.string(if uac2 { 13 } else { 10 });
                    v.string(if uac2 { 16 } else { 11 });
                    if uac2 {
                        v.n("clock", "Источник тактовой частоты", 7, 1);
                        v.n("controls", "Управление терминалом", 14, 2);
                    }
                }
                3 => {
                    v.n("terminal_id", "ID терминала", 3, 1);
                    v.n("terminal_type", "Тип терминала", 4, 2);
                    v.n("source", "ID источника", 7, 1);
                    v.string(if uac2 { 11 } else { 8 });
                    if uac2 {
                        v.n("clock", "Источник тактовой частоты", 8, 1);
                        v.n("controls", "Управление терминалом", 9, 2);
                    }
                }
                4 | 5 => {
                    v.n("unit_id", "ID блока", 3, 1);
                    if let Some(&n) = x.get(4) {
                        v.bytes("sources", "ID источников", 5, n as usize);
                    }
                    if x.len() > 5 {
                        v.string(x.len() - 1);
                    }
                }
                6 => {
                    v.n("unit_id", "ID блока", 3, 1);
                    v.n("source", "ID источника", 4, 1);
                    let start = if uac2 { 5 } else { 6 };
                    if x.len() > start {
                        v.bytes(
                            "controls",
                            "Управление каналами (bitmap)",
                            start,
                            x.len() - start - 1,
                        );
                        v.string(x.len() - 1);
                    }
                }
                0x0a if uac2 => {
                    v.n("clock_id", "ID тактового источника", 3, 1);
                    v.n("attributes", "Атрибуты тактового источника", 4, 1);
                    v.n("controls", "Управление частотой", 5, 1);
                    v.n("associated", "Связанный терминал", 6, 1);
                    v.string(7);
                }
                0x0b if uac2 => {
                    v.n("clock_id", "ID переключателя", 3, 1);
                    if let Some(&n) = x.get(4) {
                        v.bytes("sources", "Источники частоты", 5, n as usize);
                        v.string(6 + n as usize);
                    }
                }
                0x0c if uac2 => {
                    v.n("clock_id", "ID множителя", 3, 1);
                    v.n("source", "Источник частоты", 4, 1);
                    v.n("controls", "Управление множителем", 5, 1);
                    v.string(6);
                }
                _ => {}
            }
        }
        (1, 2, 0x24) if matches!(protocol, 0 | 0x20) => {
            v.add(
                "kind",
                "Назначение",
                if st == 1 {
                    "AudioStreaming General"
                } else {
                    "AudioStreaming Format"
                },
            );
            if st == 1 {
                v.n("terminal_link", "Связанный терминал", 3, 1);
                if protocol == 0 {
                    v.n("delay", "Задержка, кадров", 4, 1);
                    v.n("format_tag", "Формат аудио", 5, 2);
                } else {
                    v.n("formats", "Поддерживаемые форматы (bitmap)", 6, 4);
                    v.n("channels", "Число каналов", 10, 1);
                    v.n("channel_map", "Карта каналов", 11, 4);
                    v.string(15);
                }
            }
            if st == 2 && x.get(3) == Some(&1) {
                if protocol == 0 {
                    v.n("channels", "Число каналов", 4, 1);
                    v.n("sample_bytes", "Байт на отсчёт", 5, 1);
                    v.n("resolution", "Разрядность", 6, 1);
                    if let Some(&n) = x.get(7) {
                        let count = if n == 0 { 2 } else { n as usize };
                        for i in 0..count {
                            v.n(
                                &format!("frequency.{i}"),
                                if n == 0 {
                                    if i == 0 {
                                        "Минимальная частота, Гц"
                                    } else {
                                        "Максимальная частота, Гц"
                                    }
                                } else {
                                    "Частота дискретизации, Гц"
                                },
                                8 + 3 * i,
                                3,
                            );
                        }
                    }
                } else {
                    v.n("sample_bytes", "Байт на отсчёт", 4, 1);
                    v.n("resolution", "Разрядность", 5, 1);
                }
            }
        }
        (0x0e, 1, 0x24) => {
            v.add(
                "kind",
                "Назначение",
                match st {
                    1 => "UVC Header",
                    2 => "UVC Input Terminal",
                    3 => "UVC Output Terminal",
                    4 => "UVC Selector",
                    5 => "UVC Processing Unit",
                    6 => "UVC Extension Unit",
                    _ => "UVC control",
                },
            );
            match st {
                1 => {
                    v.version(3);
                    v.n("total_length", "Длина VideoControl-описания", 5, 2);
                    v.n("clock_hz", "Тактовая частота, Гц", 7, 4);
                    if let Some(&n) = x.get(11) {
                        v.bytes("interfaces", "Видеоинтерфейсы", 12, n as usize);
                    }
                }
                2 | 3 => {
                    v.n("terminal_id", "ID терминала", 3, 1);
                    v.n("terminal_type", "Тип терминала", 4, 2);
                    v.n("associated", "Связанный терминал", 6, 1);
                    v.string(if st == 2 { 7 } else { 8 });
                    if st == 3 {
                        v.n("source", "ID источника", 7, 1);
                    } else if u16_at(x, 4) == Some(0x0201) {
                        v.n("focal_min", "Минимальное фокусное расстояние", 8, 2);
                        v.n("focal_max", "Максимальное фокусное расстояние", 10, 2);
                        v.n("ocular", "Фокус окуляра", 12, 2);
                        if let Some(&n) = x.get(14) {
                            v.bytes("controls", "Управление камерой (bitmap)", 15, n as usize);
                        }
                    }
                }
                5 => {
                    v.n("unit_id", "ID блока", 3, 1);
                    v.n("source", "ID источника", 4, 1);
                    v.n("multiplier", "Максимальный множитель", 5, 2);
                    if let Some(&n) = x.get(7) {
                        v.bytes("controls", "Обработка изображения (bitmap)", 8, n as usize);
                        v.string(8 + n as usize);
                    }
                }
                _ => {}
            }
        }
        (0x0e, 2, 0x24) => {
            v.add(
                "kind",
                "Назначение",
                match st {
                    1 => "UVC Input Header",
                    2 => "UVC Output Header",
                    4 => "UVC Uncompressed Format",
                    5 => "UVC Uncompressed Frame",
                    6 => "UVC MJPEG Format",
                    7 => "UVC MJPEG Frame",
                    0x10 => "UVC Frame-based Format",
                    0x11 => "UVC Frame-based Frame",
                    0x0d => "UVC Color Matching",
                    _ => "UVC streaming",
                },
            );
            match st {
                1 | 2 => {
                    v.n("formats", "Число форматов", 3, 1);
                    v.n("total_length", "Длина streaming-описания", 4, 2);
                    v.n("endpoint", "Endpoint", 6, 1);
                    v.n(
                        "terminal_link",
                        "Связанный терминал",
                        if st == 1 { 8 } else { 7 },
                        1,
                    );
                }
                4 | 0x10 => {
                    v.n("format_index", "Номер формата", 3, 1);
                    v.n("frames", "Число размеров кадра", 4, 1);
                    v.bytes("format_guid", "GUID формата", 5, 16);
                    v.n("bits_pixel", "Бит на пиксель", 21, 1);
                    v.n("default_frame", "Кадр по умолчанию", 22, 1);
                }
                6 => {
                    v.n("format_index", "Номер формата", 3, 1);
                    v.n("frames", "Число размеров кадра", 4, 1);
                    v.n("default_frame", "Кадр по умолчанию", 6, 1);
                }
                5 | 7 | 0x11 => {
                    v.n("frame_index", "Номер кадра", 3, 1);
                    v.n("width", "Ширина, px", 5, 2);
                    v.n("height", "Высота, px", 7, 2);
                    v.n("bitrate_min", "Минимальный битрейт, бит/с", 9, 4);
                    v.n("bitrate_max", "Максимальный битрейт, бит/с", 13, 4);
                    let (default, count) = if st == 0x11 {
                        (17, 21)
                    } else {
                        v.n("buffer_bytes", "Максимальный размер кадра, байт", 17, 4);
                        (21, 25)
                    };
                    if let Some(t) = u32_at(x, default) {
                        v.add("interval", "Интервал по умолчанию, 100 нс", t);
                        if t > 0 {
                            v.add(
                                "fps",
                                "Кадров/с по умолчанию",
                                format!("{:.3}", 10_000_000f64 / f64::from(t)),
                            );
                        }
                    }
                    if let Some(&n) = x.get(count) {
                        for i in 0..if n == 0 { 3 } else { n as usize } {
                            v.n(
                                &format!("interval.{i}"),
                                if n == 0 {
                                    match i {
                                        0 => "Минимальный интервал, 100 нс",
                                        1 => "Максимальный интервал, 100 нс",
                                        _ => "Шаг интервала, 100 нс",
                                    }
                                } else {
                                    "Поддерживаемый интервал, 100 нс"
                                },
                                26 + 4 * i,
                                4,
                            );
                        }
                    }
                }
                0x0d => {
                    v.n("primaries", "Основные цвета", 3, 1);
                    v.n("transfer", "Передаточная характеристика", 4, 1);
                    v.n("matrix", "Матрица цветового пространства", 5, 1);
                }
                _ => {}
            }
        }
        _ => {
            v.add(
                "kind",
                "Расшифровка",
                "Неизвестный класс/версия; исходные байты сохранены",
            );
        }
    }
    (v.out, v.strings)
}

pub fn capability(x: &[u8], id: &str) -> Vec<Field> {
    let mut v = View {
        raw: x,
        id,
        group: "BOS",
        out: vec![],
        strings: vec![],
    };
    if x.len() < 3 {
        return v.out;
    }
    v.add(
        "name",
        "Возможность",
        match x[2] {
            2 => "USB 2.0 Extension",
            3 => "SuperSpeed USB",
            4 => "Container ID",
            5 => "Platform",
            10 => "SuperSpeedPlus",
            13 => "Precision Time Measurement",
            _ => "Другой capability",
        },
    );
    match x[2] {
        2 => {
            if let Some(a) = u32_at(x, 3) {
                v.add("lpm", "Link Power Management", a & 2 != 0);
                v.add("besl", "BESL поддерживается", a & 4 != 0);
                if a & 8 != 0 {
                    v.add("besl_baseline", "Baseline BESL (код)", (a >> 8) & 15);
                }
                if a & 16 != 0 {
                    v.add("besl_deep", "Deep BESL (код)", (a >> 12) & 15);
                }
            }
        }
        3 => {
            if let Some(a) = x.get(3) {
                v.add("ltm", "Latency Tolerance Messages", a & 2 != 0);
            }
            if let Some(s) = u16_at(x, 4) {
                v.add(
                    "speeds",
                    "Заявленные режимы",
                    ["Low-Speed", "Full-Speed", "High-Speed", "SuperSpeed"]
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| s & (1 << i) != 0)
                        .map(|(_, s)| *s)
                        .collect::<Vec<_>>()
                        .join(", "),
                );
            }
            v.n(
                "minimum_speed",
                "Минимальный режим для полной функциональности (код)",
                6,
                1,
            );
            v.n("u1_us", "Выход из U1, мкс", 7, 1);
            v.n("u2_us", "Выход из U2, мкс", 8, 2);
        }
        4 | 5 => {
            v.bytes(
                "uuid",
                if x[2] == 4 {
                    "Container ID"
                } else {
                    "Platform UUID"
                },
                4,
                16,
            );
            if x[2] == 4
                && let Some(f) = v.out.last_mut()
            {
                f.sensitive = true;
            }
        }
        10 => {
            if let Some(a) = u32_at(x, 4) {
                let count = (a & 31) + 1;
                v.add("sublinks", "Число sublink attributes", count);
                v.add("speed_ids", "Число speed IDs", ((a >> 5) & 15) + 1);
                if let Some(f) = u16_at(x, 8) {
                    v.add("min_speed_id", "Минимальный speed ID", f & 15);
                    v.add("min_rx_lanes", "Минимум RX-линий", (f >> 8) & 15);
                    v.add("min_tx_lanes", "Минимум TX-линий", (f >> 12) & 15);
                }
                for i in 0..count as usize {
                    if let Some(s) = u32_at(x, 12 + 4 * i) {
                        let bps = u64::from(s >> 16) * 1000u64.pow((s >> 4) & 3);
                        v.add(
                            &format!("sublink.{i}"),
                            "Заявленный sublink",
                            format!(
                                "ID={}; {}; {} бит/с; protocol={}; {}",
                                s & 15,
                                if s & 128 == 0 { "RX" } else { "TX" },
                                bps,
                                (s >> 14) & 3,
                                if s & 64 == 0 {
                                    "симметричный"
                                } else {
                                    "асимметричный"
                                }
                            ),
                        );
                    }
                }
            }
        }
        13 => v.add("ptm", "Precision Time Measurement", "Поддерживается"),
        _ => {}
    }
    v.out
}

/// Required bytes for the supported fixed prefixes and declared variable arrays.
/// An unknown subtype has no guessed layout.
pub fn required_length(x: &[u8], class: u8, subclass: u8, protocol: u8) -> Option<usize> {
    let st = *x.get(2)?;
    let n = |off: usize| x.get(off).copied().map(usize::from);
    match (class, subclass, *x.get(1)?) {
        (2, _, 0x24) => Some(match st {
            0 => 5,
            1 => 5,
            2 => 4,
            6 => 5,
            7 => 6,
            15 => 13,
            26 => 6,
            27 => 12,
            28 => 8,
            _ => return None,
        }),
        (1, 3, 0x24) if protocol == 0 => Some(match st {
            1 => 7,
            2 => 6,
            3 => 7 + 2 * n(5)?,
            _ => return None,
        }),
        (1, 3, 0x25) if protocol == 0 && st == 1 => Some(4 + n(3)?),
        (1, 1, 0x24) if matches!(protocol, 0 | 0x20) => Some(match (st, protocol) {
            (1, 0) => 8 + n(7)?,
            (1, 0x20) => 9,
            (2, 0) => 12,
            (2, 0x20) => 17,
            (3, 0) => 9,
            (3, 0x20) => 12,
            (5, 0) => 6 + n(4)?,
            (5, 0x20) => 7 + n(4)?,
            (6, 0) => 7 + n(5)?,
            (6, 0x20) => 10,
            (10, 0x20) => 8,
            (11, 0x20) => 7 + n(4)?,
            (12, 0x20) => 7,
            _ => return None,
        }),
        (1, 2, 0x24) if matches!(protocol, 0 | 0x20) => match st {
            1 => Some(if protocol == 0 { 7 } else { 16 }),
            2 if x.get(3) == Some(&1) => Some(if protocol == 0 {
                let n = n(7)?;
                8 + 3 * if n == 0 { 2 } else { n }
            } else {
                6
            }),
            _ => None,
        },
        (14, 1, 0x24) => Some(match st {
            1 => 12 + n(11)?,
            2 if u16_at(x, 4) == Some(0x0201) => 15 + n(14)?,
            2 => 8,
            3 => 9,
            5 => 9 + n(7)?,
            _ => return None,
        }),
        (14, 2, 0x24) => Some(match st {
            1 => 13 + n(3)? * n(12)?,
            2 => 9 + n(3)? * n(8)?,
            4 => 27,
            6 => 11,
            16 => 28,
            5 | 7 | 17 => {
                let n = n(if st == 17 { 21 } else { 25 })?;
                26 + 4 * if n == 0 { 3 } else { n }
            }
            13 => 6,
            _ => return None,
        }),
        _ => None,
    }
}
