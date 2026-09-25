//! Vidya offline licence maker (prompts/P12 Step 7). **Owner's laptop only —
//! never shipped, never built by the app workspace.**
//!
//! Mints an ed25519 keypair, issues machine-bound v2 licence keys + `.vlic`
//! files, records every issue/transfer in `register.csv`, and verifies keys. See
//! `README.md` for backup and lost-laptop procedures.
//!
//! Commands:
//!   init     --dir <path>
//!   issue    --school <name> --machine <code> --utr <ref> --email <addr> [--notes <text>] [--force] [--dir <path>]
//!   transfer --licence <id> --machine <code> [--force] [--dir <path>]
//!   list     [--dir <path>]
//!   verify   <licence-key-or-.vlic-path> [--machine <code>] [--dir <path>]
//!
//! `--dir` defaults to $VIDYA_LICENCE_DIR, else the current directory.

mod format;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand::RngCore;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use format::LicenceV2;

const SIGNING_FILE: &str = "licence-signing.key";
const PUBLIC_FILE: &str = "public.key";
const REGISTER_FILE: &str = "register.csv";
const REGISTER_HEADER: &str = "date,licence_id,school,email,machine,utr,kind,previous";

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&raw);
    std::process::exit(code);
}

fn run(raw: &[String]) -> i32 {
    let Some((cmd, rest)) = raw.split_first() else {
        usage();
        return 2;
    };
    let args = Args::parse(rest);
    let result = match cmd.as_str() {
        "init" => cmd_init(&args),
        "issue" => cmd_issue(&args),
        "transfer" => cmd_transfer(&args),
        "list" => cmd_list(&args),
        "verify" => cmd_verify(&args),
        "help" | "-h" | "--help" => {
            usage();
            Ok(())
        }
        other => Err(format!("unknown command '{other}'. Run `licence-maker help`.")),
    };
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn usage() {
    eprintln!(
        "licence-maker — Vidya offline licences (owner's laptop only)\n\n\
         USAGE:\n\
         \x20 licence-maker init     --dir <path>\n\
         \x20 licence-maker issue    --school <name> --machine <code> --utr <ref> --email <addr> [--notes <t>] [--force]\n\
         \x20 licence-maker transfer --licence <id> --machine <code> [--force]\n\
         \x20 licence-maker list\n\
         \x20 licence-maker verify   <key-or-.vlic> [--machine <code>]\n\n\
         \x20 --dir defaults to $VIDYA_LICENCE_DIR, else the current directory."
    );
}

// --------------------------------------------------------------------------
// Argument parsing (no CLI crate is permitted — Step 7)
// --------------------------------------------------------------------------

/// Boolean flags take no value.
const BOOL_FLAGS: &[&str] = &["force"];

struct Args {
    positional: Vec<String>,
    flags: HashMap<String, String>,
    bools: HashSet<String>,
}

impl Args {
    fn parse(tokens: &[String]) -> Self {
        let mut positional = Vec::new();
        let mut flags = HashMap::new();
        let mut bools = HashSet::new();
        let mut i = 0;
        while i < tokens.len() {
            let tok = &tokens[i];
            if let Some(name) = tok.strip_prefix("--") {
                if let Some((k, v)) = name.split_once('=') {
                    flags.insert(k.to_string(), v.to_string());
                } else if BOOL_FLAGS.contains(&name) {
                    bools.insert(name.to_string());
                } else if i + 1 < tokens.len() && !tokens[i + 1].starts_with("--") {
                    flags.insert(name.to_string(), tokens[i + 1].clone());
                    i += 1;
                } else {
                    bools.insert(name.to_string());
                }
            } else {
                positional.push(tok.clone());
            }
            i += 1;
        }
        Self { positional, flags, bools }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.flags.get(key).map(|s| s.as_str())
    }

    fn require(&self, key: &str) -> Result<&str, String> {
        self.get(key).ok_or_else(|| format!("missing required --{key}"))
    }

    fn has(&self, key: &str) -> bool {
        self.bools.contains(key)
    }

    fn dir(&self) -> PathBuf {
        if let Some(d) = self.get("dir") {
            return PathBuf::from(d);
        }
        if let Ok(d) = std::env::var("VIDYA_LICENCE_DIR") {
            if !d.is_empty() {
                return PathBuf::from(d);
            }
        }
        PathBuf::from(".")
    }
}

// --------------------------------------------------------------------------
// Commands
// --------------------------------------------------------------------------

fn cmd_init(args: &Args) -> Result<(), String> {
    let dir = args.dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let signing_path = dir.join(SIGNING_FILE);
    if signing_path.exists() {
        return Err(format!(
            "{} already exists — refusing to overwrite an existing signing key.\n\
             If you really mean to start over, move the old key folder aside first.",
            signing_path.display()
        ));
    }

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let signing = SigningKey::from_bytes(&seed);
    let public = signing.verifying_key().to_bytes();

    write_secret(&signing_path, &STANDARD.encode(seed))?;
    std::fs::write(dir.join(PUBLIC_FILE), STANDARD.encode(public))
        .map_err(|e| format!("cannot write public key: {e}"))?;
    if !dir.join(REGISTER_FILE).exists() {
        std::fs::write(dir.join(REGISTER_FILE), format!("{REGISTER_HEADER}\n"))
            .map_err(|e| format!("cannot write register: {e}"))?;
    }

    println!("Created a new licence signing key in {}", dir.display());
    println!("  {SIGNING_FILE}   (SECRET — back this up; never commit or email it)");
    println!("  {PUBLIC_FILE}          (ship this in the app build config)");
    println!("  {REGISTER_FILE}        (the record of every licence issued)");
    println!();
    println!("Public key (base64) — paste into src-tauri/build-config/release.json as \"licence_public_key\":");
    println!("  {}", STANDARD.encode(public));
    println!();
    println!("Back up the whole folder in TWO safe places (see README.md).");
    Ok(())
}

fn cmd_issue(args: &Args) -> Result<(), String> {
    let dir = args.dir();
    let signing = load_signing(&dir)?;
    let school = args.require("school")?;
    let machine = args.require("machine")?;
    let utr = args.require("utr")?;
    let email = args.require("email")?;
    let notes = args.get("notes").unwrap_or("");

    let machine_code = format::canonical_machine_code(machine)
        .ok_or_else(|| format!("'{machine}' is not a valid machine code (checksum failed)"))?;

    // Refuse a duplicate UTR unless --force (Step 7).
    let rows = read_register(&dir)?;
    if let Some(prev) = rows.iter().find(|r| r.utr == utr && !utr.is_empty()) {
        if !args.has("force") {
            return Err(format!(
                "UTR '{utr}' is already in the register (licence {}, school {} on {}).\n\
                 If this is genuinely a second purchase, re-run with --force.",
                prev.licence_id, prev.school, prev.date
            ));
        }
        eprintln!("warning: UTR '{utr}' already used (licence {}); continuing because --force was given.", prev.licence_id);
    }

    let licence_id = uuid::Uuid::now_v7().to_string();
    let issued_at = now_rfc3339()?;
    let lic = LicenceV2 {
        v: 2,
        licence_id: licence_id.clone(),
        school_name: school.to_string(),
        machine_code: machine_code.clone(),
        issued_at: issued_at.clone(),
        plan: "perpetual".into(),
        modules: vec!["core".into()],
    };
    let (key, vlic_path) = mint(&lic, &signing, &dir)?;

    append_register(&dir, &RegisterRow {
        date: issued_at,
        licence_id: licence_id.clone(),
        school: school.to_string(),
        email: email.to_string(),
        machine: machine_code,
        utr: utr.to_string(),
        kind: "issue".into(),
        previous: notes.to_string(),
    })?;

    print_minted("Issued", &licence_id, school, &key, &vlic_path);
    Ok(())
}

fn cmd_transfer(args: &Args) -> Result<(), String> {
    let dir = args.dir();
    let signing = load_signing(&dir)?;
    let licence_id = args.require("licence")?;
    let machine = args.require("machine")?;

    let machine_code = format::canonical_machine_code(machine)
        .ok_or_else(|| format!("'{machine}' is not a valid machine code (checksum failed)"))?;

    let rows = read_register(&dir)?;
    let prev = rows
        .iter()
        .rfind(|r| r.licence_id == licence_id)
        .ok_or_else(|| format!("no licence '{licence_id}' in the register — issue it first"))?
        .clone();
    if prev.machine == machine_code && !args.has("force") {
        return Err(format!(
            "licence '{licence_id}' is already bound to {machine_code}.\n\
             Re-run with --force only if you are deliberately re-issuing the same binding."
        ));
    }

    let issued_at = now_rfc3339()?;
    let lic = LicenceV2 {
        v: 2,
        licence_id: licence_id.to_string(),
        school_name: prev.school.clone(),
        machine_code: machine_code.clone(),
        issued_at: issued_at.clone(),
        plan: "perpetual".into(),
        modules: vec!["core".into()],
    };
    let (key, vlic_path) = mint(&lic, &signing, &dir)?;

    append_register(&dir, &RegisterRow {
        date: issued_at,
        licence_id: licence_id.to_string(),
        school: prev.school.clone(),
        email: prev.email.clone(),
        machine: machine_code,
        utr: String::new(),
        kind: "transfer".into(),
        previous: prev.machine.clone(),
    })?;

    println!("Transfer licence for {} — the old PC fences itself (epoch + 1) when it next reads Drive.", prev.school);
    print_minted("Transferred", licence_id, &prev.school, &key, &vlic_path);
    Ok(())
}

fn cmd_list(args: &Args) -> Result<(), String> {
    let rows = read_register(&args.dir())?;
    if rows.is_empty() {
        println!("(register is empty)");
        return Ok(());
    }
    println!("{:<20} {:<10} {:<26} {:<18} {:<10}", "date", "kind", "school", "machine", "utr");
    for r in &rows {
        println!(
            "{:<20} {:<10} {:<26} {:<18} {:<10}",
            trunc(&r.date, 20), r.kind, trunc(&r.school, 26), r.machine, trunc(&r.utr, 10)
        );
    }
    println!("\n{} licence record(s).", rows.len());
    Ok(())
}

fn cmd_verify(args: &Args) -> Result<(), String> {
    let target = args.positional.first().ok_or("usage: licence-maker verify <key-or-.vlic>")?;
    // A path to a .vlic file, or the key text itself.
    let key_text = if Path::new(target).is_file() {
        std::fs::read_to_string(target).map_err(|e| format!("cannot read {target}: {e}"))?
    } else {
        target.clone()
    };

    let public = load_public(&args.dir())?;
    let vk = VerifyingKey::from_bytes(&public).map_err(|_| "public key is not valid".to_string())?;
    let (payload, sig_bytes) = format::parse_licence_key(&key_text)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| "signature is not 64 bytes".to_string())?;
    vk.verify_strict(&payload, &sig).map_err(|_| "signature does NOT match this key folder's public key".to_string())?;

    let lic: LicenceV2 = serde_json::from_slice(&payload).map_err(|_| "payload is not a v2 licence".to_string())?;
    if lic.v != 2 {
        return Err(format!("unexpected licence version {}", lic.v));
    }

    println!("Signature: OK (signed by this key folder)");
    println!("  licence_id : {}", lic.licence_id);
    println!("  school     : {}", lic.school_name);
    println!("  machine    : {}", lic.machine_code);
    println!("  issued_at  : {}", lic.issued_at);
    println!("  plan       : {}", lic.plan);
    println!("  modules    : {}", lic.modules.join(", "));

    if let Some(m) = args.get("machine") {
        match format::canonical_machine_code(m) {
            Some(canon) if Some(&canon) == format::canonical_machine_code(&lic.machine_code).as_ref() => {
                println!("  --machine  : MATCHES this licence");
            }
            Some(_) => println!("  --machine  : does NOT match (licence is for {})", lic.machine_code),
            None => println!("  --machine  : '{m}' is not a valid machine code"),
        }
    }
    Ok(())
}

