use usb_doctor::{
    demo,
    diagnostics::{self, ComparisonError, Tone},
    model::{Speed, SpeedEvidence},
    report, wire,
};
fn speed(ex: Option<u8>, flags: Option<u32>) -> Speed {
    SpeedEvidence {
        ex_speed: ex,
        v2_flags: flags,
        port_protocols: None,
    }
    .speed()
}
#[test]
fn ex_high_is_not_enough_to_accuse_usb2() {
    assert_eq!(speed(Some(2), None), Speed::Unknown);
}
#[test]
fn v2_overrides_legacy_speed() {
    assert_eq!(speed(Some(2), Some(3)), Speed::Super);
    assert_eq!(speed(Some(2), Some(15)), Speed::SuperPlus);
}
#[test]
fn ssp_does_not_invent_lane_rate() {
    assert_eq!(
        speed(Some(2), Some(15)).label(),
        "SuperSpeedPlus · точная скорость неизвестна"
    );
}
#[test]
fn low_and_full_are_valid_without_v2() {
    assert_eq!(speed(Some(0), None), Speed::Low);
    assert_eq!(speed(Some(1), None), Speed::Full);
}
#[test]
fn conflicting_and_unknown_speed_values_stay_unknown() {
    for (ex, f) in [(1, 3), (0, 15), (2, 1), (2, 4), (2, 16), (8, 0)] {
        assert_eq!(speed(Some(ex), Some(f)), Speed::Unknown);
    }
}
#[test]
fn absence_and_false_capability_are_distinct() {
    assert_eq!(SpeedEvidence::default().super_capable(), None);
    assert_eq!(
        SpeedEvidence {
            v2_flags: Some(0),
            ..Default::default()
        }
        .super_capable(),
        Some(false)
    );
}
#[test]
fn device_capability_is_not_port_capability() {
    let mut d = demo::snapshot().devices.remove(1);
    d.speed.port_protocols = Some(7);
    assert_eq!(diagnostics::assess(&d).tone, Tone::Neutral);
}
#[test]
fn slow_ssd_gets_suggestion_not_damage_verdict() {
    let s = demo::snapshot();
    let a = diagnostics::assess(&s.devices[0]);
    assert_eq!(a.rule_id, "superspeed-capable-at-high");
    assert!(a.interpretation.contains("не установлена"));
}
#[test]
fn fullspeed_keyboard_is_not_a_bottleneck() {
    let s = demo::snapshot();
    assert_eq!(diagnostics::assess(&s.devices[1]).tone, Tone::Neutral);
}
#[test]
fn unknown_is_not_hardware_failure() {
    let s = demo::snapshot();
    let a = diagnostics::assess(&s.devices[3]);
    assert_eq!(a.tone, Tone::Unknown);
    assert!(a.interpretation.contains("не доказывают"));
}
#[test]
fn windows_problem_has_priority() {
    let mut d = demo::snapshot().devices.remove(2);
    d.windows_problem = Some(43);
    assert_eq!(diagnostics::assess(&d).rule_id, "windows-reported-problem");
}
#[test]
fn same_id_does_not_skip_user_identity_check() {
    let s = demo::snapshot();
    let d = &s.devices[0];
    assert_eq!(
        diagnostics::compare(d, d, false, true),
        Err(ComparisonError::IdentityNotConfirmed)
    );
}
#[test]
fn live_and_demo_cannot_be_compared() {
    let s = demo::snapshot();
    assert_eq!(
        diagnostics::compare(&s.devices[0], &s.devices[2], true, false),
        Err(ComparisonError::MixedDemoAndLive)
    );
}
#[test]
fn changed_speed_needs_both_observations() {
    let s = demo::snapshot();
    let c = diagnostics::compare(&s.devices[0], &s.devices[2], true, true).unwrap();
    assert!(c.changed);
    assert_eq!(c.before, Speed::High);
    assert_eq!(c.after, Speed::Super);
}
#[test]
fn report_excludes_free_text_and_private_ids() {
    let mut s = demo::snapshot();
    for d in &mut s.devices {
        d.name = "SECRET-NAME<script>alert(1)</script>".into();
        d.key = "USB\\PRIVATE-SERIAL".into();
        d.path = vec!["C:\\Users\\PRIVATE-USER".into()];
        d.service = Some("PRIVATE-SERVICE".into());
        for i in &mut d.issues {
            i.reason = "PRIVATE-REASON".into();
        }
    }
    let json = serde_json::to_string(&report::redacted(&s)).unwrap();
    let html = report::html(&s);
    for token in [
        "SECRET-NAME",
        "PRIVATE-SERIAL",
        "PRIVATE-USER",
        "PRIVATE-SERVICE",
        "PRIVATE-REASON",
        "<script>",
    ] {
        assert!(!json.contains(token), "{token}");
        assert!(!html.contains(token), "{token}");
    }
    assert!(report::redacted(&s)["demo"].as_bool().unwrap());
}
#[test]
fn report_escapes_even_unexpected_error_operation() {
    let mut s = demo::snapshot();
    s.issues.push(usb_doctor::model::Issue::new(
        "</pre><script>alert(1)</script>",
        None,
        "x",
    ));
    assert!(!report::html(&s).contains("<script>"));
}
#[test]
fn parsers_reject_truncation_and_never_panic() {
    for len in 0..512 {
        let b = vec![0xff; len];
        let _ = wire::connection(&b);
        let _ = wire::v2(&b, 1);
        let _ = wire::driver_key(&b, 1);
        let _ = wire::utf16z(&b);
        if len < 35 {
            assert!(wire::connection(&b).is_none());
        }
    }
    assert!(wire::u32_at(&[], usize::MAX).is_none());
}
fn ex_fixture() -> Vec<u8> {
    let mut b = vec![0; 35];
    b[0..4].copy_from_slice(&1u32.to_le_bytes());
    b[4] = 18;
    b[5] = 1;
    b[8] = 8;
    b[12..14].copy_from_slice(&0x1234u16.to_le_bytes());
    b[14..16].copy_from_slice(&0xabcd_u16.to_le_bytes());
    b[23] = 2;
    b[31..35].copy_from_slice(&1u32.to_le_bytes());
    b
}
#[test]
fn packed_ex_fields_decode_without_pipes() {
    let b = ex_fixture();
    let x = wire::connection(&b).unwrap();
    assert_eq!(x.vendor, Some(0x1234));
    assert_eq!(x.product, Some(0xabcd));
    assert_eq!(x.class, Some(8));
    assert_eq!(x.speed, 2);
    assert_eq!(x.status, 1);
}
#[test]
fn malformed_descriptor_does_not_invent_vid_pid() {
    let mut b = ex_fixture();
    b[4] = 0;
    let x = wire::connection(&b).unwrap();
    assert_eq!(x.vendor, None);
    assert_eq!(x.product, None);
}
#[test]
fn v2_validates_port_and_size() {
    let mut b = vec![0; 16];
    b[0..4].copy_from_slice(&3u32.to_le_bytes());
    b[4..8].copy_from_slice(&16u32.to_le_bytes());
    b[8] = 4;
    b[12] = 3;
    assert_eq!(wire::v2(&b, 3), Some((4, 3)));
    assert_eq!(wire::v2(&b, 2), None);
    b[4] = 15;
    assert_eq!(wire::v2(&b, 3), None);
}
#[test]
fn driverkey_requires_bounded_length_and_terminator() {
    let mut b = vec![0; 12];
    b[0] = 1;
    b[4] = 12;
    b[8] = b'A';
    assert_eq!(wire::driver_key(&b, 1), Some("A".into()));
    b[4] = 250;
    assert_eq!(wire::driver_key(&b, 1), None);
    b[4] = 12;
    b[10] = b'B';
    assert_eq!(wire::driver_key(&b, 1), None);
}
#[test]
fn invalid_utf16_is_rejected_not_lossily_identified() {
    assert_eq!(wire::utf16z(&[0, 0xd8, 0, 0]), None);
    assert_eq!(wire::utf16z(&[b'A', 0]), None);
    assert_eq!(wire::utf16z(&[0]), None);
}
#[test]
fn export_refuses_overwrite() {
    let path = std::env::temp_dir().join(format!(
        "usb-doctor-report-test-{}-{}.json",
        std::process::id(),
        usb_doctor::model::now_ms()
    ));
    std::fs::write(&path, b"original").unwrap();
    let result = report::save_new(&demo::snapshot(), &path);
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(std::fs::read(&path).unwrap(), b"original");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn changed_connection_discards_old_speed_and_windows_identity() {
    let mut d = demo::snapshot().devices.remove(0);
    d.invalidate_connection();
    assert_eq!(d.speed.speed(), Speed::Unknown);
    assert_eq!(d.vendor_id, None);
    assert_eq!(d.windows_problem, None);
    assert!(d.service.is_none());
    assert_eq!(diagnostics::assess(&d).tone, Tone::Unknown);
}
#[test]
fn new_usb_address_is_not_the_same_observation() {
    let a = ex_fixture();
    let mut b = a.clone();
    b[25] = 9;
    assert_ne!(wire::connection(&a), wire::connection(&b));
}

#[test]
fn selection_hides_fields_without_discarding_collected_data() {
    let s = demo::snapshot();
    let original = serde_json::to_value(&s).unwrap();
    let fields = usb_doctor::fields::for_device(&s.devices[0]);
    let mut selected = usb_doctor::fields::FieldSelection::all();
    selected.set("speed", false, &fields);
    assert!(!selected.contains("speed"));
    assert!(selected.contains("vid"));
    let view = report::selected(&s, &selected, false);
    assert!(
        !view["devices"][0]["fields"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "speed")
    );
    assert_eq!(serde_json::to_value(&s).unwrap(), original);
}
#[test]
fn selected_report_respects_both_mask_and_privacy() {
    let s = demo::snapshot();
    let mut selection = usb_doctor::fields::FieldSelection {
        all: false,
        included: Default::default(),
        excluded: Default::default(),
    };
    selection.included.insert("usb.serial".into());
    let safe = report::selected(&s, &selection, false);
    assert!(safe["devices"][0]["fields"].as_array().unwrap().is_empty());
    let full = report::selected(&s, &selection, true);
    assert_eq!(full["devices"][0]["fields"][0]["value"], "DEMO-0000");
}
#[test]
fn field_selection_survives_roundtrip() {
    let selection = usb_doctor::fields::FieldSelection::default();
    assert_eq!(
        serde_json::from_str::<usb_doctor::fields::FieldSelection>(
            &serde_json::to_string(&selection).unwrap()
        )
        .unwrap(),
        selection
    );
}
#[test]
fn malformed_configuration_cannot_loop_or_overrun() {
    use usb_doctor::descriptors::*;
    for length in 0..128 {
        let mut d = Descriptor::new("Configuration", 0, 0, vec![0xff; length]);
        let _ = configuration(&mut d, false);
        assert!(!d.complete);
    }
    let mut d = Descriptor::new(
        "Configuration",
        0,
        0,
        vec![9, 2, 11, 0, 0, 1, 0, 128, 50, 0, 4],
    );
    let _ = configuration(&mut d, false);
    assert!(!d.complete);
}
#[test]
fn config_keeps_unknown_class_descriptor_bytes() {
    use usb_doctor::descriptors::*;
    let raw = vec![9, 2, 14, 0, 0, 1, 0, 128, 50, 5, 0x24, 0x99, 0xab, 0xcd];
    let mut d = Descriptor::new("Configuration", 0, 0, raw.clone());
    let (fields, _) = configuration(&mut d, false);
    assert!(d.complete);
    assert_eq!(d.raw, raw);
    assert!(fields.iter().any(|f| f.value.contains("99 AB CD")));
}
#[test]
fn max_power_uses_descriptor_family_units() {
    use usb_doctor::descriptors::*;
    let raw = vec![9, 2, 9, 0, 0, 1, 0, 128, 50];
    let (mut a, mut b) = (
        Descriptor::new("Configuration", 0, 0, raw.clone()),
        Descriptor::new("Configuration", 0, 0, raw),
    );
    assert!(
        configuration(&mut a, false)
            .0
            .iter()
            .any(|f| f.value.starts_with("100 мА"))
    );
    assert!(
        configuration(&mut b, true)
            .0
            .iter()
            .any(|f| f.value.starts_with("400 мА"))
    );
}
#[test]
fn descriptor_string_is_length_prefixed_not_zero_terminated() {
    assert_eq!(
        usb_doctor::descriptors::text(&[6, 3, b'A', 0, b'B', 0]),
        Some("AB".into())
    );
    assert_eq!(
        usb_doctor::descriptors::text(&[8, 3, b'A', 0, b'B', 0]),
        None
    );
    assert_eq!(usb_doctor::descriptors::text(&[4, 3, 0, 0xd8]), None);
}
#[test]
fn ssp_lane_math_is_checked() {
    let ten_gbit = (10 << 16) | (3 << 4);
    assert_eq!(wire::ssp_bps(ten_gbit, 0), Some(10_000_000_000));
    assert_eq!(wire::ssp_bps(ten_gbit, 1), Some(20_000_000_000));
    assert_eq!(wire::ssp_bps(ten_gbit, 99), None);
    assert_eq!(wire::ssp_bps(0, 0), None);
}
#[test]
fn selected_html_escapes_property_values() {
    let mut s = demo::snapshot();
    s.devices[0].fields.push(usb_doctor::fields::Field::new(
        "attack",
        "<script>",
        "Windows",
        "<img src=x onerror=alert(1)>",
        "test",
        false,
    ));
    let html = report::selected_html(&s, &usb_doctor::fields::FieldSelection::all(), false);
    assert!(!html.contains("<script>"));
    assert!(!html.contains("<img"));
    assert!(html.contains("&lt;img"));
}

#[test]
fn empty_zero_echo_is_not_a_false_scan_failure() {
    let mut b = ex_fixture();
    b[..4].fill(0);
    b[31..35].fill(0);
    assert!(wire::connection_for_port(&b, 7).is_some());
    b[31] = 1;
    assert!(wire::connection_for_port(&b, 7).is_none());
}

#[test]
fn bcd_versions_use_decimal_digits() {
    assert_eq!(usb_doctor::descriptors::bcd_text(0x4401), "44.01 (0x4401)");
    assert_eq!(usb_doctor::descriptors::bcd_text(0x0310), "3.10 (0x0310)");
    assert!(usb_doctor::descriptors::bcd_text(0xffff).contains("не BCD"));
}

#[test]
fn windows_power_state_is_reported_not_measured_current() {
    let mut raw = vec![0; 64];
    raw[0] = 64;
    raw[4] = 3;
    let fields = usb_doctor::descriptors::power_fields(&raw);
    assert_eq!(fields[0].value, "D2");
    assert!(fields[0].source.contains("MostRecent"));
    raw[0] = 80;
    assert!(usb_doctor::descriptors::power_fields(&raw).is_empty());
}

#[test]
fn other_speed_configuration_has_distinct_field_ids() {
    let raw = vec![9, 7, 9, 0, 0, 1, 0, 128, 50];
    let mut d = usb_doctor::descriptors::Descriptor::new("Other speed configuration", 0, 0, raw);
    let (fields, _) = usb_doctor::descriptors::configuration(&mut d, false);
    assert!(d.complete);
    assert!(fields.iter().all(|f| f.id.starts_with("usb.other_config.")));
}
#[test]
fn topology_selection_never_returns_device_at_same_index() {
    let mut snapshot = demo::snapshot();
    let mut hub = snapshot.devices[0].clone();
    hub.key = "topology-only".into();
    snapshot.topology.push(hub);
    let index = snapshot
        .navigation_rows(true)
        .iter()
        .position(|(d, _)| d.key == "topology-only")
        .unwrap();
    assert_eq!(
        snapshot.selected_node(true, index).unwrap().key,
        "topology-only"
    );
    assert_ne!(
        snapshot.selected_node(false, 0).unwrap().key,
        "topology-only"
    );
    assert!(snapshot.selected_node(true, usize::MAX).is_none());
}
