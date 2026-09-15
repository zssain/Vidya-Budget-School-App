# LICENSE_FORMAT.md — activation and reset codes

Implemented in `crates/vidya-license`. Signing only with feature `signing` (Provider Tool). School apps contain only the public key.

## 1. Device ID (office computer)
1. Collect two platform values (PLATFORMS.md): Windows → MachineGuid and SMBIOS system UUID; macOS → IOPlatformUUID and IOPlatformSerialNumber.
2. `h = SHA-256("vidya-device-v1|" + platform + "|" + value1 + "|" + value2)` where platform is `windows` or `macos`. Values are trimmed and uppercased.
3. Take the first 8 bytes of `h` as a big-endian `u64`, shift right by 4 → 60-bit number `d`.
4. Encode `d` as exactly 12 Crockford base32 characters (most significant first, pad with leading zeros).
5. Display: `VD-XXXX-XXXX-XXXX`.
6. When reading a Device ID typed by a person: remove spaces and hyphens, uppercase, map `O`→`0`, `I` and `L`→`1`, reject `U`. Decode to the 60-bit number.

The code stores the 60-bit number (8 bytes, top 4 bits zero), because the provider only ever sees the displayed Device ID.

## 2. Payload v1 (little-endian integers)
| Field | Size | Activation | Reset |
|---|---|---|---|
| format_version | u8 | 1 | 1 |
| code_type | u8 | 1 | 2 |
| school_code_len | u8 | 3–12 | 3–12 |
| school_code | bytes | ASCII `[a-z0-9]` | same |
| device_id | u64 (8 bytes) | 60-bit device number | same |
| issued_days | u32 | days since 2020-01-01 | same |
| max_users | u16 | 1–500 | 0 |
| max_devices | u8 | 0–50 | 0 |
| license_type | u8 | 1 = standard | 0 |
| flags | u16 | 0 (reserved for update plan) | 0 |
| expires_days | u32 | absent | days since 2020-01-01 |
| nonce | 8 bytes | absent | random |

Decoding must reject: unknown version, wrong code_type for the screen, school code length or characters out of range, trailing bytes, any field out of range.

## 3. Signature
`sig = Ed25519.sign(private_key, "VIDYA-LICENSE-V1" || payload)` (64 bytes). The domain prefix is signed but not included in the code.

## 4. Text form
1. `body = payload || sig`.
2. `check = first 10 bits of SHA-256(body)` → 2 Crockford base32 characters.
3. `text = crockford_base32(body) + check` (no padding).
4. Split into groups of 5 characters with `-`, prefix `VIDYA-`.
5. Parsing: remove `VIDYA-`, spaces, hyphens and line breaks; uppercase; apply the same character mapping as Device IDs; split off the last 2 characters as check.
6. Error order and messages:
   - Cannot decode characters → `license.error.typing` "This code has a typing mistake."
   - Check characters do not match → `license.error.typing`
   - Signature invalid → `license.error.invalid` "This code is not a valid Vidya code."
   - Device mismatch → `license.error.other_device` "This code was made for a different computer."
   - Wrong type → `license.error.wrong_type`
   - Reset code expired → `license.error.expired`; nonce used → `license.error.used`

## 5. Test vectors
`vidya-license/tests/vectors.rs` must include a fixed test key pair (generated once and committed **only for tests**, clearly named `TEST_ONLY_KEY`), and at least: one valid activation code, one valid reset code, the same codes with one character changed, a code for another device, a code with bad check characters. The production public key must differ from the test key; a test asserts that.

## 6. Storage in the school app
Store the full code text, payload and signature (base64) in `license`. At every start, re-verify the stored signature and recompute the device ID. If either fails, show the locked screen (see prompt P4.2).
