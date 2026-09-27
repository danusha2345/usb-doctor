//! Windows session-local guard, also shared by differently named portable EXEs.
#[cfg(windows)]
mod platform {
    use windows_sys::Win32::{Foundation::*, System::Threading::*, UI::WindowsAndMessaging::*};
    pub struct Guard(HANDLE);
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    pub fn acquire() -> Result<Option<Guard>, String> {
        let name: Vec<u16> = "Local\\UsbDoctor.Inventory.Gui.1\0"
            .encode_utf16()
            .collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "Не удалось проверить запущенный экземпляр: {}",
                unsafe { GetLastError() }
            ));
        }
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            unsafe {
                CloseHandle(handle);
            }
            for caption in ["USB Глаз\0", "USB Eye\0"] {
                let title: Vec<u16> = caption.encode_utf16().collect();
                let hwnd = unsafe { FindWindowW(std::ptr::null(), title.as_ptr()) };
                if !hwnd.is_null() {
                    unsafe {
                        ShowWindowAsync(hwnd, SW_RESTORE);
                        SetForegroundWindow(hwnd);
                    }
                }
            }
            return Ok(None);
        }
        Ok(Some(Guard(handle)))
    }
}
#[cfg(not(windows))]
mod platform {
    pub struct Guard;
    pub fn acquire() -> Result<Option<Guard>, String> {
        Ok(Some(Guard))
    }
}
pub use platform::*;
