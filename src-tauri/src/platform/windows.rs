//! Windows platform implementation (P2.4).
//!
//! Data lives under the machine-wide `C:\ProgramData\Vidya`, so any Windows
//! account on the office computer opens the same school (the installer grants
//! the Users group modify rights in P10.1). The 32-byte database key and other
//! secrets are protected with DPAPI (`CryptProtectData`) using the local-machine
//! scope plus a fixed entropy value, never written to disk in plain form.
//!
//! This file is compiled and tested only on Windows (CI + the Windows VM); it is
//! excluded from the macOS build by `#[cfg(target_os = "windows")]` in `mod.rs`.

use std::ffi::{c_void, OsString};
use std::fs;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    BCryptGenRandom, CryptProtectData, CryptUnprotectData, BCRYPT_USE_SYSTEM_PREFERRED_RNG,
    CRYPTPROTECT_LOCAL_MACHINE, CRYPT_INTEGER_BLOB,
};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath, KF_FLAG_DEFAULT};
use zeroize::Zeroizing;

use super::{MountedVolume, NetworkDiagnostics, Platform, PlatformError, VolumeInfo};

/// Fixed DPAPI entropy for the database key blob.
const DB_KEY_ENTROPY: &[u8] = b"vidya-db-key-v1";
/// Entropy prefix for named secret blobs (combined with the secret name).
const SECRET_ENTROPY_PREFIX: &[u8] = b"vidya-secret-v1:";

pub struct WindowsPlatform;

impl WindowsPlatform {
    /// `C:\ProgramData\Vidya` via `SHGetKnownFolderPath` (never a hard-coded `C:`).
    fn program_data_vidya() -> Result<PathBuf, PlatformError> {
        // SAFETY: FOLDERID_ProgramData is a valid known-folder id; the returned
        // PWSTR is owned by the shell and freed with CoTaskMemFree below.
        let pwstr = unsafe { SHGetKnownFolderPath(&FOLDERID_ProgramData, KF_FLAG_DEFAULT, None) }
            .map_err(|e| PlatformError::Os(format!("could not locate ProgramData: {e}")))?;
        let mut len = 0usize;
        // SAFETY: pwstr points to a null-terminated wide string.
        while unsafe { *pwstr.0.add(len) } != 0 {
            len += 1;
        }
        let wide = unsafe { std::slice::from_raw_parts(pwstr.0, len) };
        let path = PathBuf::from(OsString::from_wide(wide));
        unsafe { CoTaskMemFree(Some(pwstr.0 as *const c_void)) };
        Ok(path.join("Vidya"))
    }

    fn ensure_dir(dir: &Path) -> Result<(), PlatformError> {
        fs::create_dir_all(dir)?;
        Ok(())
    }

    fn secrets_dir(&self) -> Result<PathBuf, PlatformError> {
        let dir = self.data_dir()?.join("secrets");
        Self::ensure_dir(&dir)?;
        Ok(dir)
    }

    fn secret_entropy(name: &str) -> Vec<u8> {
        let mut entropy = SECRET_ENTROPY_PREFIX.to_vec();
        entropy.extend_from_slice(name.as_bytes());
        entropy
    }
}

/// DPAPI-protect `plain` with `entropy` and the local-machine flag.
///
/// Exposed for the `windows_dpapi` integration test (CI only).
pub fn dpapi_protect(plain: &[u8], entropy: &[u8]) -> Result<Vec<u8>, PlatformError> {
    let data_in = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let entropy_blob = CRYPT_INTEGER_BLOB {
        cbData: entropy.len() as u32,
        pbData: entropy.as_ptr() as *mut u8,
    };
    let mut data_out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: all blobs point to valid memory for the duration of the call; the
    // output buffer is freed with LocalFree after we copy it out.
    unsafe {
        CryptProtectData(
            &data_in,
            PCWSTR::null(),
            Some(&entropy_blob),
            None,
            None,
            CRYPTPROTECT_LOCAL_MACHINE,
            &mut data_out,
        )
    }
    .map_err(|e| PlatformError::Os(format!("DPAPI protect failed: {e}")))?;
    let bytes = unsafe { std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize) }.to_vec();
    unsafe { LocalFree(Some(HLOCAL(data_out.pbData as *mut c_void))) };
    Ok(bytes)
}

