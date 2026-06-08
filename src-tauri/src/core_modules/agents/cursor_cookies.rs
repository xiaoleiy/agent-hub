//! Best-effort extraction of the Cursor session cookie
//! (`WorkosCursorSessionToken`) from local browsers on macOS, mirroring the
//! CodexBar approach. Everything here is fail-soft: any browser that isn't
//! present, is locked, requires permissions we don't have (Safari needs Full
//! Disk Access), or has changed format simply yields `None` and we try the next.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use aes::Aes128;
use rusqlite::{Connection, OpenFlags};
use sha1::Sha1;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Aes128CbcDec = cbc::Decryptor<Aes128>;

const COOKIE_NAME: &str = "WorkosCursorSessionToken";

fn home() -> PathBuf {
    dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Try every known local browser; return the first Cursor session cookie found.
///
/// Firefox and Safari store cookies in the clear, so they're tried first — only
/// if neither has the cookie do we touch Chromium, whose key lives in the macOS
/// Keychain and triggers a one-time "allow access" prompt.
pub fn find_cursor_session_cookie() -> Option<String> {
    if let Some(v) = firefox_cookie() {
        return Some(v);
    }
    if let Some(v) = safari_cookie() {
        return Some(v);
    }
    for b in chromium_browsers() {
        if let Some(v) = chromium_cookie(&b) {
            return Some(v);
        }
    }
    None
}

// ─── Chromium family (Chrome, Edge, Brave, Arc, Vivaldi, Opera, Chromium) ────

struct ChromiumBrowser {
    /// Path under ~/Library/Application Support
    support_subdir: &'static str,
    /// macOS Keychain service holding the "Safe Storage" password
    keychain_service: &'static str,
}

fn chromium_browsers() -> Vec<ChromiumBrowser> {
    vec![
        ChromiumBrowser { support_subdir: "Google/Chrome", keychain_service: "Chrome Safe Storage" },
        ChromiumBrowser { support_subdir: "Arc", keychain_service: "Arc Safe Storage" },
        ChromiumBrowser { support_subdir: "BraveSoftware/Brave-Browser", keychain_service: "Brave Safe Storage" },
        ChromiumBrowser { support_subdir: "Microsoft Edge", keychain_service: "Microsoft Edge Safe Storage" },
        ChromiumBrowser { support_subdir: "Vivaldi", keychain_service: "Vivaldi Safe Storage" },
        ChromiumBrowser { support_subdir: "com.operasoftware.Opera", keychain_service: "Opera Safe Storage" },
        ChromiumBrowser { support_subdir: "Chromium", keychain_service: "Chromium Safe Storage" },
    ]
}

fn chromium_cookie(browser: &ChromiumBrowser) -> Option<String> {
    let base = home()
        .join("Library/Application Support")
        .join(browser.support_subdir);
    if !base.exists() {
        return None;
    }

    let password = keychain_password(browser.keychain_service)?;
    let key = derive_chromium_key(&password);

    for profile in profile_dirs(&base) {
        // Chrome moved the cookie DB into Network/ in recent versions.
        for rel in ["Network/Cookies", "Cookies"] {
            let db = profile.join(rel);
            if db.exists() {
                if let Some(v) = read_chromium_cookie_db(&db, &key) {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// "saltysalt" + 1003 rounds of PBKDF2-HMAC-SHA1 → 16-byte AES key (macOS).
fn derive_chromium_key(password: &str) -> [u8; 16] {
    let mut key = [0u8; 16];
    pbkdf2::pbkdf2_hmac::<Sha1>(password.as_bytes(), b"saltysalt", 1003, &mut key);
    key
}

fn read_chromium_cookie_db(db: &Path, key: &[u8; 16]) -> Option<String> {
    let conn = open_sqlite_snapshot(db)?;
    let mut stmt = conn
        .prepare(
            "SELECT encrypted_value, value FROM cookies \
             WHERE name = ?1 AND host_key LIKE '%cursor.com'",
        )
        .ok()?;
    let rows = stmt
        .query_map([COOKIE_NAME], |row| {
            let enc: Vec<u8> = row.get(0).unwrap_or_default();
            let plain: String = row.get(1).unwrap_or_default();
            Ok((enc, plain))
        })
        .ok()?;

    for row in rows.flatten() {
        let (enc, plain) = row;
        if !plain.is_empty() {
            return Some(plain);
        }
        if let Some(v) = decrypt_chromium_value(&enc, key) {
            return Some(v);
        }
    }
    None
}

/// Decrypt a Chromium `v10` cookie value (macOS: AES-128-CBC, IV = 16×0x20).
fn decrypt_chromium_value(enc: &[u8], key: &[u8; 16]) -> Option<String> {
    if enc.len() <= 3 || &enc[0..3] != b"v10" {
        return None;
    }
    let ct = &enc[3..];
    let iv = [0x20u8; 16];
    let pt = Aes128CbcDec::new(key.into(), (&iv).into())
        .decrypt_padded_vec_mut::<Pkcs7>(ct)
        .ok()?;

    // Recent Chromium prepends a 32-byte SHA-256 of the domain to the plaintext.
    // That prefix isn't valid UTF-8, so if the whole buffer doesn't decode,
    // drop the first 32 bytes and try again.
    let s = match std::str::from_utf8(&pt) {
        Ok(s) => s.to_string(),
        Err(_) if pt.len() > 32 => String::from_utf8_lossy(&pt[32..]).into_owned(),
        Err(_) => return None,
    };
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn profile_dirs(base: &Path) -> Vec<PathBuf> {
    let mut dirs = vec![base.join("Default")];
    if let Ok(entries) = fs::read_dir(base) {
        for e in entries.filter_map(|e| e.ok()) {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("Profile ") {
                dirs.push(e.path());
            }
        }
    }
    dirs.push(base.to_path_buf()); // some setups keep Cookies at the root
    dirs
}

// ─── Firefox (plaintext sqlite) ──────────────────────────────────────────────

fn firefox_cookie() -> Option<String> {
    let profiles = home().join("Library/Application Support/Firefox/Profiles");
    let entries = fs::read_dir(&profiles).ok()?;
    for e in entries.filter_map(|e| e.ok()) {
        let db = e.path().join("cookies.sqlite");
        if !db.exists() {
            continue;
        }
        if let Some(conn) = open_sqlite_snapshot(&db) {
            let value: Option<String> = conn
                .query_row(
                    "SELECT value FROM moz_cookies \
                     WHERE name = ?1 AND host LIKE '%cursor.com' LIMIT 1",
                    [COOKIE_NAME],
                    |row| row.get(0),
                )
                .ok();
            if let Some(v) = value {
                if !v.is_empty() {
                    return Some(v);
                }
            }
        }
    }
    None
}

// ─── Safari (Cookies.binarycookies) ──────────────────────────────────────────

fn safari_cookie() -> Option<String> {
    let candidates = [
        home().join("Library/Cookies/Cookies.binarycookies"),
        home().join("Library/Containers/com.apple.Safari/Data/Library/Cookies/Cookies.binarycookies"),
    ];
    for path in candidates {
        if let Ok(data) = fs::read(&path) {
            if let Some(v) = parse_binarycookies(&data) {
                return Some(v);
            }
        }
    }
    None
}

/// Minimal parser for Safari's Cookies.binarycookies format. Returns the value
/// of the Cursor session cookie for a cursor.com domain if present.
fn parse_binarycookies(data: &[u8]) -> Option<String> {
    if data.len() < 8 || &data[0..4] != b"cook" {
        return None;
    }
    let num_pages = be_u32(data, 4)? as usize;
    let mut offset = 8;
    let mut page_sizes = Vec::with_capacity(num_pages);
    for _ in 0..num_pages {
        page_sizes.push(be_u32(data, offset)? as usize);
        offset += 4;
    }
    // Pages start right after the page-size table.
    let mut page_start = offset;
    for size in page_sizes {
        let page = data.get(page_start..page_start + size)?;
        if let Some(v) = parse_binarycookies_page(page) {
            return Some(v);
        }
        page_start += size;
    }
    None
}

fn parse_binarycookies_page(page: &[u8]) -> Option<String> {
    // Page header: 4-byte tag (0x00000100), then LE u32 cookie count.
    if page.len() < 8 {
        return None;
    }
    let count = le_u32(page, 4)? as usize;
    for i in 0..count {
        let off = le_u32(page, 8 + i * 4)? as usize;
        if let Some(cookie) = page.get(off..) {
            if let Some(v) = parse_binarycookie(cookie) {
                return Some(v);
            }
        }
    }
    None
}

fn parse_binarycookie(c: &[u8]) -> Option<String> {
    // Cookie record (offsets relative to the record start):
    //   0: LE u32 size
    //  16: LE u32 url offset
    //  20: LE u32 name offset
    //  24: LE u32 path offset
    //  28: LE u32 value offset
    if c.len() < 32 {
        return None;
    }
    let size = le_u32(c, 0)? as usize;
    let rec = c.get(0..size)?;
    let url = cstr_at(rec, le_u32(rec, 16)? as usize)?;
    let name = cstr_at(rec, le_u32(rec, 20)? as usize)?;
    let value = cstr_at(rec, le_u32(rec, 28)? as usize)?;
    if name == COOKIE_NAME && url.contains("cursor.com") && !value.is_empty() {
        Some(value)
    } else {
        None
    }
}

fn cstr_at(buf: &[u8], start: usize) -> Option<String> {
    let slice = buf.get(start..)?;
    let end = slice.iter().position(|&b| b == 0).unwrap_or(slice.len());
    Some(String::from_utf8_lossy(&slice[..end]).into_owned())
}

fn be_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn le_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

// ─── Shared helpers ──────────────────────────────────────────────────────────

/// Read the "Safe Storage" password for a browser from the macOS Keychain.
fn keychain_password(service: &str) -> Option<String> {
    let out = Command::new("security")
        .args(["find-generic-password", "-s", service, "-w"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let pw = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if pw.is_empty() {
        None
    } else {
        Some(pw)
    }
}

/// Open a (possibly browser-locked) SQLite cookie DB by copying it to a temp
/// snapshot first, then opening read-only. Avoids lock contention with a
/// running browser.
fn open_sqlite_snapshot(db: &Path) -> Option<Connection> {
    let tag = db
        .components()
        .rev()
        .take(3)
        .map(|c| c.as_os_str().to_string_lossy().replace(['/', ' '], "_"))
        .collect::<Vec<_>>()
        .join("-");
    let tmp = std::env::temp_dir().join(format!("agent-hub-cookie-{}.sqlite", tag));
    fs::copy(db, &tmp).ok()?;
    Connection::open_with_flags(&tmp, OpenFlags::SQLITE_OPEN_READ_ONLY).ok()
}
