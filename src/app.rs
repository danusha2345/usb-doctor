use eframe::egui::{self, Color32, RichText};
use std::{
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};
use usb_doctor::{
    demo,
    diagnostics::{Tone, assess, compare},
    model::{Device, Snapshot},
    report,
};
const TEAL: Color32 = Color32::from_rgb(24, 106, 98);
const MUTED: Color32 = Color32::from_rgb(89, 107, 121);
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Inventory,
    Overview,
    Compare,
    History,
    Details,
}
#[derive(Clone)]
struct ColumnDrag(String);
struct Baseline {
    device: Device,
    demo: bool,
    time: u64,
    generation: u64,
}
pub struct Doctor {
    demo: bool,
    snapshot: Option<Snapshot>,
    selected: usize,
    search: String,
    tab: Tab,
    baseline: Option<Baseline>,
    identity_confirmed: bool,
    job: Option<usb_doctor::scan_job::ScanJob>,
    presence_rx: Option<Receiver<(u64, Result<Snapshot, String>)>>,
    last_probe: Instant,
    epoch: u64,
    job_epoch: u64,
    detailed_dirty: bool,
    manual_refresh: bool,
    full_not_before: Instant,
    probe_again: bool,
    catalog_dirty: bool,
    started: Option<Instant>,
    error: Option<String>,
    notice: Option<String>,
    history: Vec<String>,
    generation: u64,
    selection: usb_doctor::fields::FieldSelection,
    fields_open: bool,
    field_search: String,
    show_topology: bool,
    include_sensitive: bool,
    monitor: Option<usb_doctor::monitor::Monitor>,
    monitor_status: String,
    auto_refresh: bool,
    inventory: usb_doctor::inventory::Options,
    changes: usb_doctor::inventory::Changes,
    removed_selection: Option<Device>,
    last_refresh: Instant,
    pending_reveal: Option<String>,
    inventory_dirty: bool,
    column_picker_open: bool,
    column_search: String,
    field_catalog: Vec<usb_doctor::fields::Field>,
    table_keys: Vec<String>,
    table_scroll: f32,
    table_scroll_x: f32,
    export_draft: Option<(Snapshot, usb_doctor::fields::FieldSelection, bool)>,
}
fn time(ms: u64) -> String {
    let s = ms / 1000;
    format!("{:02}:{:02}:{:02} UTC", s / 3600 % 24, s / 60 % 60, s % 60)
}
fn muted(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.label(RichText::new(text).color(if ui.visuals().dark_mode {
        Color32::from_gray(175)
    } else {
        MUTED
    }));
}
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(title).strong().size(14.0));
    ui.add_space(3.0);
}
impl Doctor {
    pub fn new(cc: &eframe::CreationContext<'_>, demo: bool) -> Self {
        crate::appearance::install(&cc.egui_ctx, load_inventory().theme);
        let (monitor, monitor_status) = if demo {
            (None, "Демо".into())
        } else {
            match usb_doctor::monitor::Monitor::with_wake({
                let ctx = cc.egui_ctx.clone();
                Box::new(move || ctx.request_repaint())
            }) {
                Ok(m) => (Some(m), "PnP: подключение и отключение".into()),
                Err(e) => (None, e),
            }
        };
        let mut app = Self {
            demo,
            snapshot: None,
            selected: 0,
            search: String::new(),
            tab: Tab::Inventory,
            baseline: None,
            identity_confirmed: false,
            job: None,
            presence_rx: None,
            last_probe: Instant::now(),
            epoch: 0,
            job_epoch: 0,
            detailed_dirty: false,
            manual_refresh: false,
            full_not_before: Instant::now(),
            probe_again: false,
            catalog_dirty: true,
            started: None,
            error: None,
            notice: None,
            history: Vec::new(),
            generation: 0,
            selection: load_selection(),
            fields_open: false,
            field_search: String::new(),
            show_topology: load_inventory().grouped,
            include_sensitive: false,
            monitor,
            monitor_status,
            auto_refresh: true,
            inventory: load_inventory(),
            changes: Default::default(),
            removed_selection: None,
            last_refresh: Instant::now(),
            pending_reveal: None,
            inventory_dirty: false,
            column_picker_open: false,
            column_search: String::new(),
            field_catalog: vec![],
            table_keys: vec![],
            table_scroll: 0.0,
            table_scroll_x: 0.0,
            export_draft: None,
        };
        app.refresh(&cc.egui_ctx);
        app
    }
    fn accept(&mut self, snapshot: Snapshot) {
        self.catalog_dirty = true;
        self.last_refresh = Instant::now();
        if let Some(old) = &self.snapshot {
            let now = usb_doctor::model::now_ms();
            self.changes.update(old, &snapshot, now);
            if self.inventory.follow_changes
                && let Some(change) = self
                    .changes
                    .entries
                    .values()
                    .filter(|c| c.at_ms == now)
                    .min_by_key(|c| {
                        (
                            c.kind == usb_doctor::inventory::ChangeKind::Removed,
                            c.device.is_hub,
                            &c.device.key,
                        )
                    })
            {
                let key = change.device.key.clone();
                if !change.device.matches_search(&self.search) {
                    self.search.clear();
                    self.notice = Some("Поиск очищен для показа изменения USB".into());
                }
                usb_doctor::inventory::reveal(&snapshot, &mut self.inventory, &self.changes, &key);
                self.pending_reveal = Some(key);
            }
        }
        self.generation = self.generation.saturating_add(1);
        let previous = self.selected_device().cloned();
        self.selected = previous
            .as_ref()
            .and_then(|d| {
                snapshot
                    .navigation_rows(self.show_topology)
                    .iter()
                    .position(|(n, _)| n.key == d.key)
            })
            .unwrap_or(usize::MAX);
        if self.selected != usize::MAX {
            self.removed_selection = None;
        } else if let Some(d) = previous
            && self
                .changes
                .entries
                .get(&d.key)
                .is_some_and(|c| c.kind == usb_doctor::inventory::ChangeKind::Removed)
        {
            self.removed_selection = Some(d);
        }
        self.identity_confirmed = false;
        if let Some(old) = &self.snapshot
            && old.inventory_complete
            && snapshot.inventory_complete
        {
            let added = snapshot
                .devices
                .iter()
                .filter(|d| !old.devices.iter().any(|p| p.key == d.key))
                .count();
            let removed = old
                .devices
                .iter()
                .filter(|d| !snapshot.devices.iter().any(|p| p.key == d.key))
                .count();
            if added + removed > 0 {
                self.history.push(format!(
                    "{}  Изменение списка: +{added} / −{removed}",
                    time(snapshot.captured_unix_ms)
                ));
            }
        }
        self.history.push(format!(
            "{}  Получен {}снимок: {} устройств. {}",
            time(snapshot.captured_unix_ms),
            if snapshot.demo { "демо-" } else { "" },
            snapshot.devices.len(),
            if snapshot.inventory_complete {
                "Перечисление завершено."
            } else {
                "Часть сведений недоступна."
            }
        ));
        if self.history.len() > 128 {
            self.history.drain(..self.history.len() - 128);
        }
        self.snapshot = Some(snapshot);
        self.error = None;
    }
    fn request_presence(&mut self, ctx: &egui::Context) {
        if self.presence_rx.is_some() {
            self.probe_again = true;
            return;
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        let epoch = self.epoch;
        self.presence_rx = Some(rx);
        self.last_probe = Instant::now();
        self.probe_again = false;
        std::thread::spawn(move || {
            let result = usb_doctor::collector::presence();
            let _ = tx.send((epoch, result));
            ctx.request_repaint();
        });
    }
    fn refresh(&mut self, ctx: &egui::Context) {
        self.notice = None;
        self.error = None;
        if self.demo {
            self.accept(demo::snapshot());
            return;
        }
        self.manual_refresh = true;
        self.detailed_dirty = true;
        self.request_presence(ctx);
        ctx.request_repaint();
    }
    fn start_details(&mut self) {
        if self.job.is_some() {
            return;
        }
        self.detailed_dirty = false;
        self.job_epoch = self.epoch;
        self.last_refresh = Instant::now();
        match usb_doctor::scan_job::ScanJob::start() {
            Ok(job) => {
                self.job = Some(job);
                self.started = Some(Instant::now());
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn selected_device(&self) -> Option<&Device> {
        self.removed_selection.as_ref().or_else(|| {
            self.snapshot
                .as_ref()?
                .selected_node(self.show_topology, self.selected)
        })
    }
    fn export(&mut self) {
        let Some((snapshot, selection, include_sensitive)) = &self.export_draft else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Отчёт: выбранные поля всех USB-устройств")
            .add_filter("HTML-отчёт", &["html"])
            .add_filter("JSON", &["json"])
            .set_file_name(format!("usb-doctor-{}.html", snapshot.captured_unix_ms))
            .save_file()
        {
            match report::save_selected_new(snapshot, &path, selection, *include_sensitive) {
                Ok(()) => {
                    self.notice = Some(
                        "Отчёт сохранён с выбранными полями и настройкой идентификаторов.".into(),
                    )
                }
                Err(e) => {
                    self.notice = Some(format!(
                        "Отчёт не сохранён: {e}. Существующие файлы не заменяются."
                    ))
                }
            }
        }
    }
    fn export_preview(&mut self, ctx: &egui::Context) {
        let Some((snapshot, selection, include_sensitive)) = &mut self.export_draft else {
            return;
        };
        let mut open = true;
        let mut save = false;
        let mut close = false;
        egui::Window::new("Предпросмотр отчёта")
            .open(&mut open)
            .default_size([640.0, 480.0])
            .show(ctx, |ui| {
                ui.label(format!(
                    "Снимок {} · {} устройств · {} узлов",
                    time(snapshot.captured_unix_ms),
                    snapshot.devices.len(),
                    snapshot.topology.len()
                ));
                ui.label(if snapshot.inventory_complete {
                    "Перечисление завершено"
                } else {
                    "Внимание: неполный снимок"
                });
                ui.checkbox(include_sensitive, "Включать идентификаторы в отчёт");
                muted(
                    ui,
                    "Показаны именно сохраняемые поля. Ошибки и статус полноты включаются всегда.",
                );
                egui::ScrollArea::vertical()
                    .max_height(330.0)
                    .show(ui, |ui| {
                        for (i, d) in snapshot
                            .devices
                            .iter()
                            .chain(&snapshot.topology)
                            .enumerate()
                        {
                            let fields = usb_doctor::fields::for_device(d);
                            let shown: Vec<_> = fields
                                .iter()
                                .filter(|f| {
                                    selection.contains(&f.id)
                                        && (*include_sensitive || !f.sensitive)
                                })
                                .collect();
                            let hidden = fields
                                .iter()
                                .filter(|f| {
                                    selection.contains(&f.id)
                                        && !(*include_sensitive || !f.sensitive)
                                })
                                .count();
                            ui.collapsing(
                                format!(
                                    "Узел {} · {} полей · скрыто {} · ошибок {}",
                                    i + 1,
                                    shown.len(),
                                    hidden,
                                    d.issues.len()
                                ),
                                |ui| {
                                    for f in shown {
                                        ui.label(format!("{}: {}", f.label, f.value));
                                    }
                                    for e in &d.issues {
                                        ui.label(format!(
                                            "{} · код {}",
                                            e.operation,
                                            usb_doctor::fields::optional_number(e.code)
                                        ));
                                    }
                                },
                            );
                        }
                        for e in &snapshot.issues {
                            ui.label(format!(
                                "{} · код {}",
                                e.operation,
                                usb_doctor::fields::optional_number(e.code)
                            ));
                        }
                    });
                ui.horizontal(|ui| {
                    save = ui.button("Сохранить файл…").clicked();
                    close = ui.button("Закрыть предпросмотр").clicked();
                });
            });
        if save {
            self.export();
        }
        if !open || close {
            self.export_draft = None;
        }
    }
    fn overview(&mut self, ui: &mut egui::Ui, d: &Device) {
        if d.port.is_some() {
            let a = assess(d);
            ui.horizontal_wrapped(|ui| {
                ui.colored_label(
                    match a.tone {
                        Tone::Attention => {
                            if ui.visuals().dark_mode {
                                Color32::from_rgb(235, 176, 85)
                            } else {
                                Color32::from_rgb(145, 90, 20)
                            }
                        }
                        Tone::Unknown => {
                            if ui.visuals().dark_mode {
                                Color32::from_gray(180)
                            } else {
                                MUTED
                            }
                        }
                        Tone::Neutral => {
                            if ui.visuals().dark_mode {
                                Color32::from_rgb(76, 195, 170)
                            } else {
                                TEAL
                            }
                        }
                    },
                    RichText::new(a.title).strong(),
                );
                if ui.small_button("Сравнить").clicked() {
                    self.baseline = Some(Baseline {
                        device: d.clone(),
                        demo: self.demo,
                        time: self.snapshot.as_ref().map_or(0, |s| s.captured_unix_ms),
                        generation: self.generation,
                    });
                    self.identity_confirmed = false;
                    self.tab = Tab::Compare;
                }
            });
            ui.collapsing(
                "Основание и следующая проверка",
                |ui| {
                    ui.label(a.fact);
                    ui.label(a.interpretation);
                    ui.label(a.next_step);
                },
            );
        }
        if !d.issues.is_empty()
            && ui
                .small_button(format!("Доп. сведения недоступны: {}", d.issues.len()))
                .clicked()
        {
            self.tab = Tab::Details;
        }
        let fields = usb_doctor::fields::for_device(d);
        let count = fields
            .iter()
            .filter(|f| self.selection.contains(&f.id))
            .count();
        ui.horizontal(|ui| {
            muted(ui, format!("Показано {count} из {} полей", fields.len()));
            if ui.small_button("Выбрать поля…").clicked() {
                self.fields_open = true;
            }
        });
        let mut labels = std::collections::BTreeSet::new();
        let show_groups = self.selection.all
            || count > 30
            || fields
                .iter()
                .filter(|f| self.selection.contains(&f.id))
                .any(|f| !labels.insert(&f.label));
        egui::Grid::new("data-fields")
            .num_columns(2)
            .spacing([12.0, 3.0])
            .striped(true)
            .min_col_width(170.0)
            .show(ui, |ui| {
                let mut previous_group = "";
                for f in fields.iter().filter(|f| self.selection.contains(&f.id)) {
                    if show_groups && f.group != previous_group {
                        ui.strong(&f.group);
                        ui.label("");
                        ui.end_row();
                        previous_group = &f.group;
                    }
                    ui.label(&f.label)
                        .on_hover_text(format!("{}\n{}\n{}", f.group, f.id, f.source));
                    let text = if f.display_value().chars().count() > 140 {
                        format!(
                            "{}…",
                            f.display_value().chars().take(140).collect::<String>()
                        )
                    } else {
                        f.display_value().to_owned()
                    };
                    ui.add(egui::Label::new(text).wrap())
                        .on_hover_text(&f.value)
                        .context_menu(|ui| {
                            if ui.button("Копировать значение целиком").clicked()
                            {
                                ui.ctx().copy_text(f.value.clone());
                                ui.close();
                            }
                        });
                    ui.end_row();
                }
            });
        if count == 0 {
            ui.label("Поля скрыты. Выберите нужные галочками или нажмите «Основное».");
        }
    }
    fn field_picker(&mut self, ctx: &egui::Context) {
        if !self.fields_open {
            return;
        }
        let mut open = true;
        let fields = self
            .selected_device()
            .map(usb_doctor::fields::for_device)
            .unwrap_or_default();
        let before = self.selection.clone();
        egui::Window::new("Показывать поля")
            .open(&mut open)
            .default_size([450.0, 520.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.small_button("Основное").clicked() {
                        self.selection = Default::default();
                    }
                    if ui.small_button("Все").clicked() {
                        self.selection = usb_doctor::fields::FieldSelection::all();
                    }
                    if ui.small_button("Ничего").clicked() {
                        self.selection = usb_doctor::fields::FieldSelection::none();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for preset in ["Подключение", "Питание", "Драйвер", "Дескрипторы"]
                    {
                        if ui.small_button(preset).clicked() {
                            self.selection =
                                usb_doctor::fields::FieldSelection::preset(preset, &fields);
                        }
                    }
                });
                ui.add(
                    egui::TextEdit::singleline(&mut self.field_search)
                        .hint_text("Поиск поля…")
                        .desired_width(f32::INFINITY),
                );
                ui.checkbox(
                    &mut self.include_sensitive,
                    "Включать идентификаторы в сохраняемый отчёт",
                );
                muted(
                    ui,
                    "Галочки меняют только вывод. Все собранные сведения остаются в снимке.",
                );
                ui.separator();
                let query = self.field_search.to_lowercase();
                let mut groups = std::collections::BTreeMap::<String, Vec<_>>::new();
                for field in &fields {
                    if format!("{} {}", field.label, field.id)
                        .to_lowercase()
                        .contains(&query)
                    {
                        groups.entry(field.group.clone()).or_default().push(field);
                    }
                }
                egui::ScrollArea::vertical()
                    .max_height(430.0)
                    .show(ui, |ui| {
                        let mut groups: Vec<_> = groups.into_iter().collect();
                        groups.sort_by_key(|(group, _)| (group != "Основное", group.clone()));
                        for (group, items) in groups {
                            egui::CollapsingHeader::new(format!("{group} ({})", items.len()))
                                .default_open(group == "Основное" || !query.is_empty())
                                .show(ui, |ui| {
                                    let mut all =
                                        items.iter().all(|f| self.selection.contains(&f.id));
                                    if ui.checkbox(&mut all, "Все поля группы").changed()
                                    {
                                        for f in &items {
                                            self.selection.set(&f.id, all, &fields);
                                        }
                                    }
                                    for f in items {
                                        ui.push_id(&f.id, |ui| {
                                            let mut on = self.selection.contains(&f.id);
                                            if ui
                                                .checkbox(&mut on, &f.label)
                                                .on_hover_text(format!(
                                                    "{}\nИсточник: {}\nID: {}",
                                                    f.value, f.source, f.id
                                                ))
                                                .changed()
                                            {
                                                self.selection.set(&f.id, on, &fields);
                                            }
                                        });
                                    }
                                });
                        }
                    });
            });
        self.fields_open = open;
        if self.selection != before
            && let Err(e) = save_selection(&self.selection)
        {
            self.notice = Some(format!("Настройки полей не сохранены: {e}"));
        }
    }
    fn comparison(&mut self, ui: &mut egui::Ui, d: &Device) {
        ui.heading("Сравнение подключений");
        if let Some(base) = &self.baseline {
            muted(
                ui,
                format!("Первый снимок: {} · {}", base.device.name, time(base.time)),
            );
            ui.label("Измените одно условие: подключите то же устройство к другому порту с тем же кабелем. Нажмите «Обновить» и выберите его в списке.");
            if self.demo {
                ui.label(
                    RichText::new("Демо обновляет те же примеры. Реального переподключения нет.")
                        .color(TEAL),
                );
            }
            if self.generation <= base.generation || self.error.is_some() {
                muted(
                    ui,
                    "Нужен новый успешный снимок: нажмите «Обновить» после изменения подключения.",
                );
                return;
            }
            ui.checkbox(
                &mut self.identity_confirmed,
                "Подтверждаю: выбрано то же физическое устройство",
            );
            let source = self.snapshot.as_ref().is_some_and(|s| s.demo == base.demo);
            if let Ok(c) = compare(&base.device, d, self.identity_confirmed, source) {
                egui::Grid::new("comparison")
                    .num_columns(3)
                    .spacing([12.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Параметр");
                        ui.strong("Сохранённый снимок");
                        ui.strong("Текущий снимок");
                        ui.end_row();
                        ui.label("Режим USB");
                        ui.label(base.device.speed_label());
                        ui.label(d.speed_label());
                        ui.end_row();
                        ui.label("Путь");
                        ui.label(base.device.path.join(" / "));
                        ui.label(d.path.join(" / "));
                        ui.end_row();
                        ui.label("Порт");
                        ui.label(usb_doctor::fields::optional_number(base.device.port));
                        ui.label(usb_doctor::fields::optional_number(d.port));
                        ui.end_row();
                        ui.label("Код Windows");
                        ui.label(usb_doctor::fields::optional_number(
                            base.device.windows_problem,
                        ));
                        ui.label(usb_doctor::fields::optional_number(d.windows_problem));
                        ui.end_row();
                    });
                ui.add_space(6.0);
                ui.label(if c.changed{"Режимы в снимках различаются. Это не доказывает неисправность порта или кабеля."}else{"Различий режима не обнаружено. Это не доказывает стабильность или исправность."});
            } else {
                muted(
                    ui,
                    "Сравнение доступно после подтверждения того же устройства. VID/PID и название не гарантируют идентичность.",
                );
            }
            if ui.button("Убрать сохранённый снимок").clicked() {
                self.baseline = None;
                self.identity_confirmed = false;
            }
        } else {
            ui.label("Сначала выберите устройство и сохраните его снимок в разделе «Обзор».");
        }
    }
    fn column_picker(&mut self, ctx: &egui::Context) {
        if !self.column_picker_open {
            return;
        }
        if self.catalog_dirty
            && let Some(snapshot) = &self.snapshot
        {
            self.field_catalog = usb_doctor::inventory::field_catalog(snapshot);
            self.catalog_dirty = false;
        }
        let mut open = true;
        let mut done = false;
        egui::Window::new("Поля таблицы").open(&mut open).default_size([620.0,520.0]).show(ctx,|ui|{
            ui.label("Любое собранное поле можно вывести отдельной колонкой. — означает, что у этого узла поле не получено.");
            ui.horizontal(|ui|{ui.add(egui::TextEdit::singleline(&mut self.column_search).hint_text("Название, группа или ID поля…").desired_width(380.0));if ui.small_button("Основные").clicked(){self.inventory.columns=usb_doctor::inventory::Column::ALL.into_iter().collect();self.inventory.extra_columns.clear();self.inventory_dirty=true;}});
            ui.label(format!("Дополнительных колонок: {} · доступных полей: {}",self.inventory.extra_columns.len(),self.field_catalog.len()));
            let query=self.column_search.to_lowercase();
            egui::ScrollArea::vertical().max_height(380.0).show(ui,|ui|{
                let mut groups=std::collections::BTreeMap::<String,Vec<_>>::new();
                for f in &self.field_catalog {if format!("{} {} {}",f.label,f.group,f.id).to_lowercase().contains(&query){groups.entry(f.group.clone()).or_default().push(f);}}
                for (group,fields) in groups {egui::CollapsingHeader::new(format!("{group} ({})",fields.len())).id_salt(("column-group",&group,query.is_empty())).default_open(!query.is_empty()||group=="Основное").show(ui,|ui|{
                    let mut all=fields.iter().all(|f|self.inventory.field_selected(&f.id));
                    if ui.checkbox(&mut all,"Все поля группы").changed(){for f in &fields {self.inventory.set_field(&f.id,&format!("{} · {}",f.label,f.group),all);}self.inventory_dirty=true;}
                    for f in fields {ui.push_id(&f.id,|ui|{let mut on=self.inventory.field_selected(&f.id);if ui.checkbox(&mut on,&f.label).on_hover_text(format!("{}\n{}",f.id,f.source)).changed(){self.inventory.set_field(&f.id,&format!("{} · {}",f.label,f.group),on);self.inventory_dirty=true;}});}
                });}
            });done=ui.button("Готово").clicked();
        });
        self.inventory.normalize();
        self.column_picker_open = open && !done;
    }
    fn inventory_ui(&mut self, ui: &mut egui::Ui) {
        use usb_doctor::inventory::{self, ChangeKind, Column};
        let before = serde_json::to_string(&self.inventory).unwrap_or_default();
        let mut reset_widths = false;
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text("Имя, VID/PID, серийный номер, порт…")
                    .desired_width(290.0),
            );
            if !self.search.is_empty() && ui.small_button("Очистить поиск").clicked() {
                self.search.clear();
            }
            if ui
                .checkbox(&mut self.inventory.grouped, "По хабам")
                .changed()
            {
                self.show_topology = self.inventory.grouped;
                self.selected = usize::MAX;
                self.removed_selection = None;
            }
            ui.checkbox(
                &mut self.inventory.show_hubs,
                if self.inventory.grouped {
                    "Пустые ветки"
                } else {
                    "Показывать хабы"
                },
            );
            if ui.button("Поля / колонки…").clicked() {
                self.column_picker_open = true;
            }
            ui.menu_button("Вид / колонки", |ui| {
                if ui.button("Сбросить порядок").clicked() {
                    self.inventory.column_order.clear();
                    self.inventory_dirty = true;
                }
                if ui.button("Сбросить ширины").clicked() {
                    self.inventory.widths.clear();
                    reset_widths = true;
                }
                ui.checkbox(
                    &mut self.inventory.follow_changes,
                    "Прокручивать к изменениям",
                );
                ui.separator();
                for col in Column::ALL {
                    let mut on = self.inventory.columns.contains(&col);
                    if ui.checkbox(&mut on, col.label()).changed() {
                        if on {
                            self.inventory.columns.insert(col);
                        } else {
                            self.inventory.columns.remove(&col);
                        }
                    }
                }
            });
            if self.inventory.grouped {
                if ui.small_button("Развернуть всё").clicked() {
                    self.inventory.collapsed.clear();
                }
                if ui.small_button("Свернуть всё").clicked()
                    && let Some(s) = &self.snapshot
                {
                    for (d, _) in s.navigation_rows(true) {
                        if d.is_hub || d.kind() == "USB-контроллер" {
                            self.inventory.collapsed.insert(d.key.clone());
                        }
                    }
                }
            }
        });
        let Some(snapshot) = &self.snapshot else {
            muted(ui, "Получаем список подключённых USB-устройств…");
            return;
        };
        let rows = inventory::rows(snapshot, &self.inventory, &self.search, &self.changes);
        let devices = snapshot.devices.iter().filter(|d| !d.is_hub).count();
        muted(
            ui,
            format!(
                "Подключено устройств: {devices} · USB-хабов: {} · нажмите строку для подробностей",
                snapshot.devices.iter().filter(|d| d.is_hub).count()
            ),
        );
        if rows.is_empty() {
            ui.label("Нет совпадений. Очистите поиск; история доступна сверху.");
        }
        let mut clicked: Option<(Device, bool)> = None;
        let columns = inventory::display_columns(&self.inventory);
        let scroll_target = self
            .pending_reveal
            .as_ref()
            .and_then(|key| rows.iter().position(|row| row.device.key == *key));
        let viewport_width = ui.available_width();
        let column_keys: Vec<String> = columns.iter().map(|c| c.key().to_owned()).collect();
        let layout_changed = column_keys != self.table_keys;
        let saved_width = columns
            .iter()
            .map(|c| {
                self.inventory
                    .widths
                    .get(c.key())
                    .copied()
                    .unwrap_or(c.width())
            })
            .sum::<f32>()
            + 7.0 * columns.len() as f32;
        let mut reorder: Option<(String, String)> = None;
        let dark = ui.visuals().dark_mode;
        let header_width =
            (viewport_width - ui.spacing().scroll.bar_width - ui.spacing().scroll.bar_inner_margin)
                .max(200.0);
        // The header and body share horizontal offset. Only the BODY owns scroll bars,
        // so the vertical bar remains on the visible window edge even for wide content.
        ui.allocate_ui_with_layout(egui::vec2(header_width,27.0),egui::Layout::top_down(egui::Align::Min),|ui|{
            egui::ScrollArea::horizontal().id_salt("table-header-scroll").horizontal_scroll_offset(self.table_scroll_x).scroll_source(egui::scroll_area::ScrollSource::NONE).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).auto_shrink([false,true]).show(ui,|ui|{
                ui.set_width(header_width.max(saved_width));
                let mut table=egui_extras::TableBuilder::new(ui).id_salt(("table-header",&column_keys)).vscroll(false).min_scrolled_height(0.0).max_scroll_height(0.0).auto_shrink([false,true]).resizable(true).cell_layout(egui::Layout::left_to_right(egui::Align::Center));
                for col in &columns {let fallback=if col.id=="name"{(header_width-columns.iter().filter(|c|c.id!="name").map(|c|c.width()).sum::<f32>()-7.0*columns.len() as f32).max(320.0)}else{col.width()};let width=self.inventory.widths.get(col.key()).copied().unwrap_or(fallback).clamp(col.minimum(),1600.0);table=table.column(egui_extras::Column::initial(width).at_least(col.minimum()).clip(true));}
                table=table.column(egui_extras::Column::remainder().at_least(0.0).resizable(false));if reset_widths{table.reset();}
                table.header(26.0,|mut header|{for col in &columns{header.col(|ui|{
                    let r=ui.add(egui::Label::new(RichText::new(col.label()).strong()).truncate().show_tooltip_when_elided(false).sense(egui::Sense::drag())).on_hover_text(format!("{}\nПеретащите заголовок для перестановки; границу — для изменения ширины",col.label()));
                    r.dnd_set_drag_payload(ColumnDrag(col.id.clone()));if r.dnd_hover_payload::<ColumnDrag>().is_some(){ui.painter().vline(r.rect.left(),r.rect.y_range(),egui::Stroke::new(2.0,TEAL));}
                    if let Some(payload)=r.dnd_release_payload::<ColumnDrag>(){reorder=Some((payload.0.clone(),col.id.clone()));}
                });}header.col(|_|{});}).body(|body|{for (col,width)in columns.iter().zip(body.widths()){self.inventory.widths.insert(col.id.clone(),*width);}});
            });
        });
        let content_width = columns
            .iter()
            .map(|c| {
                self.inventory
                    .widths
                    .get(c.key())
                    .copied()
                    .unwrap_or(c.width())
            })
            .sum::<f32>()
            + 7.0 * columns.len() as f32;
        let mut scroll = egui::ScrollArea::both()
            .id_salt("table-body-scroll")
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible);
        if layout_changed {
            scroll = scroll
                .vertical_scroll_offset(self.table_scroll)
                .horizontal_scroll_offset(self.table_scroll_x);
        }
        if let Some(index) = scroll_target {
            let pitch = 23.0 + ui.spacing().item_spacing.y;
            scroll = scroll.vertical_scroll_offset(
                (index as f32 * pitch - ui.available_height() * 0.5).max(0.0),
            );
        }
        let output = scroll.show(ui, |ui| {
            ui.set_width(header_width.max(content_width));
            let painter = ui.painter().clone();
            let mut table = egui_extras::TableBuilder::new(ui)
                .id_salt(("table-body", &column_keys))
                .vscroll(false)
                .min_scrolled_height(0.0)
                .auto_shrink([false, true])
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .sense(egui::Sense::click());
            for col in &columns {
                table = table.column(egui_extras::Column::exact(
                    self.inventory
                        .widths
                        .get(col.key())
                        .copied()
                        .unwrap_or(col.width()),
                ));
            }
            table = table.column(egui_extras::Column::remainder().at_least(0.0));
            table.body(|body| {
                body.rows(23.0, rows.len(), |mut table_row| {
                    let item = &rows[table_row.index()];
                    let d = item.device;
                    let kind = self.changes.entries.get(&d.key).map(|c| c.kind);
                    let tint = match (kind, dark) {
                        (Some(ChangeKind::Added), false) => Color32::from_rgb(218, 243, 225),
                        (Some(ChangeKind::Removed), false) => Color32::from_rgb(252, 225, 224),
                        (Some(ChangeKind::Added), true) => Color32::from_rgb(28, 66, 46),
                        (Some(ChangeKind::Removed), true) => Color32::from_rgb(82, 36, 42),
                        _ => Color32::TRANSPARENT,
                    };
                    for col in &columns {
                        table_row.col(|ui| {
                            if tint != Color32::TRANSPARENT {
                                ui.painter().rect_filled(ui.max_rect(), 0.0, tint);
                            }
                            if col.id == "name" {
                                ui.add_space((item.depth.min(8) * 12) as f32);
                                if item.group {
                                    let closed = self.inventory.collapsed.contains(&d.key);
                                    if ui
                                        .small_button(if closed { "+" } else { "-" })
                                        .on_hover_text(if closed {
                                            "Развернуть хаб"
                                        } else {
                                            "Свернуть хаб"
                                        })
                                        .clicked()
                                    {
                                        if closed {
                                            self.inventory.collapsed.remove(&d.key);
                                        } else {
                                            self.inventory.collapsed.insert(d.key.clone());
                                        }
                                    }
                                }
                                let marker = match kind {
                                    Some(ChangeKind::Added) => "+ ",
                                    Some(ChangeKind::Removed) => "− ",
                                    None => "",
                                };
                                let label = if item.group {
                                    format!(
                                        "{marker}{} · {} устр.",
                                        inventory::display_name(d),
                                        item.children
                                    )
                                } else {
                                    format!("{marker}{}", inventory::display_name(d))
                                };
                                ui.add(
                                    egui::Label::new(if item.group {
                                        RichText::new(&label).strong()
                                    } else {
                                        RichText::new(&label)
                                    })
                                    .truncate()
                                    .show_tooltip_when_elided(false),
                                )
                                .on_hover_text(&label);
                            } else {
                                let value = if col.standard == Some(Column::Status) {
                                    match kind {
                                        Some(ChangeKind::Removed) => {
                                            "Отключено · последние сведения".into()
                                        }
                                        Some(ChangeKind::Added) => "Подключено сейчас".into(),
                                        None if item.added + item.removed > 0 => {
                                            format!("Внутри: +{} −{}", item.added, item.removed)
                                        }
                                        None => col.value(d),
                                    }
                                } else {
                                    col.value(d)
                                };
                                ui.add(
                                    egui::Label::new(&value)
                                        .truncate()
                                        .show_tooltip_when_elided(false),
                                )
                                .on_hover_text(&value)
                                .context_menu(|ui| {
                                    if ui.button("Копировать значение").clicked()
                                    {
                                        ui.ctx().copy_text(value);
                                        ui.close();
                                    }
                                });
                            }
                        });
                    }
                    table_row.col(|ui| {
                        if tint != Color32::TRANSPARENT {
                            ui.painter().rect_filled(ui.max_rect(), 0.0, tint);
                        }
                    });
                    let response = table_row.response();
                    let rect = response.rect;
                    let y = rect.bottom();
                    let mut x = rect.left();
                    let color = if dark {
                        Color32::from_rgba_unmultiplied(210, 220, 230, 65)
                    } else {
                        Color32::from_rgba_unmultiplied(65, 80, 95, 65)
                    };
                    while x < rect.right() {
                        painter.line_segment(
                            [egui::pos2(x, y), egui::pos2((x + 4.0).min(rect.right()), y)],
                            egui::Stroke::new(0.75, color),
                        );
                        x += 7.0;
                    }
                    if response.clicked() {
                        clicked = Some((d.clone(), kind == Some(ChangeKind::Removed)));
                    }
                    response.context_menu(|ui| {
                        for (label, value) in [
                            ("Копировать VID:PID", inventory::vid_pid(d)),
                            ("Копировать Instance ID", d.key.clone()),
                        ] {
                            if ui.button(label).clicked() {
                                ui.ctx().copy_text(value);
                                ui.close();
                            }
                        }
                    });
                });
            });
        });
        if (self.table_scroll_x - output.state.offset.x).abs() > 0.01 {
            ui.ctx().request_repaint();
        }
        self.table_scroll = output.state.offset.y;
        self.table_scroll_x = output.state.offset.x;
        self.table_keys = column_keys;
        if let Some((source, target)) = reorder {
            self.inventory.reorder(&source, &target);
            self.inventory_dirty = true;
        }
        if scroll_target.is_some() {
            self.pending_reveal = None;
        }
        if let Some((d, removed)) = clicked {
            self.removed_selection = removed.then_some(d.clone());
            self.selected = snapshot
                .navigation_rows(self.show_topology)
                .iter()
                .position(|(n, _)| n.key == d.key)
                .unwrap_or(usize::MAX);
            self.identity_confirmed = false;
            self.tab = Tab::Overview;
        }
        if before != serde_json::to_string(&self.inventory).unwrap_or_default() {
            self.inventory_dirty = true;
        }
        if self.inventory_dirty && !ui.input(|i| i.pointer.any_down()) {
            match save_inventory(&self.inventory) {
                Ok(()) => self.inventory_dirty = false,
                Err(e) => self.notice = Some(format!("Настройки списка не сохранены: {e}")),
            }
        }
    }
    fn details(&self, ui: &mut egui::Ui, d: &Device) {
        ui.label(format!(
            "{} блоков дескрипторов · исходные байты сохранены",
            d.descriptors.len()
        ));
        for (index, desc) in d.descriptors.iter().enumerate() {
            egui::CollapsingHeader::new(format!(
                "{} #{} · lang {:04X} · {} байт{}",
                desc.kind,
                desc.index,
                desc.language,
                desc.raw.len(),
                if desc.complete {
                    ""
                } else {
                    " · неполный"
                }
            ))
            .id_salt(index)
            .show(ui, |ui| {
                for note in &desc.notes {
                    ui.label(note);
                }
                let raw = usb_doctor::fields::hex(&desc.raw);
                if ui.small_button("Копировать HEX").clicked() {
                    ui.ctx().copy_text(raw.clone());
                }
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        ui.monospace(raw);
                    });
            });
        }
        if !d.issues.is_empty() {
            section(ui, "Не получено / ошибки запросов");
            for issue in &d.issues {
                ui.label(format!(
                    "{} · {:?}: {}",
                    issue.operation, issue.code, issue.reason
                ));
            }
        }
        ui.collapsing("Ограничения интерпретации",|ui|{ui.label("Режим USB не равен скорости копирования. MaxPower — заявленное потребление. Ошибка запроса не доказывает неисправность. Неразобранные class-specific поля сохранены исходными байтами.");});
    }
}
impl eframe::App for Doctor {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.changes.expire(usb_doctor::model::now_ms(), 5);
        if self
            .pending_reveal
            .as_ref()
            .is_some_and(|k| !self.changes.entries.contains_key(k))
        {
            self.pending_reveal = None;
        }
        if self.monitor.as_ref().is_some_and(|m| m.take_changed()) {
            self.epoch = self.epoch.wrapping_add(1);
            self.full_not_before = Instant::now() + Duration::from_millis(150);
            if self.auto_refresh {
                if let Some(job) = &mut self.job {
                    let _ = job.cancel();
                }
                self.detailed_dirty = true;
            }
            self.probe_again = true;
            if self.auto_refresh && !self.demo {
                self.request_presence(ctx);
            }
        }
        let message = self.presence_rx.as_ref().map(|rx| rx.try_recv());
        match message {
            Some(Ok((epoch, result))) => {
                self.presence_rx = None;
                if epoch == self.epoch {
                    match result {
                        Ok(fast) => {
                            let changed = self
                                .snapshot
                                .as_ref()
                                .is_none_or(|old| !usb_doctor::live::same_presence(old, &fast));
                            if changed || !fast.inventory_complete {
                                self.epoch = self.epoch.wrapping_add(1);
                                self.detailed_dirty = true;
                                let merged =
                                    usb_doctor::live::merge_presence(self.snapshot.as_ref(), fast);
                                self.accept(merged);
                            }
                        }
                        Err(e) => self.error = Some(e),
                    }
                } else {
                    self.probe_again = true;
                }
            }
            Some(Err(mpsc::TryRecvError::Disconnected)) => {
                self.presence_rx = None;
                self.error = Some("Быстрый опрос завершился без результата".into());
            }
            _ => {}
        }
        if let Some(result) = self.job.as_mut().and_then(|j| j.poll()) {
            self.job = None;
            self.started = None;
            if self.job_epoch == self.epoch {
                self.manual_refresh = false;
                match result {
                    Ok(full) => {
                        let snapshot = self
                            .snapshot
                            .as_ref()
                            .map(|live| usb_doctor::live::apply_details(live, full.clone()))
                            .unwrap_or(full);
                        self.accept(snapshot);
                    }
                    Err(e) => self.error = Some(e),
                }
            } else {
                self.detailed_dirty = self.auto_refresh || self.manual_refresh;
            }
        }
        if self.auto_refresh && !self.demo {
            if self.presence_rx.is_none()
                && (self.probe_again || self.last_probe.elapsed() >= Duration::from_secs(2))
            {
                self.request_presence(ctx);
            }
            if self.last_refresh.elapsed() >= Duration::from_secs(60) {
                self.detailed_dirty = true;
            }
        }
        if !self.demo
            && self.detailed_dirty
            && (self.auto_refresh || self.manual_refresh)
            && self.job.is_none()
            && self.presence_rx.is_none()
            && Instant::now() >= self.full_not_before
        {
            self.start_details();
        }
        if self.column_picker_open
            && self.catalog_dirty
            && let Some(snapshot) = &self.snapshot
        {
            self.field_catalog = usb_doctor::inventory::field_catalog(snapshot);
            self.catalog_dirty = false;
        }
        ctx.request_repaint_after(Duration::from_millis(
            if self.job.is_some()
                || self.presence_rx.is_some()
                || (self.detailed_dirty && (self.auto_refresh || self.manual_refresh))
            {
                20
            } else {
                1000
            },
        ));
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        egui::Panel::top("header")
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(8),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("USB Глаз").strong().size(18.0));
                    ui.selectable_value(&mut self.tab, Tab::Inventory, "Подключения");
                    ui.selectable_value(&mut self.tab, Tab::History, "История");
                    if ui
                        .add_enabled(self.job.is_none(), egui::Button::new("Обновить"))
                        .clicked()
                    {
                        self.refresh(ui.ctx());
                    }
                    if self.job.is_some() && ui.button("Остановить сбор").clicked() {
                        self.auto_refresh = false;
                        self.detailed_dirty = false;
                        self.manual_refresh = false;
                        self.probe_again = false;
                        if let Some(job) = &mut self.job
                            && let Err(e) = job.cancel()
                        {
                            self.error = Some(e);
                        }
                    }
                    if ui
                        .add_enabled(
                            self.snapshot.is_some(),
                            egui::Button::new("Сохранить отчёт"),
                        )
                        .clicked()
                    {
                        self.export_draft = self.snapshot.clone().map(|s| {
                            (
                                s,
                                if self.tab == Tab::Inventory {
                                    usb_doctor::inventory::export_selection(&self.inventory)
                                } else {
                                    self.selection.clone()
                                },
                                self.include_sensitive,
                            )
                        });
                    }
                    ui.add_enabled(
                        !self.demo,
                        egui::Checkbox::new(&mut self.auto_refresh, "Автообновление"),
                    )
                    .on_hover_text(&self.monitor_status);
                    ui.menu_button("Тема", |ui| {
                        let before = self.inventory.theme;
                        for (choice, label) in [
                            (usb_doctor::inventory::ThemeChoice::System, "Как в Windows"),
                            (usb_doctor::inventory::ThemeChoice::Light, "Светлая"),
                            (usb_doctor::inventory::ThemeChoice::Dark, "Тёмная"),
                        ] {
                            ui.radio_value(&mut self.inventory.theme, choice, label);
                        }
                        if before != self.inventory.theme {
                            crate::appearance::apply(ui.ctx(), self.inventory.theme);
                            if let Err(e) = save_inventory(&self.inventory) {
                                self.notice = Some(format!("Тема не сохранена: {e}"));
                            }
                            self.inventory_dirty = true;
                        }
                    });
                });
            });
        egui::CentralPanel::default().frame(egui::Frame::new().fill(ui.visuals().panel_fill).inner_margin(8)).show(ui,|ui|{
            if !self.auto_refresh&&!self.demo{ui.colored_label(Color32::from_rgb(150,94,20),"Автообновление приостановлено. Показаны последние полученные сведения.");}
            if let Some(e)=&self.error{ui.colored_label(Color32::from_rgb(160,55,40),format!("{e} Данные могут быть устаревшими."));}
            if self.snapshot.as_ref().is_some_and(|s|!s.inventory_complete||!s.issues.is_empty()){ui.colored_label(Color32::from_rgb(150,94,20),"Часть USB-сведений недоступна; отсутствие устройства не подтверждает отключение.");}
            if let Some(notice)=&self.notice{ui.label(notice);}
            if self.tab!=Tab::Inventory&&self.pending_reveal.is_some()&&ui.button("Изменилось подключение USB — показать").clicked(){self.tab=Tab::Inventory;}
            match self.tab {
                Tab::Inventory=>self.inventory_ui(ui),
                Tab::History=>{ui.heading("История сеанса");if self.history.is_empty(){muted(ui,"Событий пока нет.");}egui::ScrollArea::vertical().show(ui,|ui|{for e in self.history.iter().rev(){ui.label(e);}});},
                _=>{
                    if ui.small_button("← К подключениям").clicked(){self.tab=Tab::Inventory;}
                    if let Some(d)=self.selected_device().cloned(){
                        ui.horizontal_wrapped(|ui|{ui.label(RichText::new(usb_doctor::inventory::display_name(&d)).strong().size(17.0));muted(ui,usb_doctor::inventory::functions(&d).join(" + "));});
                        if self.removed_selection.is_some(){ui.colored_label(Color32::from_rgb(166,54,54),"Отключено · последние известные сведения");}
                        ui.horizontal_wrapped(|ui|{
                            if ui.small_button("Копировать VID:PID").clicked(){ui.ctx().copy_text(usb_doctor::inventory::vid_pid(&d));}
                            if ui.small_button("Копировать Instance ID").clicked(){ui.ctx().copy_text(d.key.clone());}
                            if let Some(serial)=d.fields.iter().find(|f|f.id=="usb.serial")&&ui.small_button("Копировать серийный номер").clicked(){ui.ctx().copy_text(serial.value.clone());}
                            if let Some(address)=d.fields.iter().find(|f|f.id=="usb.address"){ui.label(format!("Адрес USB: {}",address.value)).on_hover_text("Адрес на шине USB; может измениться после переподключения. Это не VID/PID и не Instance ID.");}
                        });
                        ui.horizontal(|ui|{for (tab,label) in [(Tab::Overview,"Поля"),(Tab::Compare,"Сравнение"),(Tab::Details,"Дескрипторы")]{ui.selectable_value(&mut self.tab,tab,label);}});ui.separator();
                        egui::ScrollArea::vertical().show(ui,|ui|{match self.tab {Tab::Overview=>self.overview(ui,&d),Tab::Compare=>self.comparison(ui,&d),Tab::Details=>self.details(ui,&d),_=>{}}});
                    }else{muted(ui,"Выберите устройство в списке подключений.");}
                }
            }
        });
        self.field_picker(ui.ctx());
        self.export_preview(ui.ctx());
        self.column_picker(ui.ctx());
    }
}

