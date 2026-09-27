// Built only with --features test-worker; never shipped in the portable package.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = &args[2];
    match args[1].as_str() {
        "--ok" => {
            let file = std::fs::File::create(path).unwrap();
            serde_json::to_writer(file, &Ok::<_, String>(usb_doctor::demo::snapshot())).unwrap();
        }
        "--wait" => std::thread::sleep(std::time::Duration::from_secs(60)),
        "--bad" => std::fs::write(path, b"not-json").unwrap(),
        "--fail" => std::process::exit(7),
        _ => std::process::exit(2),
    }
}
