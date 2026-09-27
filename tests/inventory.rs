use usb_doctor::{
    demo,
    inventory::{self, ChangeKind, Changes, Column, Options},
    model::Snapshot,
};
#[test]
fn grouping_collapses_only_chosen_branch_and_search_reveals_matches() {
    let s = demo::snapshot();
    let changes = Changes::default();
    let mut o = Options::default();
    let full = inventory::rows(&s, &o, "", &changes);
    assert_eq!(full.len(), 6);
    o.collapsed.insert("demo:hub".into());
    assert_eq!(inventory::rows(&s, &o, "", &changes).len(), 2);
    let found = inventory::rows(&s, &o, "DEMO-0000", &changes);
    assert!(found.iter().any(|r| r.device.key == "demo:ssd"));
    assert!(o.collapsed.contains("demo:hub"));
    o.grouped = false;
    assert_eq!(inventory::rows(&s, &o, "", &changes).len(), 4);
}
#[test]
fn removed_is_red_until_expiry_and_reconnect_replaces_it() {
    let old = demo::snapshot();
    let mut next = old.clone();
    next.devices.remove(1);
    let mut c = Changes::default();
    c.update(&old, &next, 100);
    assert_eq!(c.entries["demo:keyboard"].kind, ChangeKind::Removed);
    let rows = inventory::rows(&next, &Options::default(), "", &c);
    assert!(rows.iter().any(|r| r.device.key == "demo:keyboard"));
    let hub = rows.iter().find(|r| r.device.key == "demo:hub").unwrap();
    assert_eq!(hub.children, 3);
    assert_eq!(hub.removed, 1);
    c.update(&next, &old, 200);
    assert_eq!(c.entries["demo:keyboard"].kind, ChangeKind::Added);
    c.expire(10199, 10);
    assert!(!c.entries.is_empty());
    c.expire(10200, 10);
    assert!(c.entries.is_empty());
}
#[test]
fn incomplete_scan_and_mode_switch_do_not_invent_disconnects() {
    let old = demo::snapshot();
    let mut incomplete = old.clone();
    incomplete.devices.clear();
    incomplete.inventory_complete = false;
    let mut c = Changes::default();
    c.update(&old, &incomplete, 1);
    assert!(c.entries.is_empty());
    c.update(&incomplete, &Snapshot::empty(true), 2);
    assert!(c.entries.is_empty());
    let mut other = old.clone();
    other.demo = false;
    other.devices.clear();
    c.update(&old, &other, 3);
    assert!(c.entries.is_empty());
}
#[test]
fn address_is_independent_of_vid_pid_and_port() {
    let s = demo::snapshot();
    let d = &s.devices[0];
    assert_eq!(Column::Address.value(d), "2");
    assert_eq!(Column::Port.value(d), "1");
    assert_eq!(Column::Ids.value(d), "1234:5678");
}
#[test]
fn hid_usage_page_must_belong_to_same_interface() {
    use usb_doctor::fields::Field;
    let mut d = demo::snapshot().devices.remove(1);
    d.fields
        .push(Field::new("a.hid.usage", "", "", "6", "", false));
    d.fields
        .push(Field::new("b.hid.usage_page", "", "", "1", "", false));
    assert!(!inventory::functions(&d).contains(&"Клавиатура".into()));
    d.fields
        .push(Field::new("a.hid.usage_page", "", "", "1", "", false));
    assert!(inventory::functions(&d).contains(&"Клавиатура".into()));
}
#[test]
fn preferences_keep_columns_and_collapsed_hubs() {
    let mut o = Options::default();
    o.columns.remove(&Column::Ids);
    o.collapsed.insert("hub".into());
    let read: Options = serde_json::from_slice(&serde_json::to_vec(&o).unwrap()).unwrap();
    assert!(!read.columns.contains(&Column::Ids));
    assert!(read.collapsed.contains("hub"));
}
#[cfg(windows)]
#[test]
fn single_instance_guard_rejects_second_then_releases() {
    let first = usb_doctor::single_instance::acquire()
        .unwrap()
        .expect("first instance");
    assert!(usb_doctor::single_instance::acquire().unwrap().is_none());
    drop(first);
    assert!(usb_doctor::single_instance::acquire().unwrap().is_some());
}
#[test]
fn reveal_opens_target_ancestors_without_resetting_other_groups() {
    let s = demo::snapshot();
    let c = Changes::default();
    let mut o = Options::default();
    o.collapsed.extend([
        "demo:controller".into(),
        "demo:hub".into(),
        "other-hub".into(),
    ]);
    inventory::reveal(&s, &mut o, &c, "demo:keyboard");
    assert!(!o.collapsed.contains("demo:hub"));
    assert!(!o.collapsed.contains("demo:controller"));
    assert!(o.collapsed.contains("other-hub"));
}
#[test]
fn any_collected_field_can_be_added_without_losing_values() {
    use usb_doctor::fields::Field;
    let mut s = demo::snapshot();
    s.devices[0].fields.push(Field::new(
        "custom.empty",
        "Empty",
        "Windows",
        "",
        "SetupAPI",
        false,
    ));
    let before = serde_json::to_value(&s).unwrap();
    let catalog = inventory::field_catalog(&s);
    assert!(catalog.iter().any(|f| f.id == "windows.DriverVersion"));
    assert!(catalog.iter().any(|f| f.id == "usb.address"));
    assert_eq!(serde_json::to_value(&s).unwrap(), before);
    let mut o = Options::default();
    o.extra_columns
        .insert("windows.DriverVersion".into(), "Driver version".into());
    o.extra_columns
        .insert("custom.empty".into(), "Empty".into());
    let cols = inventory::columns(&o);
    let driver = cols
        .iter()
        .find(|c| c.key() == "field:windows.DriverVersion")
        .unwrap();
    assert_eq!(driver.value(&s.devices[0]), "10.0.26100.1");
    assert_eq!(driver.value(&s.devices[3]), "—");
    assert_eq!(
        cols.iter()
            .find(|c| c.key() == "field:custom.empty")
            .unwrap()
            .value(&s.devices[0]),
        "Пустое значение"
    );
    let loaded: Options = serde_json::from_slice(&serde_json::to_vec(&o).unwrap()).unwrap();
    assert_eq!(loaded.extra_columns, o.extra_columns);
}
#[test]
fn product_name_requires_real_string_not_missing_value_placeholder() {
    use usb_doctor::fields::Field;
    let mut d = demo::snapshot().devices.remove(0);
    d.fields.push(Field::new(
        "usb.product",
        "",
        "",
        "Device product",
        "String descriptor",
        false,
    ));
    assert_eq!(inventory::display_name(&d), "Device product");
    d.name_source = "Windows FriendlyName".into();
    assert_eq!(inventory::display_name(&d), d.name);
}
#[test]
fn table_export_keeps_extra_columns_and_unconditional_errors() {
    let s = demo::snapshot();
    let mut o = Options::default();
    o.columns.remove(&Column::Port);
    o.extra_columns
        .insert("windows.DriverVersion".into(), "Version".into());
    let mask = inventory::export_selection(&o);
    assert!(mask.contains("windows.DriverVersion"));
    assert!(mask.contains("functions"));
    assert!(!mask.contains("port"));
    let exported = usb_doctor::report::selected(&s, &mask, true);
    assert!(
        exported["devices"][0]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "windows.DriverVersion")
    );
}
