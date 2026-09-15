# PLATFORMS.md — Windows, macOS and Android differences

All platform code lives in `src-tauri/src/platform/` behind this trait (extend only with the developer's approval):

```rust
pub trait Platform: Send + Sync {
    fn name(&self) -> &'static str;                        // "windows" | "macos" | "android"
    fn data_dir(&self) -> Result<PathBuf, PlatformError>;
    fn backups_dir(&self) -> Result<PathBuf, PlatformError>; // desktop only; Android returns an error
    fn load_or_create_db_key(&self) -> Result<Zeroizing<[u8; 32]>, PlatformError>;
    fn store_secret(&self, name: &str, secret: &[u8]) -> Result<(), PlatformError>;
    fn load_secret(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, PlatformError>;
    fn delete_secret(&self, name: &str) -> Result<(), PlatformError>;
    fn device_values(&self) -> Result<(String, String), PlatformError>; // desktop: for Device ID
    fn set_keep_awake(&self, on: bool) -> Result<(), PlatformError>;    // desktop
    fn removable_drives(&self) -> Result<Vec<RemovableDrive>, PlatformError>; // desktop
    fn network_diagnostics(&self) -> NetworkDiagnostics;                // desktop
}
```

| Concern | Windows | macOS | Android |
|---|---|---|---|
| App file | NSIS `.exe` installer | Universal `.dmg` with `Vidya.app` | `.apk` per ABI, `.aab` |
| Built on | GitHub Actions `windows-latest` | MacBook, and `macos-latest` in CI | MacBook, and `ubuntu-latest` in CI |
| Tested on | Windows 11 ARM VM (bridged), real PC before release | MacBook | Real phone over USB |
| Data folder | `C:\ProgramData\Vidya\data` | `~/Library/Application Support/in.vidya.school/data` | App private storage from Tauri path API |
| Backups folder | `C:\ProgramData\Vidya\backups` | `~/Library/Application Support/in.vidya.school/backups` | none |
| Database key | DPAPI `CryptProtectData`, machine scope, blob in `data\key.bin` | Keychain generic password `in.vidya.school.dbkey`, this device only | Android Keystore wraps key, blob in private storage (Kotlin plugin) |
| Other secrets | DPAPI blobs in `data\secrets\` | Keychain items `in.vidya.school.<name>` | Keystore |
| Device values | `HKLM\SOFTWARE\Microsoft\Cryptography\MachineGuid` + WMI `Win32_ComputerSystemProduct.UUID` | IOKit `IOPlatformUUID` + `IOPlatformSerialNumber` | not used |
| Keep awake | `SetThreadExecutionState(ES_CONTINUOUS \| ES_SYSTEM_REQUIRED)` | IOKit power assertion `PreventUserIdleSystemSleep` | not used |
| Background | Tray icon, window close hides | Menu bar icon, window close hides | Foreground only; sync when app opens |
| Start at login | tauri-plugin-autostart | tauri-plugin-autostart (verify method on current macOS) | not used |
| Incoming connections | Firewall rule added by NSIS hook: TCP 47631, UDP 47632, remote LocalSubnet | User must Allow; Info.plist local network usage text and Bonjour service `_vidya._tcp` (verify current keys) | not used |
| LAN discovery | mdns-sd advertise + UDP responder | same | mdns-sd browse with Wi-Fi MulticastLock (Kotlin), UDP broadcast |
| Android permissions | – | – | INTERNET, ACCESS_NETWORK_STATE, ACCESS_WIFI_STATE, CHANGE_WIFI_MULTICAST_STATE; check current rules for local network access |
| Removable drives | Drive type removable via Windows API | Volumes under `/Volumes` that are removable/ejectable | not used |
| Printing | Webview print or PDF (decided in P5.1) | same | Share PDF |
| Screenshots blocked | no | no | `FLAG_SECURE` on sensitive screens |
| Signing | Authenticode (P10.1) | Developer ID + notarization (P10.2) | Release keystore (P10.3) |
| Uninstall keeps data | Yes, uninstaller must not remove ProgramData | Yes (Trash only removes the app) | No, Android removes app data |

## Testing Windows-only code without a Windows PC
1. Unit-test logic that surrounds the Windows call with a fake `Platform`.
2. CI `windows-latest` compiles, runs clippy and runs tests, including integration tests that call DPAPI and the registry for real.
3. Add the manual check to `docs/PROGRESS.md` "Check in Windows VM" with exact steps.
