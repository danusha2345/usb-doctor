use usb_doctor::{
    class_descriptors::{capability, class_fields},
    descriptors::{self, Descriptor},
    fields::Field,
};
fn value<'a>(f: &'a [Field], key: &str) -> &'a str {
    &f.iter().find(|f| f.id.ends_with(key)).unwrap().value
}
#[test]
fn video_frame_intervals_and_dimensions() {
    let mut b = vec![0; 30];
    b[..5].copy_from_slice(&[30, 0x24, 7, 1, 0]);
    b[5..7].copy_from_slice(&1920u16.to_le_bytes());
    b[7..9].copy_from_slice(&1080u16.to_le_bytes());
    b[21..25].copy_from_slice(&333333u32.to_le_bytes());
    b[25] = 1;
    b[26..30].copy_from_slice(&666666u32.to_le_bytes());
    let (f, _) = class_fields(&b, 14, 2, 0, "test", "test");
    assert_eq!(value(&f, "width"), "1920");
    assert_eq!(value(&f, "height"), "1080");
    assert_eq!(value(&f, "fps"), "30.000");
    assert_eq!(value(&f, "interval.0"), "666666");
}
#[test]
fn audio_versions_have_different_layouts() {
    let b = [11, 0x24, 2, 1, 2, 3, 24, 1, 0x80, 0xbb, 0];
    let (f, _) = class_fields(&b, 1, 2, 0, "test", "test");
    assert_eq!(value(&f, "channels"), "2");
    assert_eq!(value(&f, "resolution"), "24");
    assert_eq!(value(&f, "frequency.0"), "48000");
    let (f, _) = class_fields(&[6, 0x24, 2, 1, 4, 32], 1, 2, 0x20, "test", "test");
    assert_eq!(value(&f, "sample_bytes"), "4");
    assert_eq!(value(&f, "resolution"), "32");
    assert!(!f.iter().any(|f| f.id.ends_with("channels")));
    let (f, _) = class_fields(&b, 1, 2, 0x30, "test", "test");
    assert!(!f.iter().any(|f| f.id.contains("frequency")));
}
#[test]
fn midi_and_cdc_collect_class_string_references() {
    let (f, refs) = class_fields(&[9, 0x24, 3, 1, 4, 1, 2, 1, 9], 1, 3, 0, "x", "x");
    assert_eq!(refs, vec![9]);
    assert_eq!(value(&f, "sources"), "02 01");
    let (f, refs) = class_fields(
        &[13, 0x24, 15, 7, 0, 0, 0, 0, 0xea, 5, 1, 0, 0],
        2,
        6,
        0,
        "x",
        "x",
    );
    assert_eq!(refs, vec![7]);
    assert_eq!(value(&f, "segment_bytes"), "1514");
}
#[test]
fn bos_ssp_is_capability_not_current_link_speed() {
    let mut b = vec![16, 16, 10, 0, 0, 0, 0, 0, 0, 0x11, 0, 0];
    b.extend_from_slice(&0x000a4030u32.to_le_bytes());
    let f = capability(&b, "usb.bos.5");
    assert!(value(&f, "sublink.0").contains("10000000000 бит/с"));
    assert!(f.iter().all(|f| !f.label.contains("Текущ")));
}
#[test]
fn bos_container_is_sensitive_and_counts_checked() {
    let mut raw = vec![5, 15, 25, 0, 2, 20, 16, 4, 0];
    raw.extend([1; 16]);
    let mut d = Descriptor::new("BOS", 0, 0, raw);
    let f = descriptors::bos(&mut d);
    assert!(!d.complete);
    assert!(f.iter().find(|f| f.id.ends_with("uuid")).unwrap().sensitive);
}
#[test]
fn configuration_routes_by_interface_context_and_preserves_raw() {
    let mut raw = vec![9, 2, 0, 0, 2, 1, 0, 0x80, 50];
    raw.extend([9, 4, 0, 0, 0, 2, 2, 1, 0]);
    raw.extend([5, 0x24, 0, 0x20, 1]);
    raw.extend([9, 4, 1, 0, 0, 1, 3, 0, 0]);
    raw.extend([6, 0x24, 2, 1, 4, 9]);
    let n = raw.len() as u16;
    raw[2..4].copy_from_slice(&n.to_le_bytes());
    let mut d = Descriptor::new("Configuration", 0, 0, raw.clone());
    let (f, refs) = descriptors::configuration(&mut d, false);
    assert_eq!(refs, vec![9]);
    assert_eq!(d.raw, raw);
    assert!(f.iter().any(|f| f.value == "CDC Header"));
    assert!(f.iter().any(|f| f.value == "MIDI IN Jack"));
}
#[test]
fn all_truncated_class_and_bos_inputs_are_bounded() {
    for class in [1, 2, 14, 255] {
        for sub in 0..4 {
            for protocol in [0, 0x20, 0x30] {
                for typ in [0x24, 0x25] {
                    for subtype in 0..32 {
                        for len in 0..40 {
                            let mut raw = vec![255; len];
                            if len > 0 {
                                raw[0] = len as u8;
                            }
                            if len > 1 {
                                raw[1] = typ;
                            }
                            if len > 2 {
                                raw[2] = subtype;
                            }
                            let _ = class_fields(&raw, class, sub, protocol, "test", "test");
                        }
                    }
                }
            }
        }
    }
    for len in 0..40 {
        for typ in 0..20 {
            let mut b = vec![255; len];
            if len > 2 {
                b[2] = typ;
            }
            let _ = capability(&b, "test");
        }
    }
}
#[test]
fn truncated_variable_descriptor_marks_configuration_incomplete() {
    let raw = vec![
        9, 2, 24, 0, 1, 1, 0, 0x80, 50, 9, 4, 0, 0, 0, 1, 3, 0, 0, 6, 0x24, 3, 1, 4, 20,
    ];
    let mut d = Descriptor::new("Configuration", 0, 0, raw);
    let _ = descriptors::configuration(&mut d, false);
    assert!(!d.complete);
    assert!(d.notes.iter().any(|n| n.contains("class-specific")));
}
#[cfg(windows)]
#[test]
fn windows_monitor_registers_and_unregisters() {
    for _ in 0..4 {
        let monitor = usb_doctor::monitor::Monitor::new().expect("CM_Register_Notification");
        let _ = monitor.take_changed();
        drop(monitor);
    }
}
