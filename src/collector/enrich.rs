use super::windows::{ioctl, query_port};
use crate::{
    descriptors::{self, Descriptor},
    fields::{Field, hex},
    model::{Device, Issue},
    wire,
};
use std::{mem::size_of, ptr::null_mut};
use windows_sys::{
    Win32::{
        Devices::{DeviceAndDriverInstallation::*, Properties::*, Usb::*},
        Foundation::*,
    },
    core::GUID,
};
#[path = "property_names.rs"]
mod names;
fn guid(g: &GUID) -> u128 {
    (u128::from(g.data1) << 96)
        | (u128::from(g.data2) << 80)
        | (u128::from(g.data3) << 64)
        | u128::from(u64::from_be_bytes(g.data4))
}
pub fn properties(set: HDEVINFO, dev: &SP_DEVINFO_DATA) -> Vec<Field> {
    let mut count = 0;
    unsafe {
        SetupDiGetDevicePropertyKeys(set, dev, null_mut(), 0, &mut count, 0);
    }
    if count == 0 || count > 512 {
        return vec![Field::new(
            "windows.properties_status",
            "Список свойств",
            "Windows",
            format!("Не получен; Win32={}", unsafe { GetLastError() }),
            "SetupDiGetDevicePropertyKeys",
            false,
        )];
    }
    let mut keys = vec![
        DEVPROPKEY {
            fmtid: GUID::from_u128(0),
            pid: 0
        };
        count as usize
    ];
    if unsafe { SetupDiGetDevicePropertyKeys(set, dev, keys.as_mut_ptr(), count, &mut count, 0) }
        == 0
        || count as usize > keys.len()
    {
        return Vec::new();
    }
    let mut out = Vec::new();
    for key in keys.into_iter().take(count as usize) {
        let mut ty = 0;
        let mut size = 0;
        unsafe {
            SetupDiGetDevicePropertyW(set, dev, &key, &mut ty, null_mut(), 0, &mut size, 0);
        }
        let (gid, name) = names::label(guid(&key.fmtid), key.pid);
        let id = if gid == "Unknown" {
            format!("windows.{:032x}.{}", guid(&key.fmtid), key.pid)
        } else {
            format!("windows.{gid}")
        };
        if size > 65536 {
            out.push(Field::new(
                id,
                name,
                "Windows",
                "Превышен лимит 64 KiB",
                "SetupAPI",
                false,
            ));
            continue;
        }
        let mut bytes = vec![0; size.max(1) as usize];
        let ok = unsafe {
            SetupDiGetDevicePropertyW(
                set,
                dev,
                &key,
                &mut ty,
                bytes.as_mut_ptr(),
                bytes.len() as u32,
                &mut size,
                0,
            )
        };
        if ok == 0 || size as usize > bytes.len() {
            out.push(Field::new(
                id,
                name,
                "Windows",
                format!("Недоступно; Win32={}", unsafe { GetLastError() }),
                "SetupAPI",
                false,
            ));
            continue;
        }
        bytes.truncate(size as usize);
        let sensitive = matches!(
            ty,
            DEVPROP_TYPE_STRING
                | DEVPROP_TYPE_STRING_LIST
                | DEVPROP_TYPE_STRING_INDIRECT
                | DEVPROP_TYPE_GUID
                | DEVPROP_TYPE_BINARY
                | DEVPROP_TYPE_SECURITY_DESCRIPTOR
                | DEVPROP_TYPE_SECURITY_DESCRIPTOR_STRING
        ) && !matches!(
            gid,
            "DriverVersion"
                | "DriverProvider"
                | "DriverDate"
                | "Manufacturer"
                | "Class"
                | "ClassGuid"
                | "DriverInfPath"
                | "DriverInfSection"
        );
        out.push(Field::new(
            id,
            name,
            "Windows",
            descriptors::property_value(ty, &bytes),
            &format!("SetupAPI property; type={ty}"),
            sensitive,
        ));
    }
    out
}
fn standard_descriptor(
    handle: HANDLE,
    port: u32,
    kind: u8,
    index: u8,
    lang: u16,
    len: u16,
) -> Result<Vec<u8>, Issue> {
    let mut buf = vec![0; 12 + usize::from(len)];
    buf[..4].copy_from_slice(&port.to_le_bytes());
    buf[4] = 0x80;
    buf[5] = 6;
    buf[6] = index;
    buf[7] = kind;
    buf[8..10].copy_from_slice(&lang.to_le_bytes());
    buf[10..12].copy_from_slice(&len.to_le_bytes());
    let response = ioctl(
        handle,
        IOCTL_USB_GET_DESCRIPTOR_FROM_NODE_CONNECTION,
        buf,
        "GET_DESCRIPTOR",
    )?;
    if response.len() < 12 || wire::u32_at(&response, 0) != Some(port) {
        return Err(Issue::new(
            "GET_DESCRIPTOR",
            None,
            "Неполный заголовок ответа",
        ));
    }
    Ok(response[12..].to_vec())
}
pub fn device_descriptors(handle: HANDLE, port: u32, ex: &[u8], d: &mut Device) {
    if ex.len() < 22 || ex[4] != 18 || ex[5] != 1 {
        return;
    }
    let raw = ex[4..22].to_vec();
    ms_os_descriptor(handle, port, &raw, d);
    let config_count = raw[17];
    let usb3 = descriptors::u16_at(&raw, 2).is_some_and(|x| x >= 0x0300);
    let mut indexes = vec![raw[14], raw[15], raw[16]];
    d.fields.extend(descriptors::device(&raw));
    d.descriptors
        .push(Descriptor::new("Device", 0, 0, raw.clone()));
    if let Some(value) = ex.get(22) {
        d.fields.push(Field::new(
            "usb.configuration_value",
            "Текущая конфигурация",
            "USB · устройство",
            value,
            "EX",
            false,
        ));
    }
    if let Some(count) = wire::u32_at(ex, 27) {
        d.fields.push(Field::new(
            "usb.open_pipes",
            "Открыто каналов",
            "USB · устройство",
            count,
            "EX.PipeList",
            false,
        ));
        for (i, x) in ex
            .get(35..)
            .unwrap_or_default()
            .as_chunks::<11>()
            .0
            .iter()
            .take(count as usize)
            .enumerate()
        {
            d.fields.push(Field::new(format!("usb.pipe.{i}"),format!("Открытый канал {i}"),"Endpoints",format!("address=0x{:02X}; attributes=0x{:02X}; maxPacket={}; interval={}; scheduleOffset={}",x[2],x[3],descriptors::u16_at(x,4).unwrap_or(0),x[6],wire::u32_at(x,7).unwrap_or(0)),"EX.PipeList",false));
        }
    }
    for index in 0..config_count {
        // A full 64 KiB request avoids drivers rejecting a header-only configuration query.
        match standard_descriptor(handle, port, 2, index, 0, u16::MAX) {
            Ok(b) => {
                let mut desc = Descriptor::new("Configuration", index, 0, b);
                let (fields, strings) = descriptors::configuration(&mut desc, usb3);
                d.fields.extend(fields);
                indexes.extend(strings);
                d.descriptors.push(desc);
            }
            Err(e) => d.issues.push(Issue::new(
                &format!("Configuration {index}"),
                e.code,
                &e.reason,
            )),
        }
    }
    if descriptors::u16_at(&raw, 2).is_some_and(|v| (0x0200..0x0300).contains(&v)) {
        match standard_descriptor(handle, port, 6, 0, 0, 10) {
            Ok(b) if b.len() >= 10 && b[0] == 10 && b[1] == 6 => {
                let count = b[8];
                d.fields.push(Field::new(
                    "usb.qualifier.version",
                    "Device qualifier · версия USB",
                    "USB · qualifier",
                    descriptors::bcd_text(descriptors::u16_at(&b, 2).unwrap()),
                    "Device qualifier",
                    false,
                ));
                d.fields.push(Field::new(
                    "usb.qualifier.class",
                    "Device qualifier · класс/подкласс/протокол",
                    "USB · qualifier",
                    format!("{:02X}/{:02X}/{:02X}", b[4], b[5], b[6]),
                    "Device qualifier",
                    false,
                ));
                d.fields.push(Field::new(
                    "usb.qualifier.packet",
                    "Device qualifier · EP0 max packet",
                    "USB · qualifier",
                    b[7],
                    "Device qualifier",
                    false,
                ));
                d.fields.push(Field::new(
                    "usb.qualifier.configs",
                    "Device qualifier · конфигураций",
                    "USB · qualifier",
                    count,
                    "Device qualifier",
                    false,
                ));
                d.descriptors
                    .push(Descriptor::new("Device qualifier", 0, 0, b));
                for index in 0..count {
                    match standard_descriptor(handle, port, 7, index, 0, u16::MAX) {
                        Ok(b) => {
                            let mut desc =
                                Descriptor::new("Other speed configuration", index, 0, b);
                            let (fields, strings) = descriptors::configuration(&mut desc, false);
                            d.fields.extend(fields);
                            indexes.extend(strings);
                            d.descriptors.push(desc);
                        }
                        Err(e) => d.issues.push(Issue::new(
                            &format!("Other speed configuration {index}"),
                            e.code,
                            &e.reason,
                        )),
                    }
                }
            }
            Ok(b) => {
                let mut desc = Descriptor::new("Device qualifier", 0, 0, b);
                desc.complete = false;
                desc.notes.push("Некорректный qualifier".into());
                d.descriptors.push(desc);
            }
            Err(e) => d
                .issues
                .push(Issue::new("Device qualifier (optional)", e.code, &e.reason)),
        }
    }
    if descriptors::u16_at(&raw, 2).is_some_and(|x| x >= 0x0201) {
        match standard_descriptor(handle, port, 15, 0, 0, u16::MAX) {
            Ok(b) => {
                let mut desc = Descriptor::new("BOS", 0, 0, b);
                d.fields.extend(descriptors::bos(&mut desc));
                d.descriptors.push(desc);
            }
            Err(e) => d.issues.push(Issue::new("BOS", e.code, &e.reason)),
        }
    }
    indexes.retain(|x| *x != 0);
    indexes.sort();
    indexes.dedup();
    if indexes.is_empty() {
        return;
    }
    match standard_descriptor(handle, port, 3, 0, 0, 255) {
        Ok(languages) => {
            let langs = descriptors::languages(&languages);
            d.descriptors
                .push(Descriptor::new("String languages", 0, 0, languages));
            for (lang_i, lang) in langs.into_iter().enumerate() {
                for index in &indexes {
                    match standard_descriptor(handle, port, 3, *index, lang, 255) {
                        Ok(b) => {
                            if let Some(text) = descriptors::text(&b) {
                                let (key, label) = if *index == raw[14] {
                                    ("manufacturer", "Производитель USB")
                                } else if *index == raw[15] {
                                    ("product", "Название USB")
                                } else if *index == raw[16] {
                                    ("serial", "Серийный номер USB")
                                } else {
                                    ("string", "Строка USB")
                                };
                                let id = if key != "string" && lang_i == 0 {
                                    format!("usb.{key}")
                                } else {
                                    format!("usb.string.{index}.{lang:04X}")
                                };
                                d.fields.push(Field::new(
                                    id,
                                    label,
                                    "Строки USB",
                                    text,
                                    "String descriptor",
                                    key == "serial" || key == "string",
                                ));
                            }
                            d.descriptors
                                .push(Descriptor::new("String", *index, lang, b));
                        }
                        Err(e) => d.issues.push(Issue::new(
                            &format!("String {index}/{lang:04X}"),
                            e.code,
                            &e.reason,
                        )),
                    }
                }
            }
        }
        Err(e) => d
            .issues
            .push(Issue::new("String languages", e.code, &e.reason)),
    }
}
pub fn port(handle: HANDLE, port: u32, d: &mut Device) {
    match query_port(
        handle,
        port,
        IOCTL_USB_GET_NODE_CONNECTION_ATTRIBUTES,
        12,
        "Port attributes",
    ) {
        Ok(b) => {
            if let Some(attrs) = wire::u32_at(&b, 8) {
                d.fields.push(Field::new(
                    "usb.port.attributes",
                    "Атрибуты порта",
                    "Порт",
                    format!("0x{attrs:08X}"),
                    "CONNECTION_ATTRIBUTES",
                    false,
                ));
            }
        }
        Err(e) => d.issues.push(e),
    }
    for companion in 0..3u16 {
        let mut b = vec![0; 4096];
        b[..4].copy_from_slice(&port.to_le_bytes());
        b[12..14].copy_from_slice(&companion.to_le_bytes());
        match ioctl(
            handle,
            IOCTL_USB_GET_PORT_CONNECTOR_PROPERTIES,
            b,
            "Port connector",
        ) {
            Ok(b) => {
                if let Some(props) = wire::u32_at(&b, 8) {
                    d.fields.push(Field::new(
                        format!("usb.port.connector.{companion}.flags"),
                        "Connector properties",
                        "Порт",
                        format!("0x{props:08X}"),
                        "PORT_CONNECTOR_PROPERTIES",
                        false,
                    ));
                }
                if let Some(n) = descriptors::u16_at(&b, 14) {
                    d.fields.push(Field::new(
                        format!("usb.port.connector.{companion}.port"),
                        "Companion port",
                        "Порт",
                        n,
                        "PORT_CONNECTOR_PROPERTIES",
                        false,
                    ));
                    if n == 0 {
                        break;
                    }
                }
                if let Some(name) = b.get(16..).and_then(wire::utf16z) {
                    d.fields.push(Field::new(
                        format!("usb.port.connector.{companion}.hub"),
                        "Companion hub",
                        "Порт",
                        name,
                        "PORT_CONNECTOR_PROPERTIES",
                        true,
                    ));
                }
            }
            Err(e) => {
                if companion == 0 {
                    d.issues.push(e);
                }
                break;
            }
        }
    }
    // Microsoft win32metadata shared/usbiodef.h: function 289, FILE_DEVICE_USB 0x22.
    // shared/usbioctl.h: six packed ULONGs, Rx/Tx lane counts encode count-1.
    if d.speed.v2_flags.is_some_and(|f| f & 4 != 0) {
        let mut b = vec![0; 24];
        b[..4].copy_from_slice(&port.to_le_bytes());
        b[4..8].copy_from_slice(&24u32.to_le_bytes());
        match ioctl(handle, (0x22 << 16) | (289 << 2), b, "SuperSpeedPlus lanes") {
            Ok(b)
                if b.len() >= 24
                    && wire::u32_at(&b, 0) == Some(port)
                    && wire::u32_at(&b, 4) == Some(24) =>
            {
                for (side, offset) in [("RX", 8), ("TX", 16)] {
                    let bits = wire::u32_at(&b, offset).unwrap();
                    let lanes = wire::u32_at(&b, offset + 4).unwrap();
                    d.fields.push(Field::new(
                        format!("usb.ssp.{side}.raw"),
                        format!("{side} lane attributes"),
                        "SuperSpeedPlus",
                        format!("0x{bits:08X}; laneCount={lanes} (raw)"),
                        "SSP_INFORMATION",
                        false,
                    ));
                    if let Some(bps) = crate::wire::ssp_bps(bits, lanes) {
                        d.fields.push(Field::new(
                            format!("usb.ssp.{side}.rate"),
                            format!("{side} согласованная скорость"),
                            "SuperSpeedPlus",
                            format!("{} Гбит/с", bps as f64 / 1e9),
                            "SSP_INFORMATION",
                            false,
                        ));
                    }
                }
            }
            Ok(_) => d.issues.push(Issue::new(
                "SSP_INFORMATION",
                None,
                "Неожиданная длина/порт",
            )),
            Err(e) => d.issues.push(e),
        }
    }
}
pub fn hub(handle: HANDLE) -> Vec<Field> {
    let mut out = Vec::new();
    for (code, len, name) in [
        (
            IOCTL_USB_GET_HUB_INFORMATION_EX,
            size_of::<USB_HUB_INFORMATION_EX>(),
            "Hub information EX",
        ),
        (IOCTL_USB_GET_HUB_CAPABILITIES_EX, 4, "Hub capabilities EX"),
    ] {
        match ioctl(handle, code, vec![0; len], name) {
            Ok(b) => {
                out.push(Field::new(
                    format!("hub.{code}.raw"),
                    name,
                    "Хаб",
                    hex(&b),
                    name,
                    false,
                ));
                if code == IOCTL_USB_GET_HUB_INFORMATION_EX {
                    if let Some(typ) = wire::u32_at(&b, 0) {
                        out.push(Field::new("hub.type", "Тип хаба", "Хаб", typ, name, false));
                    }
                    if let Some(n) = descriptors::u16_at(&b, 4) {
                        out.push(Field::new(
                            "hub.port_count",
                            "Число портов",
                            "Хаб",
                            n,
                            name,
                            false,
                        ));
                    }
                }
            }
            Err(e) => out.push(Field::new(
                format!("hub.{code}.error"),
                name,
                "Хаб",
                format!("Недоступно: {:?}", e.code),
                name,
                false,
            )),
        }
    }
    out
}
pub fn com_port(set: HDEVINFO, dev: &SP_DEVINFO_DATA) -> Option<Field> {
    use windows_sys::Win32::System::Registry::*;
    let key = unsafe { SetupDiOpenDevRegKey(set, dev, DICS_FLAG_GLOBAL, 0, DIREG_DEV, KEY_READ) };
    if key as isize == -1 || key.is_null() {
        return None;
    }
    let name: Vec<u16> = "PortName\0".encode_utf16().collect();
    let mut b = vec![0u8; 2048];
    let mut size = b.len() as u32;
    let rc = unsafe {
        RegGetValueW(
            key,
            std::ptr::null(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            b.as_mut_ptr().cast(),
            &mut size,
        )
    };
    unsafe {
        RegCloseKey(key);
    }
    if rc != 0 || size as usize > b.len() {
        return None;
    }
    let value = wire::utf16z(&b[..size as usize])?;
    if !value.starts_with("COM") {
        return None;
    }
    Some(Field::new(
        "windows.com_port",
        "COM-порт",
        "Windows",
        value,
        "Device registry PortName",
        false,
    ))
}
pub fn hid_caps(handle: HANDLE) -> Result<Vec<Field>, Issue> {
    use windows_sys::Win32::Devices::HumanInterfaceDevice::*;
    let mut preparsed = 0;
    if !unsafe { HidD_GetPreparsedData(handle, &mut preparsed) } {
        return Err(Issue::new(
            "HID preparsed data",
            Some(unsafe { GetLastError() }),
            "Windows не вернула HID capabilities",
        ));
    }
    let mut caps = HIDP_CAPS::default();
    let status = unsafe { HidP_GetCaps(preparsed, &mut caps) };
    struct Prepared(PHIDP_PREPARSED_DATA);
    impl Drop for Prepared {
        fn drop(&mut self) {
            unsafe {
                HidD_FreePreparsedData(self.0);
            }
        }
    }
    let _prepared = Prepared(preparsed);
    if status != HIDP_STATUS_SUCCESS {
        return Err(Issue::new(
            "HidP_GetCaps",
            Some(status as u32),
            "HID capabilities недоступны",
        ));
    }
    let fields = [
        ("usage", "Usage", caps.Usage),
        ("usage_page", "Usage page", caps.UsagePage),
        (
            "input_bytes",
            "Input report bytes",
            caps.InputReportByteLength,
        ),
        (
            "output_bytes",
            "Output report bytes",
            caps.OutputReportByteLength,
        ),
        (
            "feature_bytes",
            "Feature report bytes",
            caps.FeatureReportByteLength,
        ),
        (
            "collections",
            "Link collections",
            caps.NumberLinkCollectionNodes,
        ),
        (
            "input_buttons",
            "Input button capabilities",
            caps.NumberInputButtonCaps,
        ),
        (
            "input_values",
            "Input value capabilities",
            caps.NumberInputValueCaps,
        ),
        (
            "input_indices",
            "Input data indices",
            caps.NumberInputDataIndices,
        ),
        (
            "output_buttons",
            "Output button capabilities",
            caps.NumberOutputButtonCaps,
        ),
        (
            "output_values",
            "Output value capabilities",
            caps.NumberOutputValueCaps,
        ),
        (
            "output_indices",
            "Output data indices",
            caps.NumberOutputDataIndices,
        ),
        (
            "feature_buttons",
            "Feature button capabilities",
            caps.NumberFeatureButtonCaps,
        ),
        (
            "feature_values",
            "Feature value capabilities",
            caps.NumberFeatureValueCaps,
        ),
        (
            "feature_indices",
            "Feature data indices",
            caps.NumberFeatureDataIndices,
        ),
    ];
    let mut result: Vec<Field> = fields
        .into_iter()
        .map(|(id, name, value)| {
            Field::new(
                format!("hid.{id}"),
                name,
                "HID",
                value,
                "HidP_GetCaps (Windows cache)",
                false,
            )
        })
        .collect();
    result.push(Field::new("hid.report_descriptor.status","Сырой HID Report Descriptor","HID",
        "Через Windows hub API недоступен (recipient принудительно device). Смысловые HID capabilities получены из кэша Windows.",
        "USB_DESCRIPTOR_REQUEST / HidP_GetCaps",false));
    for (ty, name, buttons, values) in [
        (
            HidP_Input,
            "input",
            caps.NumberInputButtonCaps,
            caps.NumberInputValueCaps,
        ),
        (
            HidP_Output,
            "output",
            caps.NumberOutputButtonCaps,
            caps.NumberOutputValueCaps,
        ),
        (
            HidP_Feature,
            "feature",
            caps.NumberFeatureButtonCaps,
            caps.NumberFeatureValueCaps,
        ),
    ] {
        if buttons > 0 {
            let mut count = buttons.min(4096);
            let mut data = vec![HIDP_BUTTON_CAPS::default(); count as usize];
            let status =
                unsafe { HidP_GetButtonCaps(ty, data.as_mut_ptr(), &mut count, preparsed) };
            if status == HIDP_STATUS_SUCCESS && count as usize <= data.len() {
                for (i, c) in data.into_iter().take(count as usize).enumerate() {
                    let usage = unsafe {
                        let r = c.Anonymous.Range;
                        let one = c.Anonymous.NotRange;
                        let range = |enabled: bool, min: u16, max: u16, single: u16| {
                            if enabled {
                                format!("{min}..{max}")
                            } else {
                                single.to_string()
                            }
                        };
                        format!(
                            "usage={}, string={}, designator={}, data={}",
                            range(c.IsRange, r.UsageMin, r.UsageMax, one.Usage),
                            range(c.IsStringRange, r.StringMin, r.StringMax, one.StringIndex),
                            range(
                                c.IsDesignatorRange,
                                r.DesignatorMin,
                                r.DesignatorMax,
                                one.DesignatorIndex
                            ),
                            range(c.IsRange, r.DataIndexMin, r.DataIndexMax, one.DataIndex)
                        )
                    };
                    result.push(Field::new(format!("hid.{name}.button.{i}"),format!("{name} button capability {i}"),"HID",format!("page=0x{:04X}; report={}; bits=0x{:04X}; link={} ({:04X}:{:04X}); absolute={}; alias={}; reportCount={}; {usage}",c.UsagePage,c.ReportID,c.BitField,c.LinkCollection,c.LinkUsagePage,c.LinkUsage,c.IsAbsolute,c.IsAlias,c.ReportCount),"HidP_GetButtonCaps",false));
                }
            } else {
                result.push(Field::new(
                    format!("hid.{name}.buttons.status"),
                    "Button caps status",
                    "HID",
                    format!("0x{:08X}; requested={buttons}", status as u32),
                    "HidP_GetButtonCaps",
                    false,
                ));
            }
        }
        if values > 0 {
            let mut count = values.min(4096);
            let mut data = vec![HIDP_VALUE_CAPS::default(); count as usize];
            let status = unsafe { HidP_GetValueCaps(ty, data.as_mut_ptr(), &mut count, preparsed) };
            if status == HIDP_STATUS_SUCCESS && count as usize <= data.len() {
                for (i, c) in data.into_iter().take(count as usize).enumerate() {
                    let usage = unsafe {
                        let r = c.Anonymous.Range;
                        let one = c.Anonymous.NotRange;
                        let range = |enabled: bool, min: u16, max: u16, single: u16| {
                            if enabled {
                                format!("{min}..{max}")
                            } else {
                                single.to_string()
                            }
                        };
                        format!(
                            "usage={}, string={}, designator={}, data={}",
                            range(c.IsRange, r.UsageMin, r.UsageMax, one.Usage),
                            range(c.IsStringRange, r.StringMin, r.StringMax, one.StringIndex),
                            range(
                                c.IsDesignatorRange,
                                r.DesignatorMin,
                                r.DesignatorMax,
                                one.DesignatorIndex
                            ),
                            range(c.IsRange, r.DataIndexMin, r.DataIndexMax, one.DataIndex)
                        )
                    };
                    result.push(Field::new(format!("hid.{name}.value.{i}"),format!("{name} value capability {i}"),"HID",format!("page=0x{:04X}; report={}; bitSize={}; reportCount={}; logical={}..{}; physical={}..{}; units=0x{:08X}; exponent(raw)={}; absolute={}; hasNull={}; alias={}; link={} ({:04X}:{:04X}); bitField=0x{:04X}; {usage}",c.UsagePage,c.ReportID,c.BitSize,c.ReportCount,c.LogicalMin,c.LogicalMax,c.PhysicalMin,c.PhysicalMax,c.Units,c.UnitsExp,c.IsAbsolute,c.HasNull,c.IsAlias,c.LinkCollection,c.LinkUsagePage,c.LinkUsage,c.BitField),"HidP_GetValueCaps",false));
                }
            } else {
                result.push(Field::new(
                    format!("hid.{name}.values.status"),
                    "Value caps status",
                    "HID",
                    format!("0x{:08X}; requested={values}", status as u32),
                    "HidP_GetValueCaps",
                    false,
                ));
            }
        }
    }
    if caps.NumberLinkCollectionNodes > 0 {
        let mut count = u32::from(caps.NumberLinkCollectionNodes).min(4096);
        let mut data = vec![HIDP_LINK_COLLECTION_NODE::default(); count as usize];
        let status =
            unsafe { HidP_GetLinkCollectionNodes(data.as_mut_ptr(), &mut count, preparsed) };
        if status == HIDP_STATUS_SUCCESS && count as usize <= data.len() {
            for (i, c) in data.into_iter().take(count as usize).enumerate() {
                result.push(Field::new(format!("hid.collection.{i}"),format!("Link collection {i}"),"HID",format!("usage={:04X}:{:04X}; parent={}; children={}; firstChild={}; nextSibling={}; flags=0x{:08X}",c.LinkUsagePage,c.LinkUsage,c.Parent,c.NumberOfChildren,c.FirstChild,c.NextSibling,c._bitfield),"HidP_GetLinkCollectionNodes",false));
            }
        } else {
            result.push(Field::new(
                "hid.collections.status",
                "Link collection status",
                "HID",
                format!("0x{:08X}", status as u32),
                "HidP_GetLinkCollectionNodes",
                false,
            ));
        }
    }
    Ok(result)
}

pub fn legacy_properties(set: HDEVINFO, dev: &SP_DEVINFO_DATA) -> Vec<Field> {
    let mut out = Vec::new();
    let mut buffer = vec![0u8; 65536];
    for property in 0..SPDRP_MAXIMUM_PROPERTY {
        if matches!(property, 3 | 5 | 6) {
            continue;
        }
        let (mut kind, mut size) = (0, 0);
        let ok = unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                set,
                dev,
                property,
                &mut kind,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut size,
            )
        };
        if ok == 0 || size as usize > buffer.len() {
            continue;
        }
        let bytes = &buffer[..size as usize];
        let label = match property {
            0 => "Описание (реестр)",
            1 => "Hardware IDs (реестр)",
            2 => "Compatible IDs (реестр)",
            4 => "Служба (реестр)",
            7 => "Класс (реестр)",
            8 => "GUID класса (реестр)",
            9 => "Ключ драйвера",
            10 => "Config flags",
            11 => "Производитель (реестр)",
            12 => "Название (реестр)",
            13 => "Размещение (реестр)",
            14 => "Physical device object",
            15 => "Device capabilities",
            16 => "UI number",
            17 => "Upper filters",
            18 => "Lower filters",
            19 => "Bus type GUID",
            20 => "Legacy bus type",
            21 => "Bus number",
            22 => "Enumerator",
            23 => "Security descriptor",
            24 => "Security descriptor string",
            25 => "Device type",
            26 => "Exclusive access",
            27 => "Characteristics",
            28 => "Bus address",
            29 => "UI number description",
            30 => "Power data",
            31 => "Removal policy",
            32 => "Removal policy hardware default",
            33 => "Removal policy override",
            34 => "Install state",
            35 => "Location paths",
            36 => "Base container ID",
            _ => "Registry property",
        };
        let ty = match kind {
            1 | 2 => 18,
            7 => 8210,
            4 => 7,
            11 => 9,
            _ => 4099,
        };
        out.push(Field::new(
            format!("legacy.{property}"),
            label,
            "Windows · реестр",
            descriptors::property_value(ty, bytes),
            "SetupDiGetDeviceRegistryPropertyW",
            kind != 4,
        ));
        if property == SPDRP_DEVICE_POWER_DATA {
            out.extend(descriptors::power_fields(bytes));
        }
    }
    out
}

