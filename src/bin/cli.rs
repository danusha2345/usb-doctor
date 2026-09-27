use std::path::PathBuf;
fn main() {
    if let Err(e) = run() {
        eprintln!("USB Глаз: {e}");
        std::process::exit(1)
    }
}
fn run() -> Result<(), String> {
    let mut demo = false;
    let mut raw = false;
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--benchmark" => {
                let mut quick_ms = vec![];
                let mut fast = None;
                for _ in 0..5 {
                    let start = std::time::Instant::now();
                    fast = Some(usb_doctor::collector::presence()?);
                    quick_ms.push(start.elapsed().as_secs_f64() * 1000.0);
                }
                let start = std::time::Instant::now();
                let full = usb_doctor::collector::collect()?;
                let full_ms = start.elapsed().as_secs_f64() * 1000.0;
                let fast = fast.unwrap();
                let keys = |s: &usb_doctor::model::Snapshot| {
                    let mut k: Vec<_> = s.devices.iter().map(|d| d.key.clone()).collect();
                    k.sort();
                    k
                };
                println!(
                    "{}",
                    serde_json::json!({"quick_ms":quick_ms,"full_ms":full_ms,"quick_devices":fast.devices.len(),"full_devices":full.devices.len(),"quick_topology":fast.topology.len(),"full_topology":full.topology.len(),"presence_keys_match":keys(&fast)==keys(&full),"quick_complete":fast.inventory_complete,"full_complete":full.inventory_complete})
                );
                return Ok(());
            }
            "--demo" => demo = true,
            "--raw" => raw = true,
            "--output" => {
                output = Some(PathBuf::from(
                    args.next().ok_or("После --output нужен путь")?,
                ))
            }
            "--help" | "-h" => {
                println!(
                    "usb-glaz-cli [--demo] [--output report.json|report.html]\nБез --demo собирает сведения Windows. По умолчанию отчёт обезличен. --raw --output full.json сохраняет полный снимок с идентификаторами. Существующие файлы не заменяются."
                );
                return Ok(());
            }
            "--version" => {
                println!("USB Глаз CLI {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => return Err(format!("Неизвестный аргумент: {arg}")),
        }
    }
    if raw
        && output
            .as_ref()
            .is_none_or(|p| p.extension().and_then(|x| x.to_str()) != Some("json"))
    {
        return Err(
            "--raw требует --output file.json: полный снимок не выводится в консоль".into(),
        );
    }
    let snapshot = if demo {
        usb_doctor::demo::snapshot()
    } else {
        usb_doctor::collector::collect()?
    };
    if raw {
        let data = serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?;
        if let Some(path) = output {
            use std::io::Write;
            if path.extension().and_then(|x| x.to_str()) != Some("json") {
                return Err("--raw требует .json".into());
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            file.write_all(&data).map_err(|e| e.to_string())?;
        } else {
            return Err(
                "--raw требует --output: полный снимок с идентификаторами не выводится в консоль"
                    .into(),
            );
        }
        return Ok(());
    }
    if let Some(path) = output {
        usb_doctor::report::save_new(&snapshot, &path).map_err(|e| e.to_string())?;
        println!("Отчёт сохранён: {}", path.display());
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(&usb_doctor::report::redacted(&snapshot))
                .map_err(|e| e.to_string())?
        )
    }
    Ok(())
}
