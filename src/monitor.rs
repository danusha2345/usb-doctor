//! PnP callbacks only signal an atomic flag. No collection or UI work inside callback.
#[cfg(windows)]
mod platform {
    use std::{
        ffi::c_void,
        sync::atomic::{AtomicBool, Ordering},
    };
    use windows_sys::Win32::Devices::{
        DeviceAndDriverInstallation::*,
        Usb::{
            GUID_DEVINTERFACE_USB_DEVICE, GUID_DEVINTERFACE_USB_HOST_CONTROLLER,
            GUID_DEVINTERFACE_USB_HUB,
        },
    };
    pub struct Monitor {
        handles: Vec<HCMNOTIFICATION>,
        pending: Option<Box<Signal>>,
    }
    struct Signal {
        pending: AtomicBool,
        wake: Box<dyn Fn() + Send + Sync>,
    }
    unsafe extern "system" fn event(
        _: HCMNOTIFICATION,
        context: *const c_void,
        action: CM_NOTIFY_ACTION,
        _: *const CM_NOTIFY_EVENT_DATA,
        _: u32,
    ) -> u32 {
        if matches!(
            action,
            CM_NOTIFY_ACTION_DEVICEINTERFACEARRIVAL | CM_NOTIFY_ACTION_DEVICEINTERFACEREMOVAL
        ) && !context.is_null()
        {
            // Box address stays stable until unregister has waited for all callbacks.
            let signal = unsafe { &*context.cast::<Signal>() };
            if !signal.pending.swap(true, Ordering::AcqRel) {
                (signal.wake)();
            }
        }
        0
    }
    impl Monitor {
        pub fn new() -> Result<Self, String> {
            Self::with_wake(Box::new(|| {}))
        }
        pub fn with_wake(wake: Box<dyn Fn() + Send + Sync>) -> Result<Self, String> {
            let pending = Box::new(Signal {
                pending: AtomicBool::new(false),
                wake,
            });
            let mut filter = CM_NOTIFY_FILTER {
                cbSize: std::mem::size_of::<CM_NOTIFY_FILTER>() as u32,
                FilterType: CM_NOTIFY_FILTER_TYPE_DEVICEINTERFACE,
                ..Default::default()
            };
            let mut handles = Vec::new();
            for guid in [
                GUID_DEVINTERFACE_USB_DEVICE,
                GUID_DEVINTERFACE_USB_HUB,
                GUID_DEVINTERFACE_USB_HOST_CONTROLLER,
            ] {
                filter.u.DeviceInterface.ClassGuid = guid;
                let mut handle = std::ptr::null_mut();
                let rc = unsafe {
                    CM_Register_Notification(
                        &filter,
                        (&*pending as *const Signal).cast(),
                        Some(event),
                        &mut handle,
                    )
                };
                if rc != CR_SUCCESS {
                    let mut ok = true;
                    for h in handles {
                        ok &= unsafe { CM_Unregister_Notification(h) } == CR_SUCCESS;
                    }
                    if !ok {
                        let _ = Box::leak(pending);
                    }
                    return Err(format!("PnP notification: CONFIGRET={rc}"));
                }
                handles.push(handle);
            }
            Ok(Self {
                handles,
                pending: Some(pending),
            })
        }
        pub fn take_changed(&self) -> bool {
            self.pending
                .as_ref()
                .is_some_and(|p| p.pending.swap(false, Ordering::AcqRel))
        }
    }
    impl Drop for Monitor {
        fn drop(&mut self) {
            // Never called on the callback thread. On unexpected API failure retain
            // context to avoid a dangling pointer in a still-registered callback.
            let mut ok = true;
            for h in self.handles.drain(..) {
                ok &= unsafe { CM_Unregister_Notification(h) } == CR_SUCCESS;
            }
            if !ok && let Some(p) = self.pending.take() {
                let _ = Box::leak(p);
            }
        }
    }
}
#[cfg(not(windows))]
mod platform {
    pub struct Monitor;
    impl Monitor {
        pub fn with_wake(_: Box<dyn Fn() + Send + Sync>) -> Result<Self, String> {
            Self::new()
        }
        pub fn new() -> Result<Self, String> {
            Err("PnP notification доступен на Windows".into())
        }
        pub fn take_changed(&self) -> bool {
            false
        }
    }
}
pub use platform::Monitor;
