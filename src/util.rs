//! Kleine Helfer ohne Abhängigkeiten: SHA-256, Base64, Zufallszahlen.
//! (Bewusst selbst geschrieben, damit die EXE klein bleibt.)

// ----------------------------------------------------------------------
// SHA-256 (für PKCE bei der Anmeldung)
// ----------------------------------------------------------------------

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) = (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ----------------------------------------------------------------------
// Base64
// ----------------------------------------------------------------------

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64url ohne Auffüllzeichen (für PKCE)
pub fn base64url(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        let chars = [(n >> 18) & 63, (n >> 12) & 63, (n >> 6) & 63, n & 63];
        for (i, c) in chars.iter().enumerate() {
            if i <= chunk.len() {
                let ch = B64[*c as usize] as char;
                out.push(match ch {
                    '+' => '-',
                    '/' => '_',
                    other => other,
                });
            }
        }
    }
    out
}

/// Geheimnisse (Anmelde-Tokens) für die Einstellungsdatei: unter Windows mit DPAPI
/// verschlüsselt ("dpapi:…"), sodass andere Benutzer oder kopierte Dateien nichts nützen.
/// Alte, unverschlüsselte Werte werden beim Lesen weiter akzeptiert.
pub mod secret {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn protect(plain: &str) -> String {
        if plain.is_empty() {
            return String::new();
        }
        #[cfg(windows)]
        {
            if let Some(enc) = crate::winffi::dpapi_protect(plain.as_bytes()) {
                return format!("dpapi:{}", super::base64url(&enc));
            }
        }
        plain.to_string()
    }

    pub fn reveal(stored: &str) -> String {
        match stored.strip_prefix("dpapi:") {
            Some(b64) => {
                #[cfg(windows)]
                {
                    super::base64_decode(b64)
                        .and_then(|enc| crate::winffi::dpapi_unprotect(&enc))
                        .and_then(|plain| String::from_utf8(plain).ok())
                        .unwrap_or_default()
                }
                #[cfg(not(windows))]
                {
                    let _ = b64;
                    String::new()
                }
            }
            None => stored.to_string(),
        }
    }

    pub fn serialize<S: Serializer>(v: &str, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&protect(v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
        let raw = String::deserialize(d)?;
        Ok(reveal(&raw))
    }
}

/// Base64 dekodieren (Standard und URL-Variante, ignoriert Leerzeichen/Zeilenumbrüche)
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for c in text.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            b' ' | b'\n' | b'\r' | b'\t' => continue,
            _ => return None,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

// ----------------------------------------------------------------------
// Zufall (kryptografisch sicher, vom Betriebssystem)
// ----------------------------------------------------------------------

pub fn random_bytes(n: usize) -> Vec<u8> {
    let mut buf = vec![0u8; n];
    #[cfg(windows)]
    {
        #[link(name = "advapi32")]
        extern "system" {
            #[link_name = "SystemFunction036"]
            fn RtlGenRandom(buf: *mut u8, len: u32) -> u8;
        }
        let ok = unsafe { RtlGenRandom(buf.as_mut_ptr(), n as u32) };
        if ok != 0 {
            return buf;
        }
    }
    #[cfg(not(windows))]
    {
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            if f.read_exact(&mut buf).is_ok() {
                return buf;
            }
        }
    }
    // Notlösung (sollte praktisch nie passieren)
    let mut x = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        ^ ((std::process::id() as u64) << 32);
    for b in buf.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *b = x as u8;
    }
    buf
}

/// Prozent-Dekodierung für URL-Parameter (z. B. ?code=...)
pub fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
                match hex {
                    Some(v) => {
                        out.push(v);
                        i += 2;
                    }
                    None => out.push(b'%'),
                }
            }
            other => out.push(other),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Parameter aus einer Query (`a=1&b=2`) holen
pub fn query_param(query: &str, name: &str) -> Option<String> {
    query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == name)
        .map(|(_, v)| url_decode(v))
}

/// ISO-8601-Datum ("2026-10-29T15:00:00.000Z" oder "2026-10-29 23:59:00") → Unix-Sekunden (UTC)
pub fn parse_datetime(s: &str) -> Option<u64> {
    let s = s.trim();
    let date = s.get(0..10)?;
    let mut parts = date.split('-');
    let y: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let d: i64 = parts.next()?.parse().ok()?;
    let time = s.get(11..19).unwrap_or("00:00:00");
    let mut tp = time.split(':');
    let hh: i64 = tp.next()?.parse().ok()?;
    let mm: i64 = tp.next()?.parse().ok()?;
    let ss: i64 = tp.next().unwrap_or("0").parse().ok()?;
    // Tage seit 1970 (Algorithmus von Howard Hinnant)
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = if y2 >= 0 { y2 } else { y2 - 399 } / 400;
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let t = days * 86_400 + hh * 3600 + mm * 60 + ss;
    (t >= 0).then_some(t as u64)
}

/// Unix-Sekunden → (Jahr, Monat, Tag, Stunde, Minute)
pub fn civil(ts: i64) -> (i64, u32, u32, u32, u32) {
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d, (secs / 3600) as u32, ((secs % 3600) / 60) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn sha256_vectors() {
        assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let long = vec![b'a'; 1000];
        assert_eq!(hex(&sha256(&long)), "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3");
    }

    #[test]
    fn pkce_rfc7636_example() {
        // RFC 7636, Anhang B
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(base64url(&sha256(verifier.as_bytes())), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn base64_roundtrip() {
        assert_eq!(base64_decode("aGFsbG8gd2VsdA==").unwrap(), b"hallo welt");
        assert_eq!(base64_decode("aGFs\r\nbG8=").unwrap(), b"hallo");
        assert_eq!(base64url(b"hallo"), "aGFsbG8");
        assert!(base64_decode("§§").is_none());
    }

    #[test]
    fn misc() {
        assert_eq!(random_bytes(32).len(), 32);
        assert_eq!(query_param("?code=ab%2Bc&x=1", "code").as_deref(), Some("ab+c"));
        assert_eq!(url_decode("100%"), "100%");
        assert_eq!(parse_datetime("1970-01-02T00:00:00.000Z"), Some(86_400));
        assert_eq!(parse_datetime("2026-10-29T15:00:00.000Z"), Some(1_793_286_000));
        assert_eq!(parse_datetime("2026-10-29 15:00:00"), Some(1_793_286_000));
        assert_eq!(parse_datetime("N/A"), None);
        assert_eq!(civil(1_793_286_000), (2026, 10, 29, 15, 0));
        assert_eq!(civil(0), (1970, 1, 1, 0, 0));
        assert_eq!(civil(951_782_400), (2000, 2, 29, 0, 0));
    }
}
