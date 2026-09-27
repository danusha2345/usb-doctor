use usb_doctor::{
    demo,
    i18n::{Language, translate},
    inventory::{Column, Options, display_columns},
};

#[test]
fn language_roundtrip_preserves_column_ids_and_settings() {
    let mut options = Options::default();
    options.set_field("windows.DriverVersion", "Версия драйвера", true);
    options.reorder("VID:PID", "name");
    let before = display_columns(&options)
        .iter()
        .map(|c| c.id.clone())
        .collect::<Vec<_>>();
    options.language = Language::English;
    let restored: Options =
        serde_json::from_str(&serde_json::to_string(&options).unwrap()).unwrap();
    assert_eq!(restored.language, Language::English);
    assert_eq!(
        before,
        display_columns(&restored)
            .iter()
            .map(|c| c.id.clone())
            .collect::<Vec<_>>()
    );
    assert!(restored.field_selected("windows.DriverVersion"));
}
#[test]
fn old_settings_default_to_russian() {
    let old: Options = serde_json::from_str("{}").unwrap();
    assert_eq!(old.language, Language::Russian);
}
#[test]
fn messages_translate_numbers_nested_labels_and_units() {
    assert_eq!(
        translate(
            Language::English,
            "Подключено устройств: 8 · USB-хабов: 6 · нажмите строку для подробностей"
        ),
        "Connected devices: 8 · USB hubs: 6 · click a row for details"
    );
    assert_eq!(
        translate(Language::English, "Клавиатура + Мышь"),
        "Keyboard + Mouse"
    );
    assert_eq!(
        translate(Language::English, "Текущий режим: 480 Мбит/с · High-Speed."),
        "Current speed: 480 Mbit/s · High-Speed."
    );
    assert_eq!(
        translate(Language::English, "Конфигурация · интерфейс 2 / alt 0"),
        "Configuration · interface 2 / alt 0"
    );
}
#[test]
fn identifiers_and_snapshot_are_not_modified_by_ui_language() {
    let snapshot = demo::snapshot();
    let before = serde_json::to_vec(&snapshot).unwrap();
    usb_doctor::i18n::set_language(Language::English);
    assert_eq!(
        usb_doctor::i18n::t("USB\\VID_1234&PID_ABCD\\serial"),
        "USB\\VID_1234&PID_ABCD\\serial"
    );
    assert_eq!(usb_doctor::i18n::t("Мой прибор №42"), "Мой прибор №42");
    assert_eq!(before, serde_json::to_vec(&snapshot).unwrap());
    usb_doctor::i18n::set_language(Language::Russian);
}
#[test]
fn unavailable_optional_queries_are_not_reported_as_device_failure() {
    let mut d = demo::snapshot().devices.remove(0);
    d.windows_problem = Some(0);
    d.connection_status = Some(1);
    d.issues.push(usb_doctor::model::Issue::new(
        "optional",
        Some(5),
        "Отказ в доступе",
    ));
    assert_eq!(
        Column::Status.value(&d),
        "Подключено · часть данных недоступна"
    );
    d.windows_problem = Some(43);
    assert_eq!(Column::Status.value(&d), "Код Windows 43");
}
#[test]
fn catalogue_matches_all_compiled_translations() {
    let catalogue: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!("../src/translations.json")).unwrap();
    for (ru, en) in catalogue {
        // Fill each run of adjacent Rust placeholders with a distinct neutral token.
        fn fill(text: &str) -> String {
            let mut rest = text;
            let mut result = String::new();
            let mut index = 0;
            while let Some(start) = rest.find('{') {
                result.push_str(&rest[..start]);
                rest = &rest[start..];
                while rest.starts_with('{') {
                    rest = &rest[rest.find('}').unwrap() + 1..];
                }
                result.push_str(&format!("VALUE{index}"));
                index += 1;
            }
            result.push_str(rest);
            result
        }
        assert_eq!(translate(Language::Russian, &ru), ru);
        assert_eq!(translate(Language::English, &fill(&ru)), fill(&en), "{ru}");
    }
}
