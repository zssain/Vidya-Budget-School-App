// Committed type surface for the generated vidya-wasm module (`npm run build:wasm`
// → web-pwa/wasm/, gitignored). tsconfig maps `@wasm` → this file so tsc resolves
// `import … from '@wasm'` even before the module is generated (fresh checkout / CI).
// The import is aliased to the REAL module at build time in the PWA build
// (web-pwa/vite.config.ts) and to a stub in the Tauri/test build (vite.config.ts).
export function vidya_core_version(): string
export function validate_new_mark(mark: string): string | undefined
export function percent_present(p: number, a: number, l: number): number
export function hlc_new(wall_ms: number, counter: number, device_id: string): string
export function hlc_next(prev: string | null | undefined, wall_ms: number, device_id: string): string
export function hlc_parse(s: string): string
export function hlc_skew(local_wall_ms: number, remote_wall_ms: number): boolean
export function bundle_aad(audience: string, key_version: number): Uint8Array
export function relay_aad(method: string, path: string, device_id: string, server_epoch: number): Uint8Array
export function derive_direction_keys(session_key_b64: string): Uint8Array
export function seal_with_nonce(key: Uint8Array, aad: Uint8Array, nonce: Uint8Array, plaintext: Uint8Array): Uint8Array
export function seal_open(key: Uint8Array, aad: Uint8Array, sealed: Uint8Array): Uint8Array
export function pin_hash_with_salt(pin: string, salt: Uint8Array): string
export function pin_verify(pin_input: string, phc: string): boolean
export function pin_lockout_seconds(fail_count: number): number | undefined
export function pin_remaining(fail_count: number): number
export function pin_is_locked(locked_until_ms: number | null | undefined, now_ms: number): boolean
export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module
export default function init(
  module_or_path?: InitInput | Promise<InitInput> | { module_or_path: InitInput | Promise<InitInput> },
): Promise<unknown>
