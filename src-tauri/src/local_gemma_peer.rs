//! Windows executable/process pinning for local broker admission.
use std::{
    fs::{File, OpenOptions},
    io::Read,
    os::windows::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, WAIT_TIMEOUT},
    System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, WaitForSingleObject,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE,
    },
};
struct ImageInner {
    path: PathBuf,
    _file: File,
}
#[derive(Clone)]
pub struct ImagePin(Arc<ImageInner>);
pub struct ProcessPin {
    handle: isize,
    pid: u32,
    _image: ImagePin,
}
impl Drop for ProcessPin {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle as HANDLE);
        }
    }
}
impl ImagePin {
    /// Reads only the explicitly named executable. Hold a deny-write/delete file
    /// handle so the pin does not silently follow later on-disk replacement.
    pub fn verify(path: &Path, sha256: &str) -> Result<Self, String> {
        if !path.is_absolute()
            || sha256.len() != 64
            || !sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("gemma_image_config".into());
        }
        let path = std::fs::canonicalize(path).map_err(|_| "gemma_image_path")?;
        let mut file = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .map_err(|_| "gemma_image_open")?;
        if file.metadata().map_err(|_| "gemma_image_metadata")?.len() > 128 * 1024 * 1024 {
            return Err("gemma_image_limit".into());
        }
        let mut bytes = Vec::new();
        file.by_ref()
            .take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "gemma_image_read")?;
        if bytes.len() > 128 * 1024 * 1024
            || crate::instant_selection::sha256_hex(&bytes) != sha256.to_ascii_lowercase()
        {
            return Err("gemma_image_hash".into());
        }
        Ok(Self(Arc::new(ImageInner { path, _file: file })))
    }
    pub fn process(&self, pid: u32) -> Result<ProcessPin, String> {
        let handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SYNCHRONIZE,
                0,
                pid,
            )
        };
        if handle.is_null() {
            return Err("gemma_peer_open".into());
        }
        let pin = ProcessPin {
            handle: handle as isize,
            pid,
            _image: self.clone(),
        };
        let mut path = vec![0u16; 32768];
        let mut size = path.len() as u32;
        if unsafe { QueryFullProcessImageNameW(handle, 0, path.as_mut_ptr(), &mut size) } == 0 {
            return Err("gemma_peer_path".into());
        }
        let actual = std::fs::canonicalize(
            String::from_utf16(&path[..size as usize]).map_err(|_| "gemma_peer_path")?,
        )
        .map_err(|_| "gemma_peer_path")?;
        if actual.to_string_lossy().to_lowercase() != self.0.path.to_string_lossy().to_lowercase()
            || !pin.alive()
        {
            return Err("gemma_peer_image".into());
        }
        Ok(pin)
    }
}
impl ProcessPin {
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn alive(&self) -> bool {
        unsafe { WaitForSingleObject(self.handle as HANDLE, 0) == WAIT_TIMEOUT }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pin_checks_hash_and_keeps_live_process() {
        let path = std::env::current_exe().unwrap();
        let hash = crate::instant_selection::sha256_hex(&std::fs::read(&path).unwrap());
        assert!(ImagePin::verify(&path, &"0".repeat(64)).is_err());
        let image = ImagePin::verify(&path, &hash).unwrap();
        let process = image.process(std::process::id()).unwrap();
        assert!(process.alive());
        assert_eq!(process.pid(), std::process::id());
        assert!(image.process(0).is_err());
    }
}
