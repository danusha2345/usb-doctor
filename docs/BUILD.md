# Сборка / Building

Нужны Rust 1.95+, цель `x86_64-pc-windows-gnu`, MinGW GCC и windres.
Requires Rust 1.95+, the `x86_64-pc-windows-gnu` target, MinGW GCC and windres.

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu --bin usb-doctor --locked
```

Результат: `target/x86_64-pc-windows-gnu/release/usb-doctor.exe`.
Для распространения файл переименовывается в `usb-glaz.exe`.
Output: `target/x86_64-pc-windows-gnu/release/usb-doctor.exe`.
The distributed executable is renamed to `usb-glaz.exe`.

## Проверки / Checks

```sh
cargo test --all-targets --features test-worker --locked
cargo clippy --all-targets --features test-worker --locked -- -D warnings
cargo clippy --target x86_64-pc-windows-gnu --all-targets --locked -- -D warnings
```

Feature `test-worker` используется только для тестов; релиз собирается без него.
The `test-worker` feature is for tests only; do not enable it for release builds.
