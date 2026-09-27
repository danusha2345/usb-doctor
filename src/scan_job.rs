//! Isolate driver calls in a child process. Cancelling never resets USB hardware.
use crate::model::Snapshot;
use std::{
    io::{Read, Seek, SeekFrom},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
pub struct ScanJob {
    child: Option<Child>,
    file: Option<tempfile::NamedTempFile>,
    started: Instant,
    cancelled: bool,
    #[cfg(windows)]
    _job: WindowsJob,
}
impl ScanJob {
    pub fn start() -> Result<Self, String> {
        Self::spawn(
            std::env::current_exe().map_err(|e| e.to_string())?,
            "--collect-worker",
        )
    }
    pub fn spawn(exe: std::path::PathBuf, arg: &str) -> Result<Self, String> {
        let file = tempfile::Builder::new()
            .prefix("usb-doctor-scan-")
            .tempfile()
            .map_err(|e| e.to_string())?;
        let mut cmd = Command::new(exe);
        cmd.arg(arg)
            .arg(file.path())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        #[cfg(windows)]
        let job = WindowsJob::new()?;
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        #[cfg(windows)]
        if let Err(e) = job.assign(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        // Keep the binding mutable on both platforms for shared cleanup paths.
        let _ = &mut child;
        Ok(Self {
            child: Some(child),
            file: Some(file),
            started: Instant::now(),
            cancelled: false,
            #[cfg(windows)]
            _job: job,
        })
    }
    pub fn cancel(&mut self) -> Result<(), String> {
        if let Some(child) = &mut self.child
            && child.try_wait().map_err(|e| e.to_string())?.is_none()
        {
            child
                .kill()
                .map_err(|e| format!("Не удалось остановить сбор: {e}"))?;
        }
        self.cancelled = true;
        Ok(())
    }
    pub fn poll(&mut self) -> Option<Result<Snapshot, String>> {
        if self.started.elapsed() > Duration::from_secs(90)
            && !self.cancelled
            && let Err(e) = self.cancel()
        {
            return Some(Err(e));
        }
        let status = match self.child.as_mut()?.try_wait() {
            Ok(None) => return None,
            Ok(Some(s)) => s,
            Err(e) => return Some(Err(e.to_string())),
        };
        self.child = None;
        if self.cancelled {
            return Some(Err("Сбор остановлен. Предыдущий снимок сохранён.".into()));
        }
        if !status.success() {
            return Some(Err(format!("Процесс сбора завершился с ошибкой: {status}")));
        }
        Some((|| {
            let file = self.file.as_mut().ok_or("Нет результата сбора")?;
            file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
            let mut bytes = vec![];
            file.take(32 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 32 * 1024 * 1024 {
                return Err("Снимок превышает 32 MiB".into());
            }
            serde_json::from_slice::<Result<Snapshot, String>>(&bytes)
                .map_err(|e| format!("Некорректный ответ сборщика: {e}"))?
        })())
    }
}
impl Drop for ScanJob {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let file = self.file.take();
            std::thread::spawn(move || {
                let _ = child.wait();
                drop(file);
            });
        }
    }
}
pub fn worker(path: &std::path::Path) -> Result<(), String> {
    let result = crate::collector::collect();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    serde_json::to_writer(file, &result).map_err(|e| e.to_string())
}

#[cfg(windows)]
struct WindowsJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl WindowsJob {
    fn new() -> Result<Self, String> {
        use windows_sys::Win32::{Foundation::*, System::JobObjects::*};
        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            return Err(format!("CreateJobObject: {}", unsafe { GetLastError() }));
        }
        let job = Self(handle);
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                (&info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
            )
        } == 0
        {
            return Err(format!("SetInformationJobObject: {}", unsafe {
                GetLastError()
            }));
        }
        Ok(job)
    }
    fn assign(&self, child: &Child) -> Result<(), String> {
        use std::os::windows::io::AsRawHandle;
        if unsafe {
            windows_sys::Win32::System::JobObjects::AssignProcessToJobObject(
                self.0,
                child.as_raw_handle(),
            )
        } == 0
        {
            return Err(format!("AssignProcessToJobObject: {}", unsafe {
                windows_sys::Win32::Foundation::GetLastError()
            }));
        }
        Ok(())
    }
}
#[cfg(windows)]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
