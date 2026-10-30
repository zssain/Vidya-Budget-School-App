//! Vidya's in-repo Android plugin (docs/00-SYSTEM-CONTEXT.md §4, §9).
//!
//! Today it carries one feature: wrapping the 32-byte database key with a
//! non-exportable AES-256-GCM key held in `AndroidKeyStore` (Kotlin
//! `app.vidya.plugin.KeystorePlugin`). The app crate stores only the wrapped blob.
//!
//! The commands are called from Rust only (see `build.rs`: no webview permission
//! exists for them, and the Rust invoke handler below refuses any webview call).

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Runtime,
};

/// The wrapped key as returned by `wrap` and accepted by `unwrap`.
/// `iv` and `ct` are standard base64; `ct` includes the 16-byte GCM tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrappedBlob {
    pub v: u32,
    pub iv: String,
    pub ct: String,
}

/// Why a keystore call failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeystoreError {
    /// The wrapping key is gone or permanently unusable (KeyPermanentlyInvalidatedException,
    /// alias missing) or the blob can never be opened (GCM tag mismatch, unknown version).
    /// Retrying will not help. Never carries key material.
    Permanent { code: String, message: String },
    /// Anything else (keystore daemon busy, JNI hiccup, …). Retry on a later launch.
    Transient(String),
}

/// Error codes the Kotlin side uses for permanent failures.
pub const PERMANENT_CODES: &[&str] = &["KEY_INVALIDATED", "KEY_MISSING", "BAD_BLOB"];

#[cfg(target_os = "android")]
mod android {
    use super::{KeystoreError, WrappedBlob, PERMANENT_CODES};
    use serde::{Deserialize, Serialize};
    use tauri::{plugin::PluginHandle, Manager, Runtime};

    pub(crate) const PLUGIN_IDENTIFIER: &str = "app.vidya.plugin";

    /// Handle to the Kotlin `KeystorePlugin`.
    pub struct Keystore<R: Runtime>(pub(crate) PluginHandle<R>);

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct WrapArgs<'a> {
        plaintext_b64: &'a str,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct UnwrapResponse {
        plaintext_b64: String,
    }

    fn map_err(e: tauri::plugin::mobile::PluginInvokeError) -> KeystoreError {
        use tauri::plugin::mobile::PluginInvokeError;
        match e {
            PluginInvokeError::InvokeRejected(resp) => {
                let code = resp.code.unwrap_or_default();
                let message = resp.message.unwrap_or_default();
                if PERMANENT_CODES.contains(&code.as_str()) {
                    KeystoreError::Permanent { code, message }
                } else {
                    KeystoreError::Transient(format!("{code}: {message}"))
                }
            }
            other => KeystoreError::Transient(other.to_string()),
        }
    }

    impl<R: Runtime> Keystore<R> {
        /// Encrypt `plaintext_b64` (base64 of the raw key) under the keystore key.
        /// BLOCKS until the Kotlin side answers — never call it on the Android UI thread.
        pub fn wrap(&self, plaintext_b64: &str) -> Result<WrappedBlob, KeystoreError> {
            self.0.run_mobile_plugin("wrap", WrapArgs { plaintext_b64 }).map_err(map_err)
        }

        /// Decrypt a blob produced by [`Keystore::wrap`]; returns base64 of the raw key.
        /// BLOCKS until the Kotlin side answers — never call it on the Android UI thread.
        pub fn unwrap(&self, blob: &WrappedBlob) -> Result<String, KeystoreError> {
            self.0
                .run_mobile_plugin::<UnwrapResponse>("unwrap", blob)
                .map(|r| r.plaintext_b64)
                .map_err(map_err)
        }
    }

    /// Access the keystore from an `App`/`AppHandle`.
    pub trait VidyaAndroidExt<R: Runtime> {
        fn vidya_keystore(&self) -> &Keystore<R>;
    }

    impl<R: Runtime, T: Manager<R>> VidyaAndroidExt<R> for T {
        fn vidya_keystore(&self) -> &Keystore<R> {
            self.state::<Keystore<R>>().inner()
        }
    }
}

#[cfg(target_os = "android")]
pub use android::{Keystore, VidyaAndroidExt};

/// Initialise the plugin. Register it BEFORE anything asks for the DB key.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("vidya-android")
        // Defence in depth: claim every webview invoke for this plugin and refuse it,
        // so even a mis-granted capability can never reach the native keystore.
        .invoke_handler(|invoke| {
            invoke.resolver.reject("vidya-android commands are not callable from the webview");
            true
        })
        .setup(|_app, _api| {
            #[cfg(target_os = "android")]
            {
                use tauri::Manager;
                let handle = _api.register_android_plugin(android::PLUGIN_IDENTIFIER, "KeystorePlugin")?;
                _app.manage(android::Keystore(handle));
            }
            Ok(())
        })
        .build()
}
