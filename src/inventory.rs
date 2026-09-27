//! Presentation-only inventory: no filter is passed to the collector.
use crate::model::{Device, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Clone, Copy, Debug, Ord, PartialOrd, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Column {
    Kind,
    Speed,
    Port,
    Address,
    Ids,
    Status,
}
impl Column {
    pub const ALL: [Self; 6] = [
        Self::Kind,
        Self::Speed,
        Self::Port,
        Self::Address,
        Self::Ids,
        Self::Status,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Kind => "Тип / функции",
            Self::Speed => "Режим USB",
            Self::Port => "Порт",
            Self::Address => "USB-адрес",
            Self::Ids => "VID:PID",
            Self::Status => "Состояние",
        }
    }
    pub fn width(self) -> f32 {
        match self {
            Self::Kind => 145.0,
            Self::Speed => 160.0,
            Self::Port => 35.0,
            Self::Address => 65.0,
            Self::Ids => 85.0,
            Self::Status => 175.0,
        }
    }
    pub fn value(self, d: &Device) -> String {
        match self {
            Self::Kind => functions(d).join(" + "),
            Self::Speed => {
                if d.port.is_none() {
                    "—".into()
                } else {
                    d.speed_label()
                }
            }
            Self::Port => d.port.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
            Self::Address => d
                .fields
                .iter()
                .find(|f| f.id == "usb.address")
                .map(|f| f.value.clone())
                .unwrap_or_else(|| "—".into()),
            Self::Ids => vid_pid(d),
            Self::Status => {
                if d.windows_problem.is_some_and(|n| n != 0) {
                    format!("Код Windows {}", d.windows_problem.unwrap())
                } else if d.connection_status.is_some_and(|s| s != 1) {
                    crate::fields::connection_status(d.connection_status.unwrap())
                } else if !d.issues.is_empty() {
                    "Подключено · часть данных недоступна".into()
                } else {
                    "Подключено".into()
                }
            }
        }
    }
}
pub fn vid_pid(d: &Device) -> String {
    match (d.vendor_id, d.product_id) {
        (Some(v), Some(p)) => format!("{v:04X}:{p:04X}"),
        _ => "—".into(),
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub grouped: bool,
    pub show_hubs: bool,
    pub columns: BTreeSet<Column>,
    pub collapsed: BTreeSet<String>,
    pub highlight_seconds: u64,
    pub follow_changes: bool,
    pub widths: std::collections::BTreeMap<String, f32>,
    pub extra_columns: std::collections::BTreeMap<String, String>,
    pub theme: ThemeChoice,
    pub language: crate::i18n::Language,
    pub column_order: Vec<String>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            grouped: true,
            show_hubs: false,
            columns: Column::ALL.into_iter().collect(),
            collapsed: BTreeSet::new(),
            highlight_seconds: 5,
            follow_changes: true,
            widths: Default::default(),
            extra_columns: Default::default(),
            theme: ThemeChoice::System,
            language: Default::default(),
            column_order: Vec::new(),
        }
    }
}
pub struct Row<'a> {
    pub device: &'a Device,
    pub depth: usize,
    pub children: usize,
    pub added: usize,
    pub removed: usize,
    pub group: bool,
}
pub fn rows<'a>(s: &'a Snapshot, o: &Options, query: &str, changes: &'a Changes) -> Vec<Row<'a>> {
    let ghosts = changes
        .entries
        .values()
        .filter(|c| {
            c.kind == ChangeKind::Removed
                && !s
                    .devices
                    .iter()
                    .chain(&s.topology)
                    .any(|d| d.key == c.device.key)
        })
        .map(|c| &c.device);
    let source = if o.grouped {
        crate::model::ordered_tree(s.devices.iter().chain(&s.topology).chain(ghosts))
    } else {
        s.devices.iter().chain(ghosts).map(|d| (d, 0)).collect()
    };
    let mut keep = BTreeSet::new();
    let mut ancestors: Vec<(usize, usize)> = vec![];
    for (i, (d, depth)) in source.iter().enumerate() {
        while ancestors.last().is_some_and(|(_, n)| n >= depth) {
            ancestors.pop();
        }
        let structural = d.is_hub || d.kind() == "USB-контроллер";
        let wanted = if o.grouped {
            !structural
                || o.show_hubs
                || (d.is_hub && (d.port.is_some() || d.vendor_id.is_some()))
                || !query.trim().is_empty()
        } else {
            o.show_hubs || !d.is_hub
        };
        if d.matches_search(query) && wanted {
            keep.insert(i);
            for (parent, _) in &ancestors {
                keep.insert(*parent);
            }
        }
        ancestors.push((i, *depth));
    }
    let mut closed_depth = None;
    let mut result = vec![];
    for (i, (d, depth)) in source.iter().enumerate() {
        if closed_depth.is_some_and(|n| *depth <= n) {
            closed_depth = None;
        }
        if closed_depth.is_some() || !keep.contains(&i) {
            continue;
        }
        let children = if o.grouped {
            source[i + 1..]
                .iter()
                .take_while(|(_, n)| n > depth)
                .filter(|(d, _)| {
                    !d.is_hub
                        && d.kind() != "USB-контроллер"
                        && !changes
                            .entries
                            .get(&d.key)
                            .is_some_and(|c| c.kind == ChangeKind::Removed)
                })
                .count()
        } else {
            0
        };
        let group = o.grouped && (d.is_hub || d.kind() == "USB-контроллер");
        let descendants: Vec<_> = source[i + 1..]
            .iter()
            .take_while(|(_, n)| n > depth)
            .filter_map(|(d, _)| changes.entries.get(&d.key))
            .collect();
        let added = descendants
            .iter()
            .filter(|c| c.kind == ChangeKind::Added)
            .count();
        let removed = descendants
            .iter()
            .filter(|c| c.kind == ChangeKind::Removed)
            .count();
        result.push(Row {
            added,
            removed,
            device: d,
            depth: *depth,
            children,
            group,
        });
        if group && query.trim().is_empty() && o.collapsed.contains(&d.key) {
            closed_depth = Some(*depth);
        }
    }
    result
}
pub fn functions(d: &Device) -> Vec<String> {
    if d.is_hub || d.kind() == "USB-контроллер" {
        return vec![d.kind().into()];
    }
    let mut classes = BTreeSet::new();
    if let Some(c) = d.device_class {
        classes.insert(c);
    }
    let active = d
        .fields
        .iter()
        .find(|f| f.id == "usb.configuration_value")
        .and_then(|f| f.value.parse::<u8>().ok());
    for desc in d
        .descriptors
        .iter()
        .filter(|x| x.kind == "Configuration" && active.is_none_or(|v| x.raw.get(5) == Some(&v)))
    {
        let mut off = 0;
        while off + 2 <= desc.raw.len() {
            let len = desc.raw[off] as usize;
            if len < 2 || off + len > desc.raw.len() {
                break;
            }
            let b = &desc.raw[off..off + len];
            if b[1] == 4 && len >= 9 {
                classes.insert(b[5]);
            }
            off += len;
        }
    }
    let mut names: BTreeSet<String> = classes
        .into_iter()
        .filter_map(|c| match c {
            1 => Some("Аудио"),
            2 | 10 => Some("CDC / связь"),
            3 => Some("Устройство ввода"),
            6 => Some("Фото / MTP"),
            8 => Some("Накопитель"),
            9 => Some("Хаб"),
            14 => Some("Камера"),
            _ => None,
        })
        .map(str::to_owned)
        .collect();
    let mut keyboard = false;
    let mut mouse = false;
    for f in d.fields.iter().filter(|f| f.id.ends_with("hid.usage")) {
        let prefix = f.id.strip_suffix("usage").unwrap_or("");
        if d.fields
            .iter()
            .any(|p| p.id == format!("{prefix}usage_page") && p.value == "1")
        {
            keyboard |= f.value == "6";
            mouse |= f.value == "2";
        }
    }
    if keyboard || mouse {
        names.remove("Устройство ввода");
        if keyboard {
            names.insert("Клавиатура".into());
        }
        if mouse {
            names.insert("Мышь".into());
        }
    }
    for f in &d.fields {
        if f.id.ends_with("com_port") {
            names.insert(format!("COM · {}", f.value));
        }
    }
    if names.is_empty() {
        names.insert("USB-устройство".into());
    }
    names.into_iter().collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChangeKind {
    Added,
    Removed,
}
pub struct Change {
    pub device: Device,
    pub kind: ChangeKind,
    pub at_ms: u64,
}
#[derive(Default)]
pub struct Changes {
    pub entries: std::collections::BTreeMap<String, Change>,
}
impl Changes {
    pub fn expire(&mut self, now: u64, seconds: u64) {
        self.entries
            .retain(|_, c| now.saturating_sub(c.at_ms) < seconds.saturating_mul(1000));
    }
    pub fn update(&mut self, old: &Snapshot, new: &Snapshot, now: u64) {
        if old.demo != new.demo {
            self.entries.clear();
            return;
        }
        // Positive observations can remove a stale tombstone even in an incomplete scan.
        self.entries.retain(|key, c| {
            c.kind != ChangeKind::Removed || !new.devices.iter().any(|d| d.key == *key)
        });
        if !old.inventory_complete || !new.inventory_complete {
            return;
        }
        for d in &new.devices {
            if !old.devices.iter().any(|p| p.key == d.key) {
                self.entries.insert(
                    d.key.clone(),
                    Change {
                        device: d.clone(),
                        kind: ChangeKind::Added,
                        at_ms: now,
                    },
                );
            }
        }
        for d in &old.devices {
            if !new.devices.iter().any(|p| p.key == d.key) {
                self.entries.insert(
                    d.key.clone(),
                    Change {
                        device: d.clone(),
                        kind: ChangeKind::Removed,
                        at_ms: now,
                    },
                );
            }
        }
    }
}

/// Expand only the target branch; unrelated collapsed hubs retain their state.
pub fn reveal(s: &Snapshot, o: &mut Options, changes: &Changes, key: &str) {
    let mut open = o.clone();
    open.collapsed.clear();
    let all = rows(s, &open, "", changes);
    let mut ancestors: Vec<(String, usize)> = vec![];
    for row in all {
        while ancestors
            .last()
            .is_some_and(|(_, depth)| *depth >= row.depth)
        {
            ancestors.pop();
        }
        if row.device.key == key {
            for (parent, _) in ancestors {
                o.collapsed.remove(&parent);
            }
            return;
        }
        ancestors.push((row.device.key.clone(), row.depth));
    }
}

#[derive(Clone)]
pub struct DisplayColumn {
    pub standard: Option<Column>,
    pub id: String,
    pub title: String,
}
impl DisplayColumn {
    pub fn label(&self) -> &str {
        &self.title
    }
    pub fn key(&self) -> &str {
        &self.id
    }
    pub fn width(&self) -> f32 {
        if self.id == "name" {
            380.0
        } else {
            self.standard.map_or(170.0, Column::width)
        }
    }
    pub fn minimum(&self) -> f32 {
        if self.id == "name" {
            return 160.0;
        }
        if matches!(self.standard, Some(Column::Port | Column::Address)) {
            35.0
        } else {
            65.0
        }
    }
    pub fn value(&self, d: &Device) -> String {
        if let Some(c) = self.standard {
            return c.value(d);
        }
        let id = self.id.strip_prefix("field:").unwrap_or(&self.id);
        if let Some(f) = crate::fields::base_fields(d).iter().find(|f| f.id == id) {
            return f.display_value().into();
        }
        d.fields
            .iter()
            .find(|f| f.id == id)
            .map(|f| f.display_value().to_owned())
            .unwrap_or_else(|| "—".into())
    }
}
pub fn columns(o: &Options) -> Vec<DisplayColumn> {
    Column::ALL
        .into_iter()
        .filter(|c| {
            o.columns.contains(c)
                || (*c == Column::Address && o.extra_columns.contains_key("usb.address"))
        })
        .map(|c| DisplayColumn {
            standard: Some(c),
            id: c.label().into(),
            title: c.label().into(),
        })
        .chain(
            o.extra_columns
                .iter()
                .filter(|(id, _)| id.as_str() != "usb.address")
                .map(|(id, title)| DisplayColumn {
                    standard: None,
                    id: format!("field:{id}"),
                    title: title.clone(),
                }),
        )
        .collect()
}
pub fn field_catalog(s: &Snapshot) -> Vec<crate::fields::Field> {
    let mut fields = std::collections::BTreeMap::new();
    for d in s.devices.iter().chain(&s.topology) {
        for mut f in crate::fields::for_device(d) {
            f.value.clear();
            fields.entry(f.id.clone()).or_insert(f);
        }
    }
    let mut list: Vec<_> = fields.into_values().collect();
    list.sort_by(|a, b| {
        (a.group != "Основное", &a.group, &a.label, &a.id).cmp(&(
            b.group != "Основное",
            &b.group,
            &b.label,
            &b.id,
        ))
    });
    list
}

pub fn display_name(d: &Device) -> &str {
    if d.name_source == "Windows FriendlyName" {
        return &d.name;
    }
    d.fields
        .iter()
        .find(|f| {
            f.id == "usb.product" && f.source == "String descriptor" && !f.value.trim().is_empty()
        })
        .map(|f| f.value.as_str())
        .unwrap_or(&d.name)
}

pub fn export_selection(o: &Options) -> crate::fields::FieldSelection {
    let mut s = crate::fields::FieldSelection::none();
    s.included.insert("name".into());
    for c in &o.columns {
        for id in match c {
            Column::Kind => vec!["functions"],
            Column::Speed => vec!["speed"],
            Column::Port => vec!["port"],
            Column::Address => vec!["usb.address"],
            Column::Ids => vec!["vid", "pid"],
            Column::Status => vec!["connection_status"],
        } {
            s.included.insert(id.into());
        }
    }
    s.included.extend(o.extra_columns.keys().cloned());
    s
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}
impl Options {
    pub fn field_selected(&self, id: &str) -> bool {
        if id == "usb.address" {
            self.columns.contains(&Column::Address) || self.extra_columns.contains_key(id)
        } else {
            self.extra_columns.contains_key(id)
        }
    }
    pub fn set_field(&mut self, id: &str, label: &str, on: bool) {
        if id == "usb.address" {
            self.extra_columns.remove(id);
            if on {
                self.columns.insert(Column::Address);
            } else {
                self.columns.remove(&Column::Address);
            }
        } else if on {
            self.extra_columns.insert(id.into(), label.into());
        } else {
            self.extra_columns.remove(id);
        }
    }

    pub fn normalize(&mut self) {
        self.highlight_seconds = 5;
        if self.extra_columns.remove("usb.address").is_some() {
            self.columns.insert(Column::Address);
        }
        if let Some(w) = self.widths.remove("Адрес USB") {
            self.widths.entry("USB-адрес".into()).or_insert(w);
        }
        for key in &mut self.column_order {
            if key == "Адрес USB" || key == "field:usb.address" {
                *key = "USB-адрес".into();
            }
        }
    }
    pub fn reorder(&mut self, source: &str, target: &str) {
        let mut keys: Vec<String> = display_columns(self).iter().map(|c| c.id.clone()).collect();
        let Some(from) = keys.iter().position(|s| s == source) else {
            return;
        };
        let Some(to) = keys.iter().position(|s| s == target) else {
            return;
        };
        let key = keys.remove(from);
        keys.insert(if from < to { to - 1 } else { to }, key);
        self.column_order = keys;
    }
}
pub fn display_columns(o: &Options) -> Vec<DisplayColumn> {
    let mut c = vec![DisplayColumn {
        standard: None,
        id: "name".into(),
        title: "Устройство / хаб".into(),
    }];
    c.extend(columns(o));
    c.sort_by_key(|c| {
        o.column_order
            .iter()
            .position(|key| key == &c.id)
            .unwrap_or(usize::MAX)
    });
    c
}