/// DPAPI-unprotect `blob` with the same `entropy` used to protect it.
///
/// Exposed for the `windows_dpapi` integration test (CI only).
pub fn dpapi_unprotect(blob: &[u8], entropy: &[u8]) -> Result<Zeroizing<Vec<u8>>, PlatformError> {
    let data_in = CRYPT_INTEGER_BLOB {
        cbData: blob.len() as u32,
        pbData: blob.as_ptr() as *mut u8,
    };
    let entropy_blob = CRYPT_INTEGER_BLOB {
        cbData: entropy.len() as u32,
        pbData: entropy.as_ptr() as *mut u8,
    };
    let mut data_out = CRYPT_INTEGER_BLOB::default();
    // SAFETY: as in dpapi_protect.
    unsafe { CryptUnprotectData(&data_in, None, Some(&entropy_blob), None, None, 0, &mut data_out) }
        .map_err(|_| PlatformError::Os("DPAPI unprotect failed (wrong machine or damaged)".into()))?;
    let bytes = Zeroizing::new(
        unsafe { std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize) }.to_vec(),
    );
    unsafe { LocalFree(Some(HLOCAL(data_out.pbData as *mut c_void))) };
    Ok(bytes)
}

/// Fill `buf` with cryptographically secure OS random bytes.
fn os_random(buf: &mut [u8]) -> Result<(), PlatformError> {
    // SAFETY: buf is a valid, writable slice.
    let status = unsafe { BCryptGenRandom(None, buf, BCRYPT_USE_SYSTEM_PREFERRED_RNG) };
    if status.0 == 0 {
        Ok(())
    } else {
        Err(PlatformError::Os(format!(
            "secure random failed: {:#x}",
            status.0
        )))
    }
}

/// Write `bytes` to `path` atomically (`.partial` → flush → rename).
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PlatformError> {
    use std::io::Write;
    let partial = path.with_extension("bin.partial");
    let mut file = fs::File::create(&partial)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&partial, path)?;
    Ok(())
}

impl Platform for WindowsPlatform {
    fn name(&self) -> &'static str {
        "windows"
    }

    fn data_dir(&self) -> Result<PathBuf, PlatformError> {
        let dir = Self::program_data_vidya()?.join("data");
        Self::ensure_dir(&dir)?;
        Ok(dir)
    }

    fn backups_dir(&self) -> Result<PathBuf, PlatformError> {
        let dir = Self::program_data_vidya()?.join("backups");
        Self::ensure_dir(&dir)?;
        Ok(dir)
    }

    fn load_or_create_db_key(&self) -> Result<Zeroizing<[u8; 32]>, PlatformError> {
        let key_path = self.data_dir()?.join("key.bin");
        if key_path.exists() {
            let blob = fs::read(&key_path)?;
            let plain = dpapi_unprotect(&blob, DB_KEY_ENTROPY)?;
            let key: [u8; 32] = plain
                .as_slice()
                .try_into()
                .map_err(|_| PlatformError::Os("stored database key has the wrong length".into()))?;
            Ok(Zeroizing::new(key))
        } else {
            let mut key = Zeroizing::new([0u8; 32]);
            os_random(&mut key[..])?;
            let blob = dpapi_protect(key.as_ref(), DB_KEY_ENTROPY)?;
            write_atomic(&key_path, &blob)?;
            Ok(key)
        }
    }

    fn store_secret(&self, name: &str, secret: &[u8]) -> Result<(), PlatformError> {
        let blob = dpapi_protect(secret, &Self::secret_entropy(name))?;
        let path = self.secrets_dir()?.join(format!("{name}.bin"));
        write_atomic(&path, &blob)
    }

    fn load_secret(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError> {
        let path = self.secrets_dir()?.join(format!("{name}.bin"));
        if !path.exists() {
            return Ok(None);
        }
        let blob = fs::read(&path)?;
        Ok(Some(dpapi_unprotect(&blob, &Self::secret_entropy(name))?))
    }

    fn delete_secret(&self, name: &str) -> Result<(), PlatformError> {
        let path = self.secrets_dir()?.join(format!("{name}.bin"));
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(PlatformError::Io(e)),
        }
    }

    // Implemented in P4.2.
    fn device_values(&self) -> Result<(String, String), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P7.1.
    fn set_keep_awake(&self, _on: bool) -> Result<(), PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P6.2.
    fn volume_info(&self, _path: &Path) -> Result<VolumeInfo, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P6.2.
    fn removable_drives(&self) -> Result<Vec<MountedVolume>, PlatformError> {
        Err(PlatformError::Unsupported)
    }
    // Implemented in P7.2. The exact trait returns a value, so its placeholder is the Unsupported variant.
    fn network_diagnostics(&self) -> NetworkDiagnostics {
        NetworkDiagnostics::Unsupported
    }
}
