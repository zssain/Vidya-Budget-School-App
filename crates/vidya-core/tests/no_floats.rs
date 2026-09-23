//! Guard: no floating-point types anywhere in vidya-core's `src/`
//! (prompts/P02 money.rs). Money and all rounding use integer math.
//! This test lives in `tests/` so it never scans itself.

use std::fs;
use std::path::Path;

#[test]
fn no_float_types_in_src() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    visit(&src, &mut offenders);
    assert!(
        offenders.is_empty(),
        "float types (f32/f64) are banned in vidya-core; found at: {offenders:?}"
    );
}

fn visit(dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).expect("read src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            visit(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = fs::read_to_string(&path).expect("read rs file");
            for (i, line) in text.lines().enumerate() {
                if has_word(line, "f32") || has_word(line, "f64") {
                    out.push(format!("{}:{}", path.display(), i + 1));
                }
            }
        }
    }
}

fn has_word(hay: &str, needle: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut from = 0;
    while let Some(rel) = hay[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before = start == 0 || !is_ident(bytes[start - 1]);
        let after = end >= bytes.len() || !is_ident(bytes[end]);
        if before && after {
            return true;
        }
        from = end;
    }
    false
}

fn is_ident(b: u8) -> bool {
    b == b'_' || b.is_ascii_alphanumeric()
}
