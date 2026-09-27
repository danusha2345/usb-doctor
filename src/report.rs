use crate::{diagnostics::assess, model::Snapshot};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::{self, Write},
    path::Path,
};

/// Allowlist, not a regex scrub: no names, PnP keys, paths, serials or free-form errors.
pub fn redacted(snapshot: &Snapshot) -> Value {
    json!({"schema_version":1,"app_version":env!("CARGO_PKG_VERSION"),"demo":snapshot.demo,
        "captured_unix_ms":snapshot.captured_unix_ms,"inventory_complete":snapshot.inventory_complete,
        "privacy":"Имена, пути и идентификаторы экземпляров исключены",
        "scan_errors":snapshot.issues.iter().map(|i|json!({"operation":i.operation,"code":i.code})).collect::<Vec<_>>(),
        "devices":snapshot.devices.iter().enumerate().map(|(i,d)|json!({
            "label":format!("Устройство {}",i+1),"kind":d.kind(),"vendor_id":d.vendor_id,"product_id":d.product_id,
            "device_class":d.device_class,"is_hub":d.is_hub,"port":d.port,"speed":d.speed.speed(),
            "speed_label":d.speed_label(),"speed_evidence":d.speed,"connection_status":d.connection_status,
            "windows_problem":d.windows_problem,"assessment":assess(d),
            "fields":d.fields.iter().filter(|f|!f.sensitive).collect::<Vec<_>>(),
            "descriptor_blocks":d.descriptors.iter().map(|x|json!({"kind":x.kind,"index":x.index,"language":x.language,"bytes":x.raw.len(),"complete":x.complete,"notes":x.notes})).collect::<Vec<_>>(),
            "query_errors":d.issues.iter().map(|x|json!({"operation":x.operation,"code":x.code})).collect::<Vec<_>>()
        })).collect::<Vec<_>>()})
}
pub fn html(snapshot: &Snapshot) -> String {
    selected_html(snapshot, &crate::fields::FieldSelection::all(), false)
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub fn save_new(snapshot: &Snapshot, path: &Path) -> io::Result<()> {
    let data = match path
        .extension()
        .and_then(|x| x.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => html(snapshot).into_bytes(),
        Some("json") => serde_json::to_vec_pretty(&redacted(snapshot))?,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Нужно расширение .json или .html",
            ));
        }
    };
    let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(&data)?;
    f.sync_all()
}