fn settings_path() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA").map(|p| {
            std::path::PathBuf::from(p)
                .join("UsbDoctor")
                .join("fields.json")
        })
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|p| std::path::PathBuf::from(p).join(".config"))
            })
            .map(|p| p.join("usb-doctor/fields.json"))
    }
}
fn load_selection() -> usb_doctor::fields::FieldSelection {
    let mut selection: usb_doctor::fields::FieldSelection = settings_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    selection.remove_legacy_ids();
    selection
}
fn save_selection(s: &usb_doctor::fields::FieldSelection) -> std::io::Result<()> {
    if let Some(p) = settings_path() {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, serde_json::to_vec_pretty(s)?)?;
    }
    Ok(())
}

fn inventory_path() -> Option<std::path::PathBuf> {
    settings_path().map(|p| p.with_file_name("inventory.json"))
}
fn load_inventory() -> usb_doctor::inventory::Options {
    let mut o: usb_doctor::inventory::Options = inventory_path()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    o.normalize();
    o
}
fn save_inventory(o: &usb_doctor::inventory::Options) -> std::io::Result<()> {
    if let Some(p) = inventory_path() {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, serde_json::to_vec_pretty(o)?)?;
    }
    Ok(())
}

impl Drop for Doctor {
    fn drop(&mut self) {
        let _ = save_inventory(&self.inventory);
    }
}
