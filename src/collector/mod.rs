#[cfg(windows)]
mod windows;
use crate::model::Snapshot;
pub fn collect() -> Result<Snapshot, String> {
    #[cfg(windows)]
    {
        windows::collect()
    }
    #[cfg(not(windows))]
    {
        Err("Сбор USB-сведений доступен только на Windows. Для примеров запустите с --demo.".into())
    }
}

#[cfg(windows)]
mod enrich;

pub fn presence() -> Result<Snapshot, String> {
    #[cfg(windows)]
    {
        windows::presence()
    }
    #[cfg(not(windows))]
    {
        Err("PnP доступен только на Windows".into())
    }
}
