package app.vidya.plugin

import android.app.Activity
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.security.keystore.KeyProperties
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.security.KeyStore
import java.util.Arrays
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * Wraps Vidya's 32-byte database key with a non-exportable AES-256-GCM key that lives
 * in AndroidKeyStore (docs/00-SYSTEM-CONTEXT.md §9). Called ONLY from Rust
 * (PluginHandle.run_mobile_plugin) — no webview permission exists for these commands.
 *
 * Rules:
 *  - The plaintext key is never logged, stored (no SharedPreferences, no files) or put
 *    in an error message. Error messages carry exception CLASS names only.
 *  - `unwrap` never deletes or regenerates anything. Only `wrap` may replace a keystore
 *    key, and only one that Android reports as permanently invalidated (any blob under
 *    such a key is unrecoverable anyway).
 *
 * Error codes (Rust: tauri-plugin-vidya-android PERMANENT_CODES):
 *  - KEY_INVALIDATED / KEY_MISSING / BAD_BLOB → permanent (retrying cannot help)
 *  - KEYSTORE_UNAVAILABLE / BAD_INPUT → transient / caller error
 */
@TauriPlugin
class KeystorePlugin(private val activity: Activity) : Plugin(activity) {
    companion object {
        private const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val ALIAS = "vidya.dbkey.wrap.v1"
        const val BLOB_VERSION = 1
        private const val TRANSFORMATION = "AES/GCM/NoPadding"
        private const val TAG_BITS = 128

        /** Authenticated (not encrypted) metadata: binds version + alias to the ciphertext. */
        fun aad(version: Int): ByteArray = "vidya.dbkey|v=$version|alias=$ALIAS".toByteArray(Charsets.UTF_8)
    }

    private fun keyStore(): KeyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }

    private fun existingKey(ks: KeyStore): SecretKey? =
        (ks.getEntry(ALIAS, null) as? KeyStore.SecretKeyEntry)?.secretKey

    private fun generateKey(): SecretKey {
        val gen = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE)
        gen.init(
            KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                // Keystore picks a fresh random IV for every encryption.
                .setRandomizedEncryptionRequired(true)
                // No user-authentication binding: the DB must open at app start without a
                // biometric prompt (the app has its own PIN lock, §9).
                .build()
        )
        return gen.generateKey()
    }

    private fun encryptCipher(): Cipher {
        val ks = keyStore()
        val key = existingKey(ks) ?: generateKey()
        val cipher = Cipher.getInstance(TRANSFORMATION)
        try {
            cipher.init(Cipher.ENCRYPT_MODE, key)
        } catch (e: KeyPermanentlyInvalidatedException) {
            // The old keystore key can never decrypt again; replace it once.
            ks.deleteEntry(ALIAS)
            cipher.init(Cipher.ENCRYPT_MODE, generateKey())
        }
        return cipher
    }

    @Command
    fun wrap(invoke: Invoke) {
        var plaintext: ByteArray? = null
        try {
            val b64 = invoke.getArgs().getString("plaintextB64", null)
            if (b64 == null) {
                invoke.reject("missing plaintextB64", "BAD_INPUT")
                return
            }
            plaintext = Base64.decode(b64, Base64.NO_WRAP)
            if (plaintext.size != 32) {
                invoke.reject("plaintext must be 32 bytes", "BAD_INPUT")
                return
            }
            val cipher = encryptCipher()
            cipher.updateAAD(aad(BLOB_VERSION))
            val ct = cipher.doFinal(plaintext)
            val ret = JSObject()
            ret.put("v", BLOB_VERSION)
            ret.put("iv", Base64.encodeToString(cipher.iv, Base64.NO_WRAP))
            ret.put("ct", Base64.encodeToString(ct, Base64.NO_WRAP))
            invoke.resolve(ret)
        } catch (e: IllegalArgumentException) {
            invoke.reject("bad base64", "BAD_INPUT")
        } catch (e: Exception) {
            invoke.reject("keystore wrap failed: ${e.javaClass.simpleName}", "KEYSTORE_UNAVAILABLE")
        } finally {
            plaintext?.let { Arrays.fill(it, 0) }
        }
    }

    @Command
    fun unwrap(invoke: Invoke) {
        var plaintext: ByteArray? = null
        try {
            val args = invoke.getArgs()
            val v = args.getInteger("v", -1)
            val ivB64 = args.getString("iv", null)
            val ctB64 = args.getString("ct", null)
            if (v != BLOB_VERSION || ivB64 == null || ctB64 == null) {
                invoke.reject("unsupported wrapped-key blob (v=$v)", "BAD_BLOB")
                return
            }
            val iv: ByteArray
            val ct: ByteArray
            try {
                iv = Base64.decode(ivB64, Base64.NO_WRAP)
                ct = Base64.decode(ctB64, Base64.NO_WRAP)
            } catch (e: IllegalArgumentException) {
                invoke.reject("wrapped-key blob is not base64", "BAD_BLOB")
                return
            }
            val ks = keyStore()
            if (!ks.containsAlias(ALIAS)) {
                invoke.reject("keystore key $ALIAS is missing", "KEY_MISSING")
                return
            }
            val key = existingKey(ks)
            if (key == null) {
                invoke.reject("keystore entry $ALIAS is not a secret key", "KEY_MISSING")
                return
            }
            val cipher = Cipher.getInstance(TRANSFORMATION)
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(TAG_BITS, iv))
            cipher.updateAAD(aad(v))
            plaintext = cipher.doFinal(ct)
            if (plaintext.size != 32) {
                invoke.reject("unwrapped key has the wrong length", "BAD_BLOB")
                return
            }
            val ret = JSObject()
            ret.put("plaintextB64", Base64.encodeToString(plaintext, Base64.NO_WRAP))
            invoke.resolve(ret)
        } catch (e: KeyPermanentlyInvalidatedException) {
            invoke.reject("keystore key permanently invalidated", "KEY_INVALIDATED")
        } catch (e: AEADBadTagException) {
            invoke.reject("wrapped key does not authenticate under the keystore key", "BAD_BLOB")
        } catch (e: Exception) {
            invoke.reject("keystore unwrap failed: ${e.javaClass.simpleName}", "KEYSTORE_UNAVAILABLE")
        } finally {
            plaintext?.let { Arrays.fill(it, 0) }
        }
    }
}