// --------------------------------------------------------------------------
// Signing / minting
// --------------------------------------------------------------------------

/// Sign a licence and write its `.vlic` file; return `(grouped key, vlic path)`.
fn mint(lic: &LicenceV2, signing: &SigningKey, dir: &Path) -> Result<(String, PathBuf), String> {
    let (_, _, key) = build_and_sign(lic, signing);
    let file = format!("{}-{}.vlic", slug(&lic.school_name), &lic.licence_id.replace('-', "")[..8.min(lic.licence_id.replace('-', "").len())]);
    let path = dir.join(file);
    std::fs::write(&path, &key).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok((format::group5(&key), path))
}

/// Pure sign step (payload bytes, signature bytes, compact key) — testable.
fn build_and_sign(lic: &LicenceV2, signing: &SigningKey) -> (Vec<u8>, Vec<u8>, String) {
    let payload = serde_json::to_vec(lic).expect("licence serialises");
    let sig = signing.sign(&payload).to_bytes().to_vec();
    let key = format::encode_licence_key(&payload, &sig);
    (payload, sig, key)
}

fn print_minted(verb: &str, licence_id: &str, school: &str, key: &str, vlic: &Path) {
    println!("{verb} licence {licence_id} for \"{school}\".");
    println!();
    println!("Licence key (email this to the buyer — they paste it into Vidya):");
    println!("  {key}");
    println!();
    println!("Licence file: {} (you can send this .vlic instead of the key).", vlic.display());
}

