# Сборка USB Глаз 0.6.1

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

84 теста Linux, 86 Windows. Релизные GUI/CLI собираются без test-worker.
Публикуемый GUI переименован в usb-glaz.exe; внутренние Cargo targets сохранены.
Тестовый worker и диагностический CLI в релиз не входят.
Артефакты: ~/storage/usb-doctor/artifacts/0.6.1-alpha/.
Windows: `C:\Users\Daniil\UsbDoctorLab\usb-glaz-061.exe`.
[Языки и проверки](RELEASE_061.md); [измерения и ограничения](RELEASE_06.md). Публикуемый файл: `usb-glaz.exe` без упаковки,
контрольные суммы и сведения о лицензиях. Тег: `v0.6.1-alpha`.

| Файл | Размер, байт | SHA256 |
|---|---:|---|
| usb-glaz.exe | 8667648 | `99bef686f511c8405d2195f675d4bb36d2ed64cea2c3bd82161a8c22d6e94d93` |

CLI остаётся внутренним инструментом диагностики и не публикуется в релизе.
