const LETTERS: &[u8] = b"abcdefghjkmnpqrstuvwxyz";
const DIGITS: &[u8] = b"23456789";
const BASE64URL: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Formats supplied random bytes as four readable letters and four non-ambiguous digits.
pub fn format_temp_password(bytes: [u8; 8]) -> String {
    let mut output = String::with_capacity(9);
    for byte in &bytes[..4] {
        output.push(char::from(LETTERS[usize::from(*byte) % LETTERS.len()]));
    }
    output.push('-');
    for byte in &bytes[4..] {
        output.push(char::from(DIGITS[usize::from(*byte) % DIGITS.len()]));
    }
    output
}

/// Encodes 32 supplied random bytes as unpadded base64url for a session token.
pub fn format_session_token(bytes: [u8; 32]) -> String {
    let mut output = String::with_capacity(43);
    let (chunks, remainder) = bytes.as_slice().as_chunks::<3>();
    for chunk in chunks {
        let value = (u32::from(chunk[0]) << 16) | (u32::from(chunk[1]) << 8) | u32::from(chunk[2]);
        output.push(char::from(BASE64URL[((value >> 18) & 63) as usize]));
        output.push(char::from(BASE64URL[((value >> 12) & 63) as usize]));
        output.push(char::from(BASE64URL[((value >> 6) & 63) as usize]));
        output.push(char::from(BASE64URL[(value & 63) as usize]));
    }
    if remainder.len() == 2 {
        let value = (u32::from(remainder[0]) << 16) | (u32::from(remainder[1]) << 8);
        output.push(char::from(BASE64URL[((value >> 18) & 63) as usize]));
        output.push(char::from(BASE64URL[((value >> 12) & 63) as usize]));
        output.push(char::from(BASE64URL[((value >> 6) & 63) as usize]));
    }
    output
}