// --------------------------------------------------------------------------
// Key folder IO
// --------------------------------------------------------------------------

fn load_signing(dir: &Path) -> Result<SigningKey, String> {
    let p = dir.join(SIGNING_FILE);
    let text = std::fs::read_to_string(&p)
        .map_err(|_| format!("no signing key in {} — run `licence-maker init --dir {}` first", dir.display(), dir.display()))?;
    let bytes = STANDARD.decode(text.trim().as_bytes()).map_err(|_| "signing key file is corrupt".to_string())?;
    let seed: [u8; 32] = bytes.as_slice().try_into().map_err(|_| "signing key must be 32 bytes".to_string())?;
    Ok(SigningKey::from_bytes(&seed))
}

fn load_public(dir: &Path) -> Result<[u8; 32], String> {
    let p = dir.join(PUBLIC_FILE);
    let text = std::fs::read_to_string(&p)
        .map_err(|_| format!("no {PUBLIC_FILE} in {}", dir.display()))?;
    let bytes = STANDARD.decode(text.trim().as_bytes()).map_err(|_| "public key file is corrupt".to_string())?;
    bytes.as_slice().try_into().map_err(|_| "public key must be 32 bytes".to_string())
}

fn write_secret(path: &Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents).map_err(|e| format!("cannot write signing key: {e}"))?;
    // Best-effort 0600 on Unix so the secret is not world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// --------------------------------------------------------------------------
