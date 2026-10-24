//! One profile-scoped Generic Credential; never enumerate the user's vault.
use zeroize::Zeroizing;

pub(crate) trait CredentialStore: Send {
    fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>, &'static str>;
    fn write(&self, bytes: &[u8]) -> Result<(), &'static str>;
    fn delete(&self) -> Result<(), &'static str>;
}

pub(crate) struct WindowsStore {
    target: String,
}
impl WindowsStore {
    pub fn for_profile(path: &std::path::Path) -> Self {
        use sha2::{Digest, Sha256};
        let normalized = path.to_string_lossy().replace('/', "\\").to_lowercase();
        Self {
            target: format!("Pencil/API/v1/{:x}", Sha256::digest(normalized.as_bytes())),
        }
    }
}

#[cfg(windows)]
impl CredentialStore for WindowsStore {
    fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>, &'static str> {
        use windows_sys::Win32::{
            Foundation::{GetLastError, ERROR_NOT_FOUND},
            Security::Credentials::*,
        };
        let target: Vec<u16> = self.target.encode_utf16().chain(Some(0)).collect();
        let mut ptr = std::ptr::null_mut();
        unsafe {
            if CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut ptr) == 0 {
                return if GetLastError() == ERROR_NOT_FOUND {
                    Ok(None)
                } else {
                    Err("api_secure_storage_unavailable")
                };
            }
            let record = &*ptr;
            let size = record.CredentialBlobSize as usize;
            let result = if size == 0
                || size > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize
                || record.CredentialBlob.is_null()
            {
                Err("api_secure_storage_invalid")
            } else {
                let bytes = std::slice::from_raw_parts_mut(record.CredentialBlob, size);
                let copy = Zeroizing::new(bytes.to_vec());
                use zeroize::Zeroize;
                bytes.zeroize();
                Ok(Some(copy))
            };
            CredFree(ptr.cast());
            result
        }
    }
    fn write(&self, bytes: &[u8]) -> Result<(), &'static str> {
        use windows_sys::Win32::Security::Credentials::*;
        if bytes.len() > CRED_MAX_CREDENTIAL_BLOB_SIZE as usize {
            return Err("api_secure_storage_too_large");
        }
        let mut target: Vec<u16> = self.target.encode_utf16().chain(Some(0)).collect();
        let mut user: Vec<u16> = "Pencil".encode_utf16().chain(Some(0)).collect();
        let mut record: CREDENTIALW = unsafe { std::mem::zeroed() };
        record.Type = CRED_TYPE_GENERIC;
        record.TargetName = target.as_mut_ptr();
        record.UserName = user.as_mut_ptr();
        record.Persist = CRED_PERSIST_LOCAL_MACHINE;
        record.CredentialBlobSize = bytes.len() as u32;
        record.CredentialBlob = bytes.as_ptr() as *mut u8;
        if unsafe { CredWriteW(&record, 0) } == 0 {
            Err("api_secure_storage_write_failed")
        } else {
            Ok(())
        }
    }
    fn delete(&self) -> Result<(), &'static str> {
        use windows_sys::Win32::{
            Foundation::{GetLastError, ERROR_NOT_FOUND},
            Security::Credentials::*,
        };
        let target: Vec<u16> = self.target.encode_utf16().chain(Some(0)).collect();
        if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } == 0
            && unsafe { GetLastError() } != ERROR_NOT_FOUND
        {
            Err("api_secure_storage_delete_failed")
        } else {
            Ok(())
        }
    }
}
#[cfg(not(windows))]
impl CredentialStore for WindowsStore {
    fn read(&self) -> Result<Option<Zeroizing<Vec<u8>>>, &'static str> {
        Err("api_secure_storage_unavailable")
    }
    fn write(&self, _: &[u8]) -> Result<(), &'static str> {
        Err("api_secure_storage_unavailable")
    }
    fn delete(&self) -> Result<(), &'static str> {
        Err("api_secure_storage_unavailable")
    }
}