/// Follow the Windows OS-string support cache; do not probe devices which Windows
/// already rejected or has not recorded. No vendor control requests are issued.
fn ms_os_descriptor(handle: HANDLE, port: u32, raw: &[u8], d: &mut Device) {
    use windows_sys::Win32::System::Registry::*;
    let path: Vec<u16> = format!(
        "SYSTEM\\CurrentControlSet\\Control\\usbflags\\{:04X}{:04X}{:04X}\0",
        descriptors::u16_at(raw, 8).unwrap_or(0),
        descriptors::u16_at(raw, 10).unwrap_or(0),
        descriptors::u16_at(raw, 12).unwrap_or(0)
    )
    .encode_utf16()
    .collect();
    let name: Vec<u16> = "osvc\0".encode_utf16().collect();
    let mut bytes = [0u8; 256];
    let mut size = bytes.len() as u32;
    let rc = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            path.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_BINARY,
            null_mut(),
            bytes.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if rc != 0 || size < 2 || size as usize > bytes.len() {
        d.fields.push(Field::new(
            "usb.ms_os.status",
            "Microsoft OS descriptors",
            "USB · MS OS",
            format!("Кэш osvc недоступен (Win32 {rc}); дополнительный запрос 0xEE не отправлен"),
            "Windows usbflags/osvc",
            false,
        ));
        return;
    }
    d.fields.push(Field::new(
        "usb.ms_os.osvc",
        "Кэш Windows osvc",
        "USB · MS OS",
        hex(&bytes[..size as usize]),
        "Windows usbflags/osvc",
        false,
    ));
    if size != 2 || bytes[0] != 1 {
        d.fields.push(Field::new(
            "usb.ms_os.status",
            "Microsoft OS descriptors",
            "USB · MS OS",
            "Кэш Windows не подтверждает поддержку; запрос 0xEE пропущен",
            "Windows usbflags/osvc",
            false,
        ));
        return;
    }
    match standard_descriptor(handle, port, 3, 0xee, 0, 18) {
        Ok(b) => {
            let mut desc = Descriptor::new("Microsoft OS string", 0xee, 0, b);
            let signature: Vec<u8> = "MSFT100"
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();
            if desc.raw.len() == 18
                && desc.raw[0] == 18
                && desc.raw[1] == 3
                && desc.raw[2..16] == signature
            {
                d.fields.push(Field::new(
                    "usb.ms_os.vendor_code",
                    "MS OS vendor code",
                    "USB · MS OS",
                    desc.raw[16],
                    "MSFT100 string",
                    false,
                ));
                d.fields.push(Field::new("usb.ms_os.status","Microsoft OS descriptors","USB · MS OS","MSFT100 получен. Extended Compat ID/Properties требуют vendor-запроса, недоступного через hub IOCTL.","USB_DESCRIPTOR_REQUEST",false));
            } else {
                desc.complete = false;
                desc.notes
                    .push("Ответ не является MSFT100 OS string".into());
            }
            d.descriptors.push(desc);
        }
        Err(e) => d
            .issues
            .push(Issue::new("Microsoft OS string", e.code, &e.reason)),
    }
}
