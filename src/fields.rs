//! Presentation is a projection of the snapshot, never a collector input.
use crate::model::Device;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    pub id: String,
    pub label: String,
    pub group: String,
    pub value: String,
    pub source: String,
    pub sensitive: bool,
}
impl Field {
    pub fn display_value(&self) -> &str {
        if self.value.trim().is_empty() {
            "Пустое значение"
        } else {
            &self.value
        }
    }
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        group: &str,
        value: impl ToString,
        source: &str,
        sensitive: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            group: group.into(),
            value: value.to_string(),
            source: source.into(),
            sensitive,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct FieldSelection {
    pub all: bool,
    pub included: BTreeSet<String>,
    pub excluded: BTreeSet<String>,
}
impl Default for FieldSelection {
    fn default() -> Self {
        Self {
            all: false,
            excluded: BTreeSet::new(),
            included: [
                "name",
                "kind",
                "speed",
                "capability",
                "vid",
                "pid",
                "path",
                "port",
                "usb.address",
                "windows_problem",
                "service",
                "usb.bcdUSB",
                "usb.bcdDevice",
                "usb.serial",
                "usb.manufacturer",
                "usb.product",
                "windows.DriverVersion",
                "windows.DriverDate",
                "windows.power.state",
                "usb.config.0.max_power",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        }
    }
}
impl FieldSelection {
    pub fn all() -> Self {
        Self {
            all: true,
            excluded: BTreeSet::new(),
            included: BTreeSet::new(),
        }
    }
    pub fn contains(&self, id: &str) -> bool {
        if self.all {
            !self.excluded.contains(id)
        } else {
            self.included.contains(id)
        }
    }
    pub fn set(&mut self, id: &str, on: bool, _available: &[Field]) {
        if self.all {
            if on {
                self.excluded.remove(id);
            } else {
                self.excluded.insert(id.into());
            }
        } else if on {
            self.included.insert(id.into());
        } else {
            self.included.remove(id);
        }
    }
    pub fn none() -> Self {
        Self {
            all: false,
            included: BTreeSet::new(),
            excluded: BTreeSet::new(),
        }
    }
    /// Old positional function IDs cannot be safely mapped to a physical node.
    pub fn remove_legacy_ids(&mut self) {
        let old = |id: &String| {
            ["function.", "interface.", "disk."].iter().any(|prefix| {
                id.strip_prefix(prefix)
                    .and_then(|s| s.split('.').next())
                    .is_some_and(|n| n.parse::<usize>().is_ok())
            })
        };
        self.included.retain(|id| !old(id));
        self.excluded.retain(|id| !old(id));
    }
    pub fn preset(group: &str, fields: &[Field]) -> Self {
        let mut s = Self::none();
        for f in fields {
            let yes = match group {
                "Подключение" => matches!(
                    f.id.as_str(),
                    "name"
                        | "kind"
                        | "vid"
                        | "pid"
                        | "path"
                        | "port"
                        | "speed"
                        | "capability"
                        | "port_protocols"
                ),
                "Питание" => f.id.to_lowercase().contains("power") || f.group == "Питание",
                "Драйвер" => {
                    f.id.contains("Driver") || f.id == "service" || f.id == "windows_problem"
                }
                "Дескрипторы" => {
                    f.id.starts_with("usb.") || f.group == "HID" || f.group == "BOS"
                }
                _ => false,
            };
            if yes {
                s.included.insert(f.id.clone());
            }
        }
        s
    }
}
/// Opaque, deterministic identity: no serial or instance path in exported field IDs.
pub fn stable_id(identity: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(identity.to_uppercase().as_bytes()))
}
pub fn optional_number(value: Option<u32>) -> String {
    value
        .map(|n| n.to_string())
        .unwrap_or_else(|| "Не получено".into())
}
pub fn connection_status(value: i32) -> String {
    match value {
        0 => "Свободен",
        1 => "Подключено",
        2 => "Не удалось перечислить устройство",
        3 => "Общая ошибка",
        4 => "Перегрузка по току",
        5 => "Недостаточно питания",
        6 => "Недостаточно пропускной способности",
        7 => "Слишком много уровней хабов",
        8 => "Устаревший хаб",
        9 => "Идёт перечисление",
        10 => "Устройство сбрасывается",
        _ => return format!("Неизвестное состояние ({value})"),
    }
    .into()
}

pub fn base_fields(d: &Device) -> Vec<Field> {
    let opt = |x: Option<u32>| {
        x.map(|n| n.to_string())
            .unwrap_or_else(|| "Не получено".into())
    };
    vec![
        Field::new(
            "name",
            "Название",
            "Основное",
            &d.name,
            &d.name_source,
            true,
        ),
        Field::new(
            "kind",
            "Тип",
            "Основное",
            d.kind(),
            "Device descriptor",
            false,
        ),
        Field::new(
            "speed",
            "Текущий режим",
            "Основное",
            d.speed_label(),
            "EX + EX_V2",
            false,
        ),
        Field::new(
            "capability",
            "SuperSpeed поддерживается",
            "Основное",
            match d.speed.super_capable() {
                Some(true) => "Да",
                Some(false) => "Не сообщается",
                None => "Неизвестно",
            },
            "EX_V2",
            false,
        ),
        Field::new(
            "vid",
            "VID",
            "Основное",
            d.vendor_id
                .map(|v| format!("{v:04X}"))
                .unwrap_or_else(|| "?".into()),
            "Device descriptor",
            false,
        ),
        Field::new(
            "pid",
            "PID",
            "Основное",
            d.product_id
                .map(|v| format!("{v:04X}"))
                .unwrap_or_else(|| "?".into()),
            "Device descriptor",
            false,
        ),
        Field::new(
            "path",
            "Путь подключения",
            "Основное",
            d.path.join(" / "),
            "Windows PnP / hub",
            true,
        ),
        Field::new(
            "port",
            "Порт",
            "Основное",
            opt(d.port),
            "Hub connection index",
            false,
        ),
        Field::new(
            "windows_problem",
            "Код проблемы Windows",
            "Основное",
            if d.windows_problem == Some(0) {
                "Нет ошибок (0)".into()
            } else {
                opt(d.windows_problem)
            },
            "CM_Get_DevNode_Status",
            false,
        ),
        Field::new(
            "functions",
            "Функции устройства",
            "Основное",
            crate::inventory::functions(d).join(" + "),
            "Configuration / HID capabilities",
            false,
        ),
        Field::new(
            "connection_status",
            "Состояние подключения",
            "Основное",
            crate::inventory::Column::Status.value(d),
            "EX / Windows / query status",
            false,
        ),
        Field::new(
            "service",
            "Служба драйвера",
            "Основное",
            d.service.as_deref().unwrap_or("Не получена"),
            "SPDRP_SERVICE",
            true,
        ),
        Field::new(
            "instance",
            "Instance ID",
            "Windows",
            &d.key,
            "SetupAPI",
            true,
        ),
        Field::new(
            "ex_speed",
            "EX.Speed",
            "USB · сырые поля",
            d.speed
                .ex_speed
                .map(|x| x.to_string())
                .unwrap_or_else(|| "Не получено".into()),
            "EX",
            false,
        ),
        Field::new(
            "v2_flags",
            "EX_V2 flags",
            "USB · сырые поля",
            opt(d.speed.v2_flags),
            "EX_V2",
            false,
        ),
        Field::new(
            "port_protocols",
            "Протоколы порта",
            "USB · сырые поля",
            opt(d.speed.port_protocols),
            "EX_V2",
            false,
        ),
    ]
}
pub fn for_device(d: &Device) -> Vec<Field> {
    let mut result = base_fields(d);
    result.extend(d.fields.iter().cloned());
    let mut ids = BTreeSet::new();
    result.retain(|f| ids.insert(f.id.clone()));
    result
        .sort_by(|a, b| (a.group != "Основное", &a.group).cmp(&(b.group != "Основное", &b.group)));
    result
}
pub fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}
