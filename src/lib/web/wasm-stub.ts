// The Tauri (non-web) build aliases `@wasm` to this stub so the shared Web-backend
// code bundles without the browser WASM module. None of it is ever called there
// (`isWeb` is false in the Tauri app), so every export just throws if reached.
const nope = (): never => {
  throw new Error('vidya-wasm is only available in the iPhone PWA (web) build')
}

export const vidya_core_version = nope
export const validate_new_mark = nope
export const percent_present = nope
export const hlc_new = nope
export const hlc_next = nope
export const hlc_parse = nope
export const hlc_skew = nope
export const bundle_aad = nope
export const relay_aad = nope
export const derive_direction_keys = nope
export const seal_with_nonce = nope
export const seal_open = nope
export const pin_hash_with_salt = nope
export const pin_verify = nope
export const pin_lockout_seconds = nope
export const pin_remaining = nope
export const pin_is_locked = nope
export default async function init(): Promise<unknown> {
  return {}
}