// register.csv (tiny hand-rolled CSV — no csv crate is permitted)
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RegisterRow {
    date: String,
    licence_id: String,
    school: String,
    email: String,
    machine: String,
    utr: String,
    kind: String,
    previous: String,
}

fn read_register(dir: &Path) -> Result<Vec<RegisterRow>, String> {
    let p = dir.join(REGISTER_FILE);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => return Ok(Vec::new()),
    };
    let mut rows = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 || line.trim().is_empty() {
            continue; // header / blank
        }
        let f = parse_csv_line(line);
        rows.push(RegisterRow {
            date: f.first().cloned().unwrap_or_default(),
            licence_id: f.get(1).cloned().unwrap_or_default(),
            school: f.get(2).cloned().unwrap_or_default(),
            email: f.get(3).cloned().unwrap_or_default(),
            machine: f.get(4).cloned().unwrap_or_default(),
            utr: f.get(5).cloned().unwrap_or_default(),
            kind: f.get(6).cloned().unwrap_or_default(),
            previous: f.get(7).cloned().unwrap_or_default(),
        });
    }
    Ok(rows)
}

fn append_register(dir: &Path, row: &RegisterRow) -> Result<(), String> {
    use std::io::Write;
    let p = dir.join(REGISTER_FILE);
    if !p.exists() {
        std::fs::write(&p, format!("{REGISTER_HEADER}\n")).map_err(|e| format!("cannot create register: {e}"))?;
    }
    let line = [
        &row.date, &row.licence_id, &row.school, &row.email, &row.machine, &row.utr, &row.kind, &row.previous,
    ]
    .iter()
    .map(|s| csv_escape(s))
    .collect::<Vec<_>>()
    .join(",");
    let mut file = std::fs::OpenOptions::new().append(true).open(&p).map_err(|e| format!("cannot open register: {e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("cannot write register: {e}"))?;
    Ok(())
}

fn csv_escape(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    cur.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut cur));
            }
            other => cur.push(other),
        }
    }
    fields.push(cur);
    fields
}

