use usb_doctor::{
    demo,
    fields::Field,
    inventory::{self, Column, Options, ThemeChoice},
    live,
};
fn fast_from(s: &usb_doctor::model::Snapshot) -> usb_doctor::model::Snapshot {
    let mut f = s.clone();
    for d in f.devices.iter_mut().chain(&mut f.topology) {
        d.descriptors.clear();
        d.fields
            .retain(|f| f.id == "node.parent_key" || f.id == "node.kind");
        d.fields.push(Field::new(
            "live.details_pending",
            "",
            "",
            "pending",
            "PnP",
            false,
        ));
    }
    f
}
#[test]
fn quick_presence_keeps_details_for_existing_nodes_and_removes_absent() {
    let old = demo::snapshot();
    let mut fast = fast_from(&old);
    fast.devices.remove(1);
    let merged = live::merge_presence(Some(&old), fast);
    assert_eq!(merged.devices.len(), 3);
    assert_eq!(merged.devices[0].descriptors, old.devices[0].descriptors);
    assert!(!merged.devices.iter().any(|d| d.key == "demo:keyboard"));
}
#[test]
fn incomplete_fast_pass_cannot_remove_devices() {
    let old = demo::snapshot();
    let mut fast = fast_from(&old);
    fast.devices.clear();
    fast.inventory_complete = false;
    let merged = live::merge_presence(Some(&old), fast);
    assert_eq!(merged.devices.len(), 4);
    assert!(!merged.inventory_complete);
}
#[test]
fn late_detailed_result_cannot_resurrect_removed_device() {
    let old = demo::snapshot();
    let mut live = fast_from(&old);
    live.devices.remove(1);
    let merged = live::apply_details(&live, old);
    assert_eq!(merged.devices.len(), 3);
    assert!(!merged.devices.iter().any(|d| d.key == "demo:keyboard"));
}
#[test]
fn hub_specific_fields_survive_enrichment() {
    let mut full = demo::snapshot();
    let hub = full.topology[1].clone();
    full.devices.push(hub);
    full.topology[1]
        .fields
        .push(Field::new("hub.special", "", "", "123", "Hub", false));
    let quick = fast_from(&full);
    let merged = live::apply_details(&quick, full);
    assert!(
        merged.topology[1]
            .fields
            .iter()
            .any(|f| f.id == "hub.special")
    );
}
#[test]
fn address_alias_migrates_and_can_be_unchecked() {
    let mut o = Options::default();
    o.extra_columns
        .insert("usb.address".into(), "USB Device Address".into());
    o.widths.insert("Адрес USB".into(), 92.0);
    o.normalize();
    assert_eq!(
        inventory::columns(&o)
            .iter()
            .filter(|c| c.standard == Some(Column::Address))
            .count(),
        1
    );
    assert!(!o.extra_columns.contains_key("usb.address"));
    assert_eq!(o.widths["USB-адрес"], 92.0);
    o.set_field("usb.address", "USB-адрес", false);
    assert!(!o.field_selected("usb.address"));
    o.set_field("usb.address", "USB-адрес", true);
    assert!(o.field_selected("usb.address"));
}
#[test]
fn column_reorder_preserves_visibility_and_serializes() {
    let mut o = Options::default();
    o.reorder("VID:PID", "name");
    let order = inventory::display_columns(&o);
    assert_eq!(order[0].id, "VID:PID");
    assert_eq!(order[1].id, "name");
    o.theme = ThemeChoice::Dark;
    let restored: Options = serde_json::from_slice(&serde_json::to_vec(&o).unwrap()).unwrap();
    assert_eq!(inventory::display_columns(&restored)[0].id, "VID:PID");
    assert!(restored.theme == ThemeChoice::Dark);
    assert_eq!(restored.highlight_seconds, 5);
}
#[test]
fn fast_new_children_count_before_port_is_known() {
    let full = demo::snapshot();
    let mut quick = fast_from(&full);
    for d in &mut quick.devices {
        d.port = None;
    }
    let changes = Default::default();
    let rows = inventory::rows(&quick, &Options::default(), "", &changes);
    assert_eq!(
        rows.iter()
            .find(|r| r.device.key == "demo:hub")
            .unwrap()
            .children,
        4
    );
}
#[test]
fn changing_parent_is_visible_and_drops_old_link_details() {
    let old = demo::snapshot();
    let mut fast = fast_from(&old);
    let d = &mut fast.devices[0];
    d.fields
        .iter_mut()
        .find(|f| f.id == "node.parent_key")
        .unwrap()
        .value = "new-hub".into();
    assert!(!live::same_presence(&old, &fast));
    let merged = live::merge_presence(Some(&old), fast);
    assert!(merged.devices[0].descriptors.is_empty());
    assert!(
        !merged.devices[0]
            .fields
            .iter()
            .any(|f| f.id == "usb.address")
    );
}
#[test]
fn full_scan_adds_and_removes_failed_ports_outside_interface_cache() {
    let live = demo::snapshot();
    let mut full = live.clone();
    let mut failed = live.devices[0].clone();
    failed.key = "hub:port:9".into();
    failed.connection_status = Some(2);
    full.devices.push(failed);
    let merged = live::apply_details(&live, full);
    assert!(merged.devices.iter().any(|d| d.key == "hub:port:9"));
    assert!(live::same_presence(&merged, &live));
    let removed = live::apply_details(&merged, live.clone());
    assert!(!removed.devices.iter().any(|d| d.key == "hub:port:9"));
}
#[test]
fn full_scan_can_fill_devices_when_fast_enumeration_was_incomplete() {
    let full = demo::snapshot();
    let mut partial = full.clone();
    partial.devices.clear();
    partial.inventory_complete = false;
    let merged = live::apply_details(&partial, full);
    assert_eq!(merged.devices.len(), 4);
}
