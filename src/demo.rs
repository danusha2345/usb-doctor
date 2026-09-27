use crate::model::{Device, Issue, Snapshot, SpeedEvidence};
pub fn snapshot() -> Snapshot {
    let mut s = Snapshot::empty(true);
    s.devices = vec![
        device("ssd", "Внешний SSD", Some(8), Some(2), Some(2)),
        device("keyboard", "USB-клавиатура", Some(3), Some(1), Some(0)),
        device("fast", "Накопитель SuperSpeed", Some(8), Some(2), Some(3)),
        device("unknown", "Неизвестное устройство", None, None, None),
    ];
    s.devices[3].issues.push(Issue::new(
        "EX_V2",
        Some(5),
        "Отказ в доступе — демонстрационный пример",
    ));
    for (i, d) in s.devices.iter_mut().enumerate().take(3) {
        d.fields.push(crate::fields::Field::new(
            "usb.manufacturer",
            "Производитель USB",
            "Строки USB",
            "Demo manufacturer",
            "Демо",
            false,
        ));
        d.fields.push(crate::fields::Field::new(
            "usb.serial",
            "Серийный номер USB",
            "Строки USB",
            format!("DEMO-{i:04}"),
            "Демо",
            true,
        ));
        d.fields.push(crate::fields::Field::new(
            "windows.DriverVersion",
            "Версия драйвера",
            "Windows",
            "10.0.26100.1",
            "Демо",
            false,
        ));
        let raw = vec![
            9,
            2,
            25,
            0,
            1,
            1,
            0,
            0x80,
            50,
            9,
            4,
            0,
            0,
            1,
            d.device_class.unwrap_or(0),
            0,
            0,
            0,
            7,
            5,
            0x81,
            3,
            64,
            0,
            10,
        ];
        let mut config = crate::descriptors::Descriptor::new("Configuration", 0, 0, raw);
        let (fields, _) = crate::descriptors::configuration(&mut config, false);
        d.fields.extend(fields);
        d.descriptors.push(config);
    }
    let mut controller = device("controller", "Контроллер USB · пример", None, None, None);
    controller.port = None;
    controller.fields.clear();
    controller.fields.push(crate::fields::Field::new(
        "node.kind",
        "Вид узла",
        "Основное",
        "Host controller",
        "Демо",
        false,
    ));
    let mut hub = device("hub", "Хаб USB · пример", Some(9), Some(2), Some(0));
    hub.is_hub = true;
    hub.port = Some(1);
    hub.fields.push(crate::fields::Field::new(
        "node.parent_key",
        "Родитель",
        "Windows",
        &controller.key,
        "Демо",
        true,
    ));
    for (i, d) in s.devices.iter_mut().enumerate() {
        d.port = Some(i as u32 + 1);
        d.fields.push(crate::fields::Field::new(
            "node.parent_key",
            "Родитель",
            "Windows",
            &hub.key,
            "Демо",
            true,
        ));
        d.fields.push(crate::fields::Field::new(
            "usb.address",
            "USB-адрес",
            "Основное",
            i + 2,
            "Демо",
            false,
        ));
    }
    s.topology = vec![controller, hub];
    s
}
fn device(
    id: &str,
    name: &str,
    class: Option<u8>,
    speed: Option<u8>,
    flags: Option<u32>,
) -> Device {
    Device {
        fields: Vec::new(),
        descriptors: Vec::new(),
        key: format!("demo:{id}"),
        name: name.into(),
        name_source: "Демонстрационные данные".into(),
        path: vec!["Компьютер — пример".into(), "Корневой хаб — пример".into()],
        port: Some(3),
        vendor_id: Some(0x1234),
        product_id: Some(0x5678),
        device_class: class,
        is_hub: false,
        connection_status: Some(1),
        windows_problem: Some(0),
        service: None,
        speed: SpeedEvidence {
            ex_speed: speed,
            v2_flags: flags,
            port_protocols: flags.map(|_| 7),
        },
        issues: Vec::new(),
    }
}