fn query_errors(issues: &[crate::model::Issue], include_sensitive: bool) -> Vec<Value> {
    issues
        .iter()
        .map(|i| {
            let mut v = json!({"operation":i.operation,"code":i.code});
            if include_sensitive {
                v["reason"] = json!(i.reason);
            }
            v
        })
        .collect()
}
fn descriptor_status(d: &crate::model::Device) -> Vec<Value> {
    d.descriptors.iter().map(|b|json!({"kind":b.kind,"index":b.index,"language":b.language,"bytes":b.raw.len(),"complete":b.complete})).collect()
}
pub fn selected(
    snapshot: &Snapshot,
    selection: &crate::fields::FieldSelection,
    include_sensitive: bool,
) -> Value {
    json!({"schema_version":2,"app_version":env!("CARGO_PKG_VERSION"),"demo":snapshot.demo,"captured_unix_ms":snapshot.captured_unix_ms,"inventory_complete":snapshot.inventory_complete,"includes_sensitive":include_sensitive,"scan_errors":query_errors(&snapshot.issues,include_sensitive),
        "devices":snapshot.devices.iter().enumerate().map(|(i,d)|json!({"label":format!("Устройство {}",i+1),"query_errors":query_errors(&d.issues,include_sensitive),"descriptor_blocks":descriptor_status(d),"fields":crate::fields::for_device(d).into_iter().filter(|f|selection.contains(&f.id)&&(include_sensitive||!f.sensitive)).collect::<Vec<_>>() })).collect::<Vec<_>>(),
        "topology":snapshot.topology.iter().enumerate().map(|(i,d)|json!({"label":format!("Узел {}",i+1),"query_errors":query_errors(&d.issues,include_sensitive),"descriptor_blocks":descriptor_status(d),"fields":crate::fields::for_device(d).into_iter().filter(|f|selection.contains(&f.id)&&(include_sensitive||!f.sensitive)).collect::<Vec<_>>() })).collect::<Vec<_>>()})
}
pub fn selected_html(
    snapshot: &Snapshot,
    selection: &crate::fields::FieldSelection,
    include_sensitive: bool,
) -> String {
    let mut out = String::from(
        "<!doctype html><html lang=\"ru\"><meta charset=\"utf-8\"><title>USB Глаз</title><style>body{font:13px system-ui;color:#243440;margin:12px;max-width:1400px}h1{font-size:20px;margin:8px 0}h2{font-size:15px;margin:12px 0 4px}table{border-collapse:collapse;width:100%}td{padding:3px 6px;border-bottom:1px solid #e4e8eb;vertical-align:top;overflow-wrap:anywhere}td:first-child{width:220px;color:#536470}tr:nth-child(even){background:#f6f8f9}</style><h1>USB Глаз · выбранные поля</h1>",
    );
    out.push_str(&format!(
        "<p>Снимок: {} · Перечисление: {} · Ошибок сбора: {}</p>",
        snapshot.captured_unix_ms,
        if snapshot.inventory_complete {
            "завершено"
        } else {
            "неполное"
        },
        snapshot.issues.len()
    ));
    for issue in &snapshot.issues {
        out.push_str(&error_html(issue, include_sensitive));
    }
    if snapshot.demo {
        out.push_str("<p>Демо: вымышленные сведения.</p>");
    }
    out.push_str(if include_sensitive {
        "<p>Отчёт включает идентификаторы.</p>"
    } else {
        "<p>Идентификаторы скрыты.</p>"
    });
    for (i, d) in snapshot
        .devices
        .iter()
        .chain(snapshot.topology.iter())
        .enumerate()
    {
        out.push_str(&format!(
            "<h2>Узел {}</h2><p>Недоступных запросов: {} · Неполных блоков: {}</p>",
            i + 1,
            d.issues.len(),
            d.descriptors.iter().filter(|b| !b.complete).count()
        ));
        for issue in &d.issues {
            out.push_str(&error_html(issue, include_sensitive));
        }
        for b in d.descriptors.iter().filter(|b| !b.complete) {
            out.push_str(&format!(
                "<p>Неполный дескриптор: {} #{}</p>",
                escape(&b.kind),
                b.index
            ));
        }
        out.push_str("<table>");
        let mut previous_group = String::new();
        for f in crate::fields::for_device(d)
            .into_iter()
            .filter(|f| selection.contains(&f.id) && (include_sensitive || !f.sensitive))
        {
            if previous_group != f.group {
                out.push_str(&format!(
                    "<tr><th colspan=\"2\">{}</th></tr>",
                    escape(&f.group)
                ));
                previous_group = f.group.clone();
            }
            out.push_str(&format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                escape(&f.label),
                escape(f.display_value())
            ));
        }
        out.push_str("</table>");
    }
    out.push_str("</html>");
    out
}
pub fn save_selected_new(
    snapshot: &Snapshot,
    path: &Path,
    selection: &crate::fields::FieldSelection,
    include_sensitive: bool,
) -> io::Result<()> {
    let data = match path
        .extension()
        .and_then(|x| x.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("html" | "htm") => selected_html(snapshot, selection, include_sensitive).into_bytes(),
        Some("json") => {
            serde_json::to_vec_pretty(&selected(snapshot, selection, include_sensitive))?
        }
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Нужно расширение .json или .html",
            ));
        }
    };
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&data)?;
    file.sync_all()
}

fn error_html(issue: &crate::model::Issue, include_sensitive: bool) -> String {
    format!(
        "<p>{}: код {}{}</p>",
        escape(&issue.operation),
        crate::fields::optional_number(issue.code),
        if include_sensitive {
            format!(" — {}", escape(&issue.reason))
        } else {
            String::new()
        }
    )
}
