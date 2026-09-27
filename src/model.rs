use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Speed {
    Low,
    Full,
    High,
    Super,
    SuperPlus,
    Unknown,
}
impl Speed {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Low => "1,5 Мбит/с · Low-Speed",
            Self::Full => "12 Мбит/с · Full-Speed",
            Self::High => "480 Мбит/с · High-Speed",
            Self::Super => "5 Гбит/с · SuperSpeed",
            Self::SuperPlus => "SuperSpeedPlus · точная скорость неизвестна",
            Self::Unknown => "Неизвестно",
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeedEvidence {
    pub ex_speed: Option<u8>,
    /// None means the query failed or was not made, NOT all flags clear.
    pub v2_flags: Option<u32>,
    pub port_protocols: Option<u32>,
}
impl SpeedEvidence {
    pub fn speed(&self) -> Speed {
        let operating = self.v2_flags.map(|f| f & 5 != 0).unwrap_or(false);
        if operating && matches!(self.ex_speed, Some(0 | 1)) {
            return Speed::Unknown;
        }
        if let Some(f) = self.v2_flags {
            // Reserved bits or operating without matching capability: retain unknown.
            if f & !15 != 0 || (f & 1 != 0 && f & 2 == 0) || (f & 4 != 0 && f & 8 == 0) {
                return Speed::Unknown;
            }
            if f & 4 != 0 {
                return Speed::SuperPlus;
            }
            if f & 1 != 0 {
                return Speed::Super;
            }
        }
        match self.ex_speed {
            Some(0) => Speed::Low,
            Some(1) => Speed::Full,
            // EX alone cannot distinguish High-Speed from SuperSpeed.
            Some(2) if self.v2_flags.is_some() => Speed::High,
            _ => Speed::Unknown,
        }
    }
    pub fn super_capable(&self) -> Option<bool> {
        self.v2_flags.filter(|f| f & !15 == 0).map(|f| f & 10 != 0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    pub operation: String,
    pub code: Option<u32>,
    pub reason: String,
}
impl Issue {
    pub fn new(operation: &str, code: Option<u32>, reason: &str) -> Self {
        Self {
            operation: operation.into(),
            code,
            reason: reason.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    #[serde(default)]
    pub fields: Vec<crate::fields::Field>,
    #[serde(default)]
    pub descriptors: Vec<crate::descriptors::Descriptor>,
    /// Internal instance/port identity, never emitted in a redacted report.
    pub key: String,
    pub name: String,
    pub name_source: String,
    pub path: Vec<String>,
    pub port: Option<u32>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub device_class: Option<u8>,
    pub is_hub: bool,
    pub connection_status: Option<i32>,
    pub windows_problem: Option<u32>,
    pub service: Option<String>,
    pub speed: SpeedEvidence,
    pub issues: Vec<Issue>,
}
impl Device {
    pub fn invalidate_connection(&mut self) {
        self.fields.clear();
        self.descriptors.clear();
        self.name = "Устройство изменилось во время опроса".into();
        self.name_source = "Согласованный снимок недоступен".into();
        self.vendor_id = None;
        self.product_id = None;
        self.device_class = None;
        self.connection_status = None;
        self.windows_problem = None;
        self.service = None;
        self.speed = SpeedEvidence::default();
        self.issues.push(Issue::new(
            "Connection consistency",
            None,
            "Повторный ответ отличается или недоступен. Сведения разных моментов не объединены.",
        ));
    }

    pub fn speed_label(&self) -> String {
        if self.speed.speed() == Speed::SuperPlus {
            let rx = self.fields.iter().find(|f| f.id == "usb.ssp.RX.rate");
            let tx = self.fields.iter().find(|f| f.id == "usb.ssp.TX.rate");
            if let (Some(rx), Some(tx)) = (rx, tx) {
                return if rx.value == tx.value {
                    format!("{} · SuperSpeedPlus", rx.value)
                } else {
                    format!("RX {} / TX {} · SuperSpeedPlus", rx.value, tx.value)
                };
            }
        }
        self.speed.speed().label().into()
    }
    pub fn matches_search(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        let text = format!(
            "{} {} {} {} {} {} {}",
            self.name,
            self.kind(),
            self.key,
            self.path.join(" / "),
            self.vendor_id
                .map(|n| format!("{n:04x} vid_{n:04x}"))
                .unwrap_or_default(),
            self.product_id
                .map(|n| format!("{n:04x} pid_{n:04x}"))
                .unwrap_or_default(),
            self.port.map(|n| format!("порт {n}")).unwrap_or_default()
        )
        .to_lowercase();
        query.split_whitespace().all(|part| {
            text.contains(part)
                || self
                    .fields
                    .iter()
                    .any(|f| f.value.to_lowercase().contains(part))
        })
    }
    pub fn kind(&self) -> &'static str {
        if self.is_hub {
            return "USB-хаб";
        }
        if self
            .fields
            .iter()
            .any(|f| f.id == "node.kind" && f.value == "Host controller")
        {
            return "USB-контроллер";
        }
        match self.device_class {
            Some(1) => "Аудиоустройство",
            Some(3) => "Устройство ввода",
            Some(8) => "Накопитель",
            Some(14) => "Камера",
            Some(0 | 239) => "USB-устройство · класс задаётся интерфейсами",
            _ => "USB-устройство",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub captured_unix_ms: u64,
    pub demo: bool,
    pub inventory_complete: bool,
    pub devices: Vec<Device>,
    #[serde(default)]
    pub topology: Vec<Device>,
    pub issues: Vec<Issue>,
}
impl Snapshot {
    /// Tree nodes are deduplicated by Windows identity, never by display name.
    /// Missing parents remain roots; cycles are bounded and every node stays visible.
    pub fn navigation_rows(&self, tree: bool) -> Vec<(&Device, usize)> {
        if !tree {
            return self.devices.iter().map(|d| (d, 0)).collect();
        }
        ordered_tree(self.devices.iter().chain(&self.topology))
    }
    pub fn visible_selection(&self, tree: bool, current: usize, query: &str) -> Option<usize> {
        let rows = self.navigation_rows(tree);
        if rows
            .get(current)
            .is_some_and(|(d, _)| d.matches_search(query))
        {
            Some(current)
        } else {
            rows.iter().position(|(d, _)| d.matches_search(query))
        }
    }
    pub fn selected_node(&self, tree: bool, index: usize) -> Option<&Device> {
        self.navigation_rows(tree).get(index).map(|(d, _)| *d)
    }

    pub fn empty(demo: bool) -> Self {
        Self {
            schema_version: 1,
            captured_unix_ms: now_ms(),
            demo,
            inventory_complete: true,
            devices: Vec::new(),
            topology: Vec::new(),
            issues: Vec::new(),
        }
    }
}
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

pub fn ordered_tree<'a>(devices: impl IntoIterator<Item = &'a Device>) -> Vec<(&'a Device, usize)> {
    let mut nodes = std::collections::BTreeMap::new();
    for d in devices {
        nodes.insert(d.key.clone(), d);
    }
    let mut children: std::collections::BTreeMap<Option<String>, Vec<String>> =
        std::collections::BTreeMap::new();
    for (key, d) in &nodes {
        let parent = d
            .fields
            .iter()
            .find(|f| f.id == "node.parent_key")
            .map(|f| f.value.clone())
            .filter(|p| p != key && nodes.contains_key(p));
        children.entry(parent).or_default().push(key.clone());
    }
    for list in children.values_mut() {
        list.sort_by_key(|key| {
            let d = nodes[key];
            (d.port.unwrap_or(0), d.name.clone(), key.clone())
        });
    }
    let mut rows = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let roots = children.get(&None).cloned().unwrap_or_default();
    // Start orphan/cyclic components after ordinary roots.
    for root in roots.into_iter().chain(nodes.keys().cloned()) {
        let mut stack = vec![(root, 0)];
        while let Some((key, depth)) = stack.pop() {
            if !seen.insert(key.clone()) {
                continue;
            }
            rows.push((nodes[&key], depth));
            if let Some(kids) = children.get(&Some(key)) {
                for k in kids.iter().rev() {
                    stack.push((k.clone(), depth + 1));
                }
            }
        }
    }
    rows
}
