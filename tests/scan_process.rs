#![cfg(feature = "test-worker")]
use usb_doctor::scan_job::ScanJob;
fn fixture() -> std::path::PathBuf {
    std::env::var_os("USB_DOCTOR_TEST_WORKER")
        .map(Into::into)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_usb-doctor-test-worker").into())
}
fn result(job: &mut ScanJob) -> Result<usb_doctor::model::Snapshot, String> {
    let start = std::time::Instant::now();
    loop {
        if let Some(result) = job.poll() {
            return result;
        }
        assert!(start.elapsed() < std::time::Duration::from_secs(5));
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
#[test]
fn completed_worker_returns_snapshot() {
    let mut job = ScanJob::spawn(fixture(), "--ok").unwrap();
    assert_eq!(result(&mut job).unwrap().devices.len(), 4);
}
#[test]
fn cancel_waiting_worker_is_bounded() {
    let mut job = ScanJob::spawn(fixture(), "--wait").unwrap();
    job.cancel().unwrap();
    assert!(result(&mut job).unwrap_err().contains("остановлен"));
}
#[test]
fn malformed_output_is_reported() {
    let mut job = ScanJob::spawn(fixture(), "--bad").unwrap();
    assert!(result(&mut job).is_err());
}
#[test]
fn failing_process_is_reported() {
    let mut job = ScanJob::spawn(fixture(), "--fail").unwrap();
    assert!(result(&mut job).is_err());
}
