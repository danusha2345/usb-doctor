use usb_doctor::{
    demo,
    fields::{Field, FieldSelection, for_device, stable_id},
    model::{Issue, Snapshot},
    report,
};
#[test]
fn all_except_one_survives_new_devices_and_roundtrip() {
    let a = demo::snapshot().devices.remove(0);
    let mut mask = FieldSelection::all();
    mask.set("pid", false, &for_device(&a));
    assert!(mask.all);
    assert!(!mask.contains("pid"));
    assert!(mask.contains("future.device.field"));
    let mut restored: FieldSelection =
        serde_json::from_str(&serde_json::to_string(&mask).unwrap()).unwrap();
    assert!(restored.contains("future.device.field"));
    restored.set("pid", true, &[]);
    assert!(restored.contains("pid"));
}
#[test]
fn legacy_settings_keep_basics_but_drop_unsafe_position_ids() {
    let mut s:FieldSelection=serde_json::from_str(r#"{"all":false,"included":["pid","function.4.name","interface.3.hid.usage","disk.1.number","function.abcdef.name"]}"#).unwrap();
    s.remove_legacy_ids();
    assert!(s.contains("pid"));
    assert!(s.contains("function.abcdef.name"));
    assert!(!s.contains("function.4.name"));
    assert!(!s.contains("interface.3.hid.usage"));
    assert!(!s.contains("disk.1.number"));
}
#[test]
fn function_identity_does_not_depend_on_order_or_case() {
    let a = "USB\\VID_1234\\PRIVATE-SERIAL-A";
    let b = "USB\\VID_1234\\PRIVATE-SERIAL-B";
    assert_eq!(stable_id(a), stable_id(&a.to_lowercase()));
    assert_ne!(stable_id(a), stable_id(b));
    assert!(!stable_id(a).contains("PRIVATE"));
    let mut mask = FieldSelection::none();
    let id = format!("function.{}.name", stable_id(a));
    mask.set(&id, true, &[]);
    for node in [b, a] {
        assert_eq!(
            mask.contains(&format!("function.{}.name", stable_id(node))),
            node == a
        );
    }
}
#[test]
fn errors_are_unconditional_but_reasons_respect_privacy() {
    let mut s = demo::snapshot();
    s.inventory_complete = false;
    s.issues.push(Issue::new("Scan", Some(5), "PRIVATE-PATH"));
    s.devices[0]
        .issues
        .push(Issue::new("GET_DESCRIPTOR", Some(31), "PRIVATE-SERIAL"));
    s.devices[0]
        .descriptors
        .push(usb_doctor::descriptors::Descriptor {
            kind: "Configuration".into(),
            index: 0,
            language: 0,
            raw: vec![9, 2],
            complete: false,
            notes: vec!["PRIVATE-NOTE".into()],
        });
    let mask = FieldSelection::none();
    let safe = report::selected(&s, &mask, false);
    assert_eq!(safe["scan_errors"][0]["code"], 5);
    assert_eq!(safe["devices"][0]["query_errors"][0]["code"], 31);
    assert!(
        safe["devices"][0]["descriptor_blocks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|b| b["complete"] == false)
    );
    assert!(!safe.to_string().contains("PRIVATE"));
    let html = report::selected_html(&s, &mask, false);
    assert!(html.contains("неполное"));
    assert!(html.contains("GET_DESCRIPTOR"));
    assert!(html.contains("Неполный дескриптор"));
    assert!(!html.contains("PRIVATE"));
    assert!(report::selected_html(&s, &mask, true).contains("PRIVATE-SERIAL"));
    let mut complete = s.clone();
    complete.inventory_complete = true;
    complete.issues.clear();
    for d in &mut complete.devices {
        d.issues.clear();
        d.descriptors.clear();
    }
    assert_ne!(html, report::selected_html(&complete, &mask, false));
}
#[test]
fn search_tracks_visible_selection_and_matches_usb_ids() {
    let s = demo::snapshot();
    let index = s
        .devices
        .iter()
        .position(|d| d.name.contains("клавиатура"))
        .unwrap();
    assert_eq!(s.visible_selection(false, 0, "клавиатура"), Some(index));
    assert_eq!(s.visible_selection(false, 0, "NO SUCH DEVICE xyz"), None);
    let d = &s.devices[0];
    assert!(d.matches_search(&format!("vid_{:04X}", d.vendor_id.unwrap())));
    assert!(d.matches_search("DEMO-0000"));
}
fn parent(d: &mut usb_doctor::model::Device, key: &str) {
    d.fields.push(Field::new(
        "node.parent_key",
        "Родитель",
        "Windows",
        key,
        "fixture",
        true,
    ));
}
#[test]
fn tree_uses_identity_and_handles_cycles_and_duplicate_hubs() {
    let mut s = Snapshot::empty(true);
    let mut root = demo::snapshot().devices.remove(0);
    root.key = "controller".into();
    root.name = "Same name".into();
    root.fields.clear();
    let mut hub = root.clone();
    hub.key = "hub".into();
    parent(&mut hub, "controller");
    let mut leaf = root.clone();
    leaf.key = "leaf".into();
    parent(&mut leaf, "hub");
    s.topology = vec![hub.clone(), root];
    s.devices = vec![leaf, hub];
    let rows = s.navigation_rows(true);
    assert_eq!(
        rows.iter()
            .map(|(d, n)| (d.key.as_str(), *n))
            .collect::<Vec<_>>(),
        vec![("controller", 0), ("hub", 1), ("leaf", 2)]
    );
    parent(&mut s.topology[1], "hub");
    let rows = s.navigation_rows(true);
    assert_eq!(rows.len(), 3);
}
#[test]
fn empty_inventory_has_no_selected_device() {
    assert_eq!(Snapshot::empty(false).visible_selection(false, 0, ""), None);
}
