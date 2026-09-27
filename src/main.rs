#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod app;
mod appearance;
fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() == 2 && args[0] == "--collect-worker" {
        if usb_doctor::scan_job::worker(std::path::Path::new(&args[1])).is_err() {
            std::process::exit(1);
        }
        return Ok(());
    }
    let _guard = match usb_doctor::single_instance::acquire() {
        Ok(Some(g)) => g,
        Ok(None) => return Ok(()),
        Err(e) => {
            rfd::MessageDialog::new().set_description(e).show();
            return Ok(());
        }
    };
    // Synthetic GUI data is available only in test builds, never in the portable release.
    let demo = cfg!(feature = "test-worker") && args.as_slice() == ["--demo"];
    if !args.is_empty() && !demo {
        rfd::MessageDialog::new()
            .set_title("USB Глаз")
            .set_description("Для отчётов используйте usb-glaz-cli --help.")
            .show();
        return Ok(());
    }
    let result = eframe::run_native(
        if demo {
            "USB Глаз · тестовые данные"
        } else {
            "USB Глаз"
        },
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_icon(eframe::egui::IconData {
                    rgba: include_bytes!("../assets/usb-eye-64.rgba").to_vec(),
                    width: 64,
                    height: 64,
                })
                .with_inner_size([1180.0, 620.0])
                .with_min_inner_size([860.0, 540.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| Ok(Box::new(app::Doctor::new(cc, demo)))),
    );
    if let Err(error) = &result {
        rfd::MessageDialog::new().set_title("USB Глаз").set_description(format!("Не удалось открыть интерфейс: {error}\nНужны Windows 10/11 x64. Проверьте видеодрайвер и поддержку OpenGL. CLI работает без графического интерфейса.")).show();
    }
    result
}
