//! Conservative user-mode inventory. No vendor/class requests, resets or registry writes.
use crate::{
    model::{Device, Issue, Snapshot, SpeedEvidence},
    wire,
};
use std::{
    collections::HashMap,
    mem::{offset_of, size_of},
    ptr::{null, null_mut},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Devices::{DeviceAndDriverInstallation::*, Usb::*},
    Foundation::*,
    Storage::FileSystem::*,
    System::IO::DeviceIoControl,
};

// Checked against the SDK layout. Decoding stays byte-based, avoiding packed/bool references.
const _: () = assert!(offset_of!(USB_NODE_CONNECTION_INFORMATION_EX, ConnectionStatus) == 31);
const _: () = assert!(offset_of!(USB_NODE_CONNECTION_INFORMATION_EX, Speed) == 23);
const _: () = assert!(size_of::<USB_NODE_CONNECTION_INFORMATION_EX_V2>() == 16);
const LIMIT: usize = 4096;
struct InfoSet(HDEVINFO);
impl Drop for InfoSet {
    fn drop(&mut self) {
        unsafe {
            SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
#[derive(Clone)]
struct Node {
    id: String,
    name: String,
    source: String,
    service: Option<String>,
    problem: Option<u32>,
    devinst: u32,
    parent: Option<u32>,
    fields: Vec<crate::fields::Field>,
}
fn last_issue(op: &str) -> Issue {
    let code = unsafe { GetLastError() };
    Issue::new(
        op,
        Some(code),
        if code == ERROR_ACCESS_DENIED {
            "Отказ в доступе"
        } else {
            "Windows не вернула сведения"
        },
    )
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
fn info_data() -> SP_DEVINFO_DATA {
    SP_DEVINFO_DATA {
        cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
        ..Default::default()
    }
}
fn property(set: HDEVINFO, dev: &SP_DEVINFO_DATA, property: u32) -> Option<String> {
    let mut buffer = vec![0u16; LIMIT];
    let mut required = 0;
    let mut kind = 0;
    // Valid live HDEVINFO and initialized SP_DEVINFO_DATA; buffer writable and aligned.
    let ok = unsafe {
        SetupDiGetDeviceRegistryPropertyW(
            set,
            dev,
            property,
            &mut kind,
            buffer.as_mut_ptr().cast(),
            (buffer.len() * 2) as u32,
            &mut required,
        )
    };
    if ok == 0
        || kind != 1
        || required as usize > buffer.len() * 2
        || required < 2
        || required % 2 != 0
    {
        return None;
    }
    let units = &buffer[..required as usize / 2];
    let end = units.iter().position(|x| *x == 0)?;
    String::from_utf16(&units[..end])
        .ok()
        .filter(|s| !s.is_empty())
}
fn node_mode(set: HDEVINFO, dev: &SP_DEVINFO_DATA, details: bool) -> Node {
    let mut id = vec![0u16; LIMIT];
    let id = if unsafe {
        SetupDiGetDeviceInstanceIdW(set, dev, id.as_mut_ptr(), id.len() as u32, null_mut())
    } != 0
    {
        let end = id.iter().position(|x| *x == 0).unwrap_or(0);
        String::from_utf16_lossy(&id[..end])
    } else {
        format!("devinst:{}", dev.DevInst)
    };
    let (name, source) = if let Some(x) = property(set, dev, SPDRP_FRIENDLYNAME) {
        (x, "Windows FriendlyName")
    } else if let Some(x) = property(set, dev, SPDRP_DEVICEDESC) {
        (x, "Windows DeviceDesc")
    } else {
        ("USB-устройство".into(), "Имя недоступно")
    };
    let (mut status, mut problem) = (0, 0);
    let problem = if unsafe { CM_Get_DevNode_Status(&mut status, &mut problem, dev.DevInst, 0) }
        == CR_SUCCESS
    {
        Some(if status & DN_HAS_PROBLEM != 0 {
            problem
        } else {
            0
        })
    } else {
        None
    };
    let mut parent = 0;
    let parent = if unsafe { CM_Get_Parent(&mut parent, dev.DevInst, 0) } == CR_SUCCESS {
        Some(parent)
    } else {
        None
    };
    let mut fields = if details {
        super::enrich::properties(set, dev)
    } else {
        Vec::new()
    };
    if details {
        fields.extend(super::enrich::legacy_properties(set, dev));
    }
    if let Some(parent) = parent {
        let mut id = vec![0u16; LIMIT];
        if unsafe { CM_Get_Device_IDW(parent, id.as_mut_ptr(), id.len() as u32, 0) } == CR_SUCCESS {
            let end = id.iter().position(|x| *x == 0).unwrap_or(id.len());
            fields.push(crate::fields::Field::new(
                "node.parent_key",
                "Родительский узел",
                "Windows",
                String::from_utf16_lossy(&id[..end]),
                "CM_Get_Parent / CM_Get_Device_IDW",
                true,
            ));
        }
    }
    if details && let Some(port) = super::enrich::com_port(set, dev) {
        fields.push(port);
    }
    Node {
        devinst: dev.DevInst,
        parent,
        fields,
        id,
        name,
        source: source.into(),
        service: property(set, dev, SPDRP_SERVICE),
        problem,
    }
}
fn pnp_inventory(issues: &mut Vec<Issue>) -> HashMap<String, Vec<Node>> {
    let raw = unsafe {
        SetupDiGetClassDevsW(null(), null(), null_mut(), DIGCF_PRESENT | DIGCF_ALLCLASSES)
    };
    if raw == INVALID_HANDLE_VALUE as isize {
        issues.push(last_issue("SetupDiGetClassDevsW/USB"));
        return HashMap::new();
    }
    let set = InfoSet(raw);
    let mut entries = Vec::new();
    let mut finished = false;
    for index in 0..LIMIT as u32 {
        let mut data = info_data();
        if unsafe { SetupDiEnumDeviceInfo(set.0, index, &mut data) } == 0 {
            if unsafe { GetLastError() } != ERROR_NO_MORE_ITEMS {
                issues.push(last_issue("SetupDiEnumDeviceInfo"));
            }
            finished = true;
            break;
        }
        let n = node_mode(set.0, &data, false);
        let key =
            property(set.0, &data, SPDRP_DRIVER).unwrap_or_else(|| format!("instance:{}", n.id));
        entries.push((key, data, n));
    }
    if !finished {
        issues.push(Issue::new(
            "PnP inventory",
            None,
            "Достигнут предел числа узлов",
        ));
    }
    let index: HashMap<_, _> = entries.iter().map(|(_, _, n)| (n.devinst, n)).collect();
    let mut related = std::collections::HashSet::new();
    for (_, _, n) in &entries {
        let mut chain = Vec::new();
        let mut current = Some(n.devinst);
        let mut usb = false;
        while let Some(id) = current {
            if chain.contains(&id) || chain.len() > 32 {
                break;
            }
            chain.push(id);
            let Some(p) = index.get(&id) else {
                break;
            };
            usb |= p.id.to_ascii_uppercase().starts_with("USB\\");
            current = p.parent;
        }
        if usb {
            related.extend(chain);
        }
    }
    drop(index);
    let mut nodes: HashMap<String, Vec<Node>> = HashMap::new();
    for (key, data, mut n) in entries {
        if related.contains(&n.devinst) {
            n.fields.extend(super::enrich::properties(set.0, &data));
            n.fields
                .extend(super::enrich::legacy_properties(set.0, &data));
            if let Some(f) = super::enrich::com_port(set.0, &data) {
                n.fields.push(f);
            }
        }
        nodes.entry(key.to_ascii_lowercase()).or_default().push(n);
    }
    nodes
}
pub(super) fn ioctl(
    handle: HANDLE,
    code: u32,
    mut buffer: Vec<u8>,
    operation: &str,
) -> Result<Vec<u8>, Issue> {
    let mut returned = 0;
    let pointer = buffer.as_mut_ptr();
    let length = buffer.len() as u32;
    // FILE_ANY_ACCESS hub query, METHOD_BUFFERED; one bounded in/out allocation remains alive.
    let ok = unsafe {
        DeviceIoControl(
            handle,
            code,
            pointer.cast(),
            length,
            pointer.cast(),
            length,
            &mut returned,
            null_mut(),
        )
    };
    if ok == 0 {
        return Err(last_issue(operation));
    }
    if returned as usize > buffer.len() {
        return Err(Issue::new(operation, None, "Некорректная длина ответа"));
    }
    buffer.truncate(returned as usize);
    Ok(buffer)
}
pub(super) fn query_port(
    handle: HANDLE,
    port: u32,
    code: u32,
    len: usize,
    op: &str,
) -> Result<Vec<u8>, Issue> {
    let mut buffer = vec![0; len];
    buffer[..4].copy_from_slice(&port.to_le_bytes());
    if code == IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX_V2 {
        buffer[4..8].copy_from_slice(&16u32.to_le_bytes());
        buffer[8..12].copy_from_slice(&4u32.to_le_bytes()); // SupportedUsbProtocols.Usb300
    }
    ioctl(handle, code, buffer, op)
}
fn enumerate_hub(
    set: HDEVINFO,
    interface: &SP_DEVICE_INTERFACE_DATA,
) -> Result<(Vec<u16>, Node), Issue> {
    enumerate_interface(set, interface, true)
}
fn enumerate_interface(
    set: HDEVINFO,
    interface: &SP_DEVICE_INTERFACE_DATA,
    details: bool,
) -> Result<(Vec<u16>, Node), Issue> {
    let mut needed = 0;
    unsafe {
        SetupDiGetDeviceInterfaceDetailW(set, interface, null_mut(), 0, &mut needed, null_mut());
    }
    if !(8..=65536).contains(&needed) {
        return Err(last_issue("SetupDiGetDeviceInterfaceDetailW/size"));
    }
    // u64 storage ensures the SDK structure's alignment; DevicePath begins at byte 4 (not cbSize).
    let mut aligned = vec![0u64; (needed as usize).div_ceil(8)];
    let detail = aligned
        .as_mut_ptr()
        .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
    unsafe {
        (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
    }
    let mut dev = info_data();
    if unsafe {
        SetupDiGetDeviceInterfaceDetailW(set, interface, detail, needed, &mut needed, &mut dev)
    } == 0
    {
        return Err(last_issue("SetupDiGetDeviceInterfaceDetailW"));
    }
    if needed as usize > aligned.len() * 8 || needed < 6 {
        return Err(Issue::new("Interface path", None, "Некорректная длина"));
    }
    let bytes =
        unsafe { std::slice::from_raw_parts(aligned.as_ptr().cast::<u8>(), needed as usize) };
    let path = wire::utf16z(&bytes[4..])
        .ok_or_else(|| Issue::new("Interface path", None, "Некорректная строка UTF-16"))?;
    Ok((wide(&path), node_mode(set, &dev, details)))
}
fn collect_hub(
    path: &[u16],
    hub: &Node,
    pnp: &HashMap<String, Vec<Node>>,
    snapshot: &mut Snapshot,
    start: Instant,
    ports_seen: &mut usize,
) {
    let node_index: HashMap<u32, &Node> = pnp.values().flatten().map(|n| (n.devinst, n)).collect();
    // No read/write data access requested. IOCTLs used here require FILE_ANY_ACCESS.
    let raw = unsafe {
        CreateFileW(
            path.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        snapshot.issues.push(last_issue("CreateFileW/USB hub"));
        return;
    }
    let handle = Handle(raw);
    let mut hub_device = Device {
        fields: hub.fields.clone(),
        descriptors: Vec::new(),
        key: hub.id.clone(),
        name: hub.name.clone(),
        name_source: hub.source.clone(),
        path: vec!["Компьютер".into()],
        port: None,
        vendor_id: None,
        product_id: None,
        device_class: Some(9),
        is_hub: true,
        connection_status: None,
        windows_problem: hub.problem,
        service: hub.service.clone(),
        speed: SpeedEvidence::default(),
        issues: Vec::new(),
    };
    hub_device.fields.extend(super::enrich::hub(handle.0));
    let hub_index = snapshot.topology.len();
    snapshot.topology.push(hub_device);
    let bytes = match ioctl(
        handle.0,
        IOCTL_USB_GET_NODE_INFORMATION,
        vec![0; size_of::<USB_NODE_INFORMATION>()],
        "USB_GET_NODE_INFORMATION",
    ) {
        Ok(x) => x,
        Err(e) => {
            snapshot.issues.push(e);
            return;
        }
    };
    snapshot.topology[hub_index]
        .fields
        .push(crate::fields::Field::new(
            "hub.node_information",
            "Hub node information · raw",
            "Хаб",
            crate::fields::hex(&bytes),
            "USB_NODE_INFORMATION",
            false,
        ));
    let power_offset =
        offset_of!(USB_NODE_INFORMATION, u) + offset_of!(USB_HUB_INFORMATION, HubIsBusPowered);
    if let Some(power) = bytes.get(power_offset) {
        snapshot.topology[hub_index]
            .fields
            .push(crate::fields::Field::new(
                "hub.bus_powered",
                "Питание хаба от шины (сообщено Windows)",
                "Питание",
                *power != 0,
                "USB_HUB_INFORMATION",
                false,
            ));
    }
    let count_offset = offset_of!(USB_NODE_INFORMATION, u)
        + offset_of!(USB_HUB_INFORMATION, HubDescriptor)
        + offset_of!(USB_HUB_DESCRIPTOR, bNumberOfPorts);
    let Some(&count) = bytes.get(count_offset) else {
        snapshot.issues.push(Issue::new(
            "USB_GET_NODE_INFORMATION",
            None,
            "Усечённый ответ",
        ));
        return;
    };
    for port in 1..=u32::from(count) {
        *ports_seen += 1;
        if *ports_seen > 1024 || start.elapsed() > Duration::from_secs(60) {
            snapshot.issues.push(Issue::new("Scan limit",None,"Сбор ограничен по времени или числу портов; отдельный вызов драйвера может длиться дольше"));
            return;
        }
        let ex_bytes = match query_port(
            handle.0,
            port,
            IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX,
            4096,
            "EX",
        ) {
            Ok(x) => x,
            Err(e) => {
                snapshot.issues.push(e);
                continue;
            }
        };
        let Some(ex) = wire::connection_for_port(&ex_bytes, port) else {
            snapshot.issues.push(Issue::new(
                "EX",
                None,
                "Усечённый ответ или несовпадающий порт",
            ));
            continue;
        };
        snapshot.topology[hub_index]
            .fields
            .push(crate::fields::Field::new(
                format!("port.{port}.status"),
                format!("Порт {port}: состояние"),
                "Порты хаба",
                crate::fields::connection_status(ex.status),
                "EX.ConnectionStatus",
                false,
            ));
        if ex.status == NoDeviceConnected {
            continue;
        }
        let mut d = Device {
            fields: Vec::new(),
            descriptors: vec![crate::descriptors::Descriptor::new(
                "Connection EX",
                0,
                0,
                ex_bytes.clone(),
            )],
            key: format!("{}:port:{port}", hub.id),
            name: if ex.is_hub {
                "USB-хаб".into()
            } else {
                "USB-устройство".into()
            },
            name_source: "Имя недоступно".into(),
            path: vec!["Компьютер".into(), hub.name.clone()],
            port: Some(port),
            vendor_id: ex.vendor,
            product_id: ex.product,
            device_class: ex.class,
            is_hub: ex.is_hub,
            connection_status: Some(ex.status),
            windows_problem: None,
            service: None,
            speed: SpeedEvidence::default(),
            issues: Vec::new(),
        };
        if ex.status == DeviceConnected {
            d.fields.push(crate::fields::Field::new(
                "usb.address",
                "USB-адрес",
                "Основное",
                ex.address,
                "USB_NODE_CONNECTION_INFORMATION_EX.DeviceAddress",
                false,
            ));
            d.speed.ex_speed = Some(ex.speed);
            match query_port(
                handle.0,
                port,
                IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX_V2,
                16,
                "EX_V2",
            ) {
                Ok(b) => match wire::v2(&b, port) {
                    Some((protocols, flags)) => {
                        d.descriptors.push(crate::descriptors::Descriptor::new(
                            "Connection EX_V2",
                            0,
                            0,
                            b.clone(),
                        ));
                        d.speed.v2_flags = Some(flags);
                        d.speed.port_protocols = Some(protocols);
                    }
                    None => d
                        .issues
                        .push(Issue::new("EX_V2", None, "Некорректный ответ")),
                },
                Err(e) => d.issues.push(e),
            }
            match query_port(
                handle.0,
                port,
                IOCTL_USB_GET_NODE_CONNECTION_DRIVERKEY_NAME,
                4096,
                "DriverKey",
            ) {
                Ok(b) => {
                    if let Some(key) = wire::driver_key(&b, port) {
                        match pnp.get(&key.to_ascii_lowercase()) {
                            Some(nodes) if nodes.len() == 1 => {
                                let n = &nodes[0];
                                d.key = n.id.clone();
                                d.name = n.name.clone();
                                d.name_source = n.source.clone();
                                d.service = n.service.clone();
                                d.windows_problem = n.problem;
                                d.fields.extend(n.fields.clone());
                                let mut chain = Vec::new();
                                let mut parent = Some(n.devinst);
                                let mut seen = std::collections::HashSet::new();
                                while let Some(id) = parent {
                                    if !seen.insert(id) || seen.len() > 32 {
                                        break;
                                    }
                                    let Some(node) = node_index.get(&id).copied() else {
                                        break;
                                    };
                                    chain.push(node.name.clone());
                                    parent = node.parent;
                                }
                                chain.reverse();
                                if chain.len() > 1 {
                                    d.path = chain;
                                }
                                let mut children: Vec<_> = pnp
                                    .values()
                                    .flatten()
                                    .filter(|child| {
                                        let mut parent = child.parent;
                                        let mut guard = 0;
                                        while let Some(id) = parent {
                                            if id == n.devinst {
                                                return true;
                                            }
                                            guard += 1;
                                            if guard > 32 {
                                                return false;
                                            }
                                            parent =
                                                node_index.get(&id).copied().and_then(|x| x.parent);
                                        }
                                        false
                                    })
                                    .collect();
                                children.sort_by(|a, b| a.id.cmp(&b.id));
                                for (index, child) in children.into_iter().enumerate() {
                                    let identity = crate::fields::stable_id(&child.id);
                                    d.fields.push(crate::fields::Field::new(
                                        format!("function.{identity}.name"),
                                        format!("Функция {index}: название"),
                                        "Функции Windows",
                                        &child.name,
                                        "PnP descendants",
                                        true,
                                    ));
                                    for f in &child.fields {
                                        let mut f = f.clone();
                                        f.id = format!("function.{identity}.{}", f.id);
                                        f.label = format!("Функция {index}: {}", f.label);
                                        f.group = format!("Функция {index}");
                                        d.fields.push(f);
                                    }
                                }
                            }
                            _ => d.issues.push(Issue::new(
                                "PnP mapping",
                                None,
                                "Однозначное сопоставление Windows-узла недоступно",
                            )),
                        }
                    } else {
                        d.issues
                            .push(Issue::new("DriverKey", None, "Некорректная строка ответа"));
                    }
                }
                Err(e) => d.issues.push(e),
            }
        }
        if ex.status == DeviceConnected {
            super::enrich::device_descriptors(handle.0, port, &ex_bytes, &mut d);
            super::enrich::port(handle.0, port, &mut d);
            if ex_bytes.get(4) == Some(&18) && ex_bytes.get(5) == Some(&1) {
                for (key, label, offset, sensitive) in [
                    ("manufacturer", "Производитель USB", 18, false),
                    ("product", "Название USB", 19, false),
                    ("serial", "Серийный номер USB", 20, true),
                ] {
                    let id = format!("usb.{key}");
                    if !d.fields.iter().any(|f| f.id == id) {
                        let value = if ex_bytes.get(offset) == Some(&0) {
                            "Не задан в дескрипторе"
                        } else {
                            "Не получен; см. ошибки запросов"
                        };
                        d.fields.push(crate::fields::Field::new(
                            id,
                            label,
                            "Строки USB",
                            value,
                            "Device/String descriptor",
                            sensitive,
                        ));
                    }
                }
            }
        }
        // Detect observable unplug/re-enumeration between EX, V2 and PnP association.
        // This is a consistency check, not an atomic hardware snapshot guarantee.
        let consistent = query_port(
            handle.0,
            port,
            IOCTL_USB_GET_NODE_CONNECTION_INFORMATION_EX,
            4096,
            "EX/verify",
        )
        .ok()
        .filter(|b| wire::u32_at(b, 0) == Some(port))
        .and_then(|b| wire::connection(&b))
        .is_some_and(|after| after == ex);
        if !consistent {
            d.key = format!("{}:port:{port}", hub.id);
            d.invalidate_connection();
            snapshot.issues.push(Issue::new(
                "Connection consistency",
                None,
                "Подключение изменилось во время сбора или повторная проверка недоступна",
            ));
        }
        snapshot.devices.push(d);
    }
}
pub fn collect() -> Result<Snapshot, String> {
    let mut snapshot = Snapshot::empty(false);
    let start = Instant::now();
    let pnp = pnp_inventory(&mut snapshot.issues);
    let raw = unsafe {
        SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_USB_HUB,
            null(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        )
    };
    if raw == INVALID_HANDLE_VALUE as isize {
        return Err(format!(
            "Не удалось получить список USB-хабов (Windows error {}).",
            unsafe { GetLastError() }
        ));
    }
    let set = InfoSet(raw);
    let mut finished = false;
    let mut ports_seen = 0;
    for index in 0..128u32 {
        if start.elapsed() > Duration::from_secs(60) {
            break;
        }
        let mut interface = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        if unsafe {
            SetupDiEnumDeviceInterfaces(
                set.0,
                null(),
                &GUID_DEVINTERFACE_USB_HUB,
                index,
                &mut interface,
            )
        } == 0
        {
            if unsafe { GetLastError() } != ERROR_NO_MORE_ITEMS {
                snapshot
                    .issues
                    .push(last_issue("SetupDiEnumDeviceInterfaces"));
            }
            finished = true;
            break;
        }
        match enumerate_hub(set.0, &interface) {
            Ok((path, hub)) => {
                collect_hub(&path, &hub, &pnp, &mut snapshot, start, &mut ports_seen)
            }
            Err(e) => snapshot.issues.push(e),
        }
    }
    if !finished {
        snapshot.issues.push(Issue::new(
            "Hub inventory",
            None,
            "Сбор завершён по лимиту; часть хабов могла не попасть в снимок",
        ));
    }
    extra_interfaces(&pnp, &mut snapshot);
    snapshot.inventory_complete = snapshot.issues.is_empty();
    snapshot
        .devices
        .sort_by(|a, b| a.name.cmp(&b.name).then(a.key.cmp(&b.key)));
    for top in &mut snapshot.topology {
        if let Some(d) = snapshot.devices.iter().find(|d| d.key == top.key) {
            let extra = std::mem::take(&mut top.fields);
            *top = d.clone();
            top.fields.extend(extra);
        }
    }
    snapshot.captured_unix_ms = crate::model::now_ms();
    Ok(snapshot)
}

fn root_device_for(
    node: &Node,
    pnp: &HashMap<String, Vec<Node>>,
    snapshot: &Snapshot,
) -> Option<usize> {
    let mut current = Some(node.devinst);
    let mut seen = std::collections::HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id) || seen.len() > 32 {
            return None;
        }
        let n = if id == node.devinst {
            node
        } else {
            pnp.values().flatten().find(|x| x.devinst == id)?
        };
        if let Some(index) = snapshot
            .devices
            .iter()
            .position(|d| d.key.eq_ignore_ascii_case(&n.id))
        {
            return Some(index);
        }
        current = n.parent;
    }
    None
}
fn extra_interfaces(pnp: &HashMap<String, Vec<Node>>, snapshot: &mut Snapshot) {
    use windows_sys::Win32::{
        Devices::HumanInterfaceDevice::GUID_DEVINTERFACE_HID,
        System::Ioctl::{GUID_DEVINTERFACE_DISK, IOCTL_STORAGE_GET_DEVICE_NUMBER},
    };
    for (guid, kind) in [
        (GUID_DEVINTERFACE_USB_HOST_CONTROLLER, "controller"),
        (GUID_DEVINTERFACE_HID, "hid"),
        (GUID_DEVINTERFACE_DISK, "disk"),
    ] {
        let raw = unsafe {
            SetupDiGetClassDevsW(
                &guid,
                null(),
                null_mut(),
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        };
        if raw == INVALID_HANDLE_VALUE as isize {
            snapshot
                .issues
                .push(last_issue("Extra interface enumeration"));
            continue;
        }
        let set = InfoSet(raw);
        for index in 0..1024u32 {
            let mut interface = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            if unsafe { SetupDiEnumDeviceInterfaces(set.0, null(), &guid, index, &mut interface) }
                == 0
            {
                break;
            }
            let Ok((path, node)) = enumerate_hub(set.0, &interface) else {
                continue;
            };
            let root = root_device_for(&node, pnp, snapshot);
            if kind != "controller" && root.is_none() {
                continue;
            }
            let raw = unsafe {
                CreateFileW(
                    path.as_ptr(),
                    0,
                    FILE_SHARE_READ | FILE_SHARE_WRITE,
                    null(),
                    OPEN_EXISTING,
                    0,
                    null_mut(),
                )
            };
            if raw == INVALID_HANDLE_VALUE {
                if let Some(i) = root {
                    snapshot.devices[i]
                        .issues
                        .push(last_issue("Open function interface"));
                }
                continue;
            }
            let handle = Handle(raw);
            if kind == "controller" {
                let mut fields = node.fields.clone();
                fields.push(crate::fields::Field::new(
                    "node.kind",
                    "Вид узла",
                    "Основное",
                    "Host controller",
                    "USB host interface",
                    false,
                ));
                match ioctl(
                    handle.0,
                    IOCTL_USB_GET_ROOT_HUB_NAME,
                    vec![0; 4096],
                    "Root hub name",
                ) {
                    Ok(b) => {
                        if let Some(name) = b.get(4..).and_then(wire::utf16z) {
                            fields.push(crate::fields::Field::new(
                                "controller.root_hub",
                                "Корневой хаб",
                                "Контроллер",
                                name,
                                "ROOT_HUB_NAME",
                                true,
                            ));
                        }
                    }
                    Err(e) => snapshot.issues.push(e),
                }
                snapshot.topology.push(Device {
                    fields,
                    descriptors: Vec::new(),
                    key: node.id,
                    name: node.name,
                    name_source: node.source,
                    path: vec!["Компьютер".into()],
                    port: None,
                    vendor_id: None,
                    product_id: None,
                    device_class: None,
                    is_hub: false,
                    connection_status: None,
                    windows_problem: node.problem,
                    service: node.service,
                    speed: SpeedEvidence::default(),
                    issues: Vec::new(),
                });
            } else if let Some(i) = root {
                if kind == "hid" {
                    match super::enrich::hid_caps(handle.0) {
                        Ok(fields) => {
                            for mut field in fields {
                                field.id = format!(
                                    "interface.{}.{}",
                                    crate::fields::stable_id(&String::from_utf16_lossy(&path)),
                                    field.id
                                );
                                field.label = format!("HID {index}: {}", field.label);
                                snapshot.devices[i].fields.push(field);
                            }
                        }
                        Err(e) => snapshot.devices[i].issues.push(e),
                    }
                } else {
                    match ioctl(
                        handle.0,
                        IOCTL_STORAGE_GET_DEVICE_NUMBER,
                        vec![0; 12],
                        "Storage device number",
                    ) {
                        Ok(b) => {
                            if let Some(number) = wire::u32_at(&b, 4) {
                                snapshot.devices[i].fields.push(crate::fields::Field::new(
                                    format!("disk.{}.number", crate::fields::stable_id(&node.id)),
                                    "PhysicalDrive number",
                                    "Накопитель",
                                    number,
                                    "STORAGE_DEVICE_NUMBER",
                                    false,
                                ));
                                let letters = drive_letters(number);
                                if !letters.is_empty() {
                                    snapshot.devices[i].fields.push(crate::fields::Field::new(
                                        format!(
                                            "disk.{}.letters",
                                            crate::fields::stable_id(&node.id)
                                        ),
                                        "Буквы томов",
                                        "Накопитель",
                                        letters.join(", "),
                                        "Volume disk extents",
                                        false,
                                    ));
                                }
                            }
                        }
                        Err(e) => snapshot.devices[i].issues.push(e),
                    }
                }
            }
        }
    }
}
fn drive_letters(disk_number: u32) -> Vec<String> {
    let mut result = Vec::new();
    for letter in b'A'..=b'Z' {
        let path = wide(&format!("\\\\.\\{}:", letter as char));
        let raw = unsafe {
            CreateFileW(
                path.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                0,
                null_mut(),
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            continue;
        }
        let handle = Handle(raw);
        if let Ok(b) = ioctl(
            handle.0,
            IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS,
            vec![0; 65536],
            "Volume extents",
        ) {
            let count = wire::u32_at(&b, 0).unwrap_or(0) as usize;
            if count > 2048 || 8usize.saturating_add(count.saturating_mul(24)) > b.len() {
                continue;
            }
            if (0..count).any(|i| wire::u32_at(&b, 8 + i * 24) == Some(disk_number)) {
                result.push(format!("{}:", letter as char));
            }
        }
    }
    result
}

/// PnP/cache-only presence pass. No hub handles, IOCTLs, descriptors or disk I/O.
pub fn presence() -> Result<Snapshot, String> {
    let mut snapshot = Snapshot::empty(false);
    let mut all = std::collections::BTreeMap::<String, Device>::new();
    for (guid, hub, controller) in [
        (GUID_DEVINTERFACE_USB_DEVICE, false, false),
        (GUID_DEVINTERFACE_USB_HUB, true, false),
        (GUID_DEVINTERFACE_USB_HOST_CONTROLLER, false, true),
    ] {
        let raw = unsafe {
            SetupDiGetClassDevsW(
                &guid,
                null(),
                null_mut(),
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        };
        if raw == INVALID_HANDLE_VALUE as isize {
            snapshot.issues.push(last_issue("Presence interface list"));
            continue;
        }
        let set = InfoSet(raw);
        let mut finished = false;
        for index in 0..1024u32 {
            let mut interface = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            if unsafe { SetupDiEnumDeviceInterfaces(set.0, null(), &guid, index, &mut interface) }
                == 0
            {
                if unsafe { GetLastError() } != ERROR_NO_MORE_ITEMS {
                    snapshot.issues.push(last_issue("Presence enumerate"));
                }
                finished = true;
                break;
            }
            match enumerate_interface(set.0, &interface, false) {
                Ok((_, n)) => {
                    let mut d = Device {
                        fields: n.fields,
                        descriptors: vec![],
                        key: n.id.clone(),
                        name: n.name,
                        name_source: n.source,
                        path: vec!["Компьютер".into()],
                        port: None,
                        vendor_id: None,
                        product_id: None,
                        device_class: hub.then_some(9),
                        is_hub: hub,
                        connection_status: Some(1),
                        windows_problem: n.problem,
                        service: n.service,
                        speed: SpeedEvidence::default(),
                        issues: vec![],
                    };
                    let id = n.id.to_ascii_uppercase();
                    // This is a documented USB device ID pattern, not a parsed interface path.
                    let number = |prefix: &str| {
                        id.split(prefix)
                            .nth(1)
                            .and_then(|v| v.get(..4))
                            .and_then(|v| u16::from_str_radix(v, 16).ok())
                    };
                    d.vendor_id = number("VID_");
                    d.product_id = number("PID_");
                    d.fields.push(crate::fields::Field::new(
                        "live.details_pending",
                        "Подробности",
                        "Windows",
                        "Ожидают уточнения",
                        "PnP cache",
                        false,
                    ));
                    if controller {
                        d.fields.push(crate::fields::Field::new(
                            "node.kind",
                            "Вид узла",
                            "Основное",
                            "Host controller",
                            "USB host interface",
                            false,
                        ));
                    }
                    all.insert(n.id, d);
                }
                Err(e) => snapshot.issues.push(e),
            }
        }
        if !finished {
            snapshot.issues.push(Issue::new(
                "Presence limit",
                None,
                "Достигнут предел интерфейсов",
            ));
        }
    }
    for (_, mut d) in all {
        if d.kind() == "USB-контроллер" || d.key.to_ascii_uppercase().contains("ROOT_HUB")
        {
            d.connection_status = None;
            snapshot.topology.push(d);
        } else {
            if d.is_hub {
                snapshot.topology.push(d.clone());
            }
            snapshot.devices.push(d);
        }
    }
    snapshot.inventory_complete = snapshot.issues.is_empty();
    snapshot.captured_unix_ms = crate::model::now_ms();
    Ok(snapshot)
}