// --------------------------------------------------------------------------
// small helpers
// --------------------------------------------------------------------------

fn now_rfc3339() -> Result<String, String> {
    OffsetDateTime::now_utc().format(&Rfc3339).map_err(|e| format!("clock error: {e}"))
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() { "school".into() } else { trimmed }
}

fn trunc(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keypair(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn fixture_licence() -> LicenceV2 {
        LicenceV2 {
            v: 2,
            licence_id: "0193b6c0-0000-7000-8000-000000000001".into(),
            school_name: "Saraswati Public School".into(),
            machine_code: format::machine_code("golden-machine"),
            issued_at: "2026-09-25T00:00:00Z".into(),
            plan: "perpetual".into(),
            modules: vec!["core".into()],
        }
    }

    /// Golden vector: with a fixed key + payload, licence-maker produces exactly
    /// this key. `vidya-core`'s `golden_licence_key_from_maker_verifies` test
    /// pins the SAME string and asserts `verify_v2` accepts it — the two crates
    /// therefore agree byte-for-byte on the format.
    #[test]
    fn golden_vector_is_stable() {
        let signing = keypair(42);
        let (_, sig, key) = build_and_sign(&fixture_licence(), &signing);
        assert_eq!(sig.len(), 64);
        assert_eq!(key, GOLDEN_KEY, "licence-maker key format changed — update the golden vector in BOTH crates");
    }

    #[test]
    fn machine_code_golden() {
        assert_eq!(format::machine_code("golden-machine"), GOLDEN_MACHINE_CODE);
        assert!(format::canonical_machine_code(GOLDEN_MACHINE_CODE).is_some());
    }

    #[test]
    fn verify_round_trip_and_public_key() {
        let signing = keypair(42);
        let public = signing.verifying_key();
        let (payload, sig, _key) = build_and_sign(&fixture_licence(), &signing);
        let signature = Signature::from_slice(&sig).unwrap();
        public.verify_strict(&payload, &signature).expect("round-trips");
    }

    #[test]
    fn duplicate_utr_csv_and_escape() {
        // CSV escaping survives commas and quotes.
        let escaped = csv_escape("Sri \"Vidya\", School");
        assert_eq!(escaped, "\"Sri \"\"Vidya\"\", School\"");
        assert_eq!(parse_csv_line(&escaped), vec!["Sri \"Vidya\", School".to_string()]);
    }

    #[test]
    fn arg_parser_flags_bools_and_eq() {
        let a = Args::parse(&[
            "--school".into(), "S".into(),
            "--force".into(),
            "--notes=hello world".into(),
            "pos".into(),
        ]);
        assert_eq!(a.get("school"), Some("S"));
        assert!(a.has("force"));
        assert_eq!(a.get("notes"), Some("hello world"));
        assert_eq!(a.positional, vec!["pos".to_string()]);
    }

    // Golden vector: machine_id "golden-machine" + signing seed [42;32] +
    // fixture licence. `vidya-core`'s `golden_licence_key_from_maker_verifies`
    // pins the SAME two constants and asserts verify_v2 accepts the key.
    const GOLDEN_MACHINE_CODE: &str = "3N0S-10YX-JQCR-T";
    const GOLDEN_KEY: &str = "eyJ2IjoyLCJsaWNlbmNlX2lkIjoiMDE5M2I2YzAtMDAwMC03MDAwLTgwMDAtMDAwMDAwMDAwMDAxIiwic2Nob29sX25hbWUiOiJTYXJhc3dhdGkgUHVibGljIFNjaG9vbCIsIm1hY2hpbmVfY29kZSI6IjNOMFMtMTBZWC1KUUNSLVQiLCJpc3N1ZWRfYXQiOiIyMDI2LTA5LTI1VDAwOjAwOjAwWiIsInBsYW4iOiJwZXJwZXR1YWwiLCJtb2R1bGVzIjpbImNvcmUiXX0sel8zbFpx8xCuDxYhHoM9kZ6raBqbSkf-O3JKmdQ5t2OK9Z0ZYQO3RhjJdKu2gdWndFijhL48Powb0xSDAIoD";
}
