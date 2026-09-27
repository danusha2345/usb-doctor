# Сборка USB Глаз 0.6.0

Rust 1.95+, проверено на Rust/Cargo 1.98.1. Cargo.lock фиксирует зависимости.
Крупные build outputs — ~/storage/usb-doctor/target, вне checkout.
Для Windows GNU нужны MinGW GCC и windres (иконка и manifest).

```sh
export CARGO_TARGET_DIR="$HOME/storage/usb-doctor/target"
cargo test --all-targets --features test-worker --locked
cargo clippy --all-targets --features test-worker --locked -- -D warnings
cargo clippy --target x86_64-pc-windows-gnu --all-targets --locked -- -D warnings
cargo build --release --target x86_64-pc-windows-gnu --bin usb-doctor --bin usb-doctor-cli --locked
```

78 тестов Linux, 80 Windows. Релизные GUI/CLI собираются без test-worker.
Публикуемый GUI переименован в usb-glaz.exe; внутренние Cargo targets сохранены.
Тестовый worker и диагностический CLI в релиз не входят.
Артефакты: ~/storage/usb-doctor/artifacts/0.6.0-alpha/.
Windows: `C:\Users\Daniil\UsbDoctorLab\usb-glaz-06.exe`.
[Поведение, измерения и ограничения](RELEASE_06.md). Публикуемый файл: `usb-glaz.exe` без упаковки,
контрольные суммы и сведения о лицензиях. Тег: `v0.6.0-alpha`.

| Файл | Размер, байт | SHA256 |
|---|---:|---|
| usb-glaz.exe | 8560640 | `9f7d5a9c230e54ba54f474435ab50fb444cc181c98456b9a5428b6842ada9f13` |
| usb-glaz-cli.exe | 782336 | `9aff746dcf038cea26561b1426f4b5027d0baa01c7c024037ba65a11f94d799b` |

CLI остаётся внутренним инструментом диагностики и не публикуется в релизе.
