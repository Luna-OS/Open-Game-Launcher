//! Liest Steams `appcache/appinfo.vdf` (binäres Format, Version 39–41)
//! und holt daraus nur Name und Typ jeder App. So bekommen auch nicht
//! installierte Spiele ihren richtigen Namen – ohne Internet und ohne API-Schlüssel.
//! Formatbeschreibung: https://github.com/SteamDatabase/SteamAppInfo

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct AppMeta {
    pub name: String,
    /// "game", "dlc", "tool", "application", "demo", "music" …
    pub kind: String,
}

struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn u8(&mut self) -> Option<u8> {
        let v = *self.data.get(self.pos)?;
        self.pos += 1;
        Some(v)
    }
    fn u32(&mut self) -> Option<u32> {
        let b = self.data.get(self.pos..self.pos + 4)?;
        self.pos += 4;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn i64(&mut self) -> Option<i64> {
        let b = self.data.get(self.pos..self.pos + 8)?;
        self.pos += 8;
        Some(i64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]))
    }
    fn skip(&mut self, n: usize) -> Option<()> {
        if self.pos + n > self.data.len() {
            return None;
        }
        self.pos += n;
        Some(())
    }
    fn cstr(&mut self) -> Option<String> {
        let rest = self.data.get(self.pos..)?;
        let end = rest.iter().position(|b| *b == 0)?;
        let s = String::from_utf8_lossy(&rest[..end]).to_string();
        self.pos += end + 1;
        Some(s)
    }
}

/// Liest einen binären KeyValues-Block und sammelt nur die Werte, die wir brauchen
/// (Pfad `appinfo/common/name` und `appinfo/common/type`).
fn read_kv(r: &mut Reader, strings: Option<&[String]>, path: &mut Vec<String>, out: &mut AppMeta) -> Option<()> {
    loop {
        let t = r.u8()?;
        if t == 0x08 || t == 0x0B {
            return Some(());
        }
        let key = match strings {
            Some(table) => {
                let idx = r.u32()? as usize;
                table.get(idx).cloned().unwrap_or_default()
            }
            None => r.cstr()?,
        };
        match t {
            0x00 => {
                path.push(key);
                read_kv(r, strings, path, out)?;
                path.pop();
            }
            0x01 => {
                let v = r.cstr()?;
                if path.len() == 2 && path[1].eq_ignore_ascii_case("common") {
                    if key.eq_ignore_ascii_case("name") {
                        out.name = v;
                    } else if key.eq_ignore_ascii_case("type") {
                        out.kind = v.to_lowercase();
                    }
                }
            }
            0x02 | 0x03 | 0x04 | 0x06 => r.skip(4)?,
            0x07 | 0x0A => r.skip(8)?,
            0x05 => {
                // UTF-16-String bis 0x0000
                loop {
                    let a = r.u8()?;
                    let b = r.u8()?;
                    if a == 0 && b == 0 {
                        break;
                    }
                }
            }
            _ => return None,
        }
    }
}

/// Alle Apps mit Name und Typ. Bei unbekanntem Format: leere Liste.
pub fn parse(data: &[u8]) -> HashMap<u32, AppMeta> {
    let mut out = HashMap::new();
    let mut r = Reader { data, pos: 0 };
    let Some(magic) = r.u32() else { return out };
    let version = magic & 0xFF;
    if magic >> 8 != 0x07_56_44 || !(39..=41).contains(&version) {
        return out;
    }
    if r.u32().is_none() {
        return out;
    }
    let mut table: Option<Vec<String>> = None;
    if version >= 41 {
        let Some(offset) = r.i64() else { return out };
        let mut t = Reader { data, pos: offset.max(0) as usize };
        if let Some(count) = t.u32() {
            let mut v = Vec::with_capacity(count.min(1_000_000) as usize);
            for _ in 0..count {
                match t.cstr() {
                    Some(s) => v.push(s),
                    None => break,
                }
            }
            table = Some(v);
        }
    }
    loop {
        let Some(appid) = r.u32() else { break };
        if appid == 0 {
            break;
        }
        let Some(size) = r.u32() else { break };
        let end = r.pos + size as usize;
        if end > data.len() {
            break;
        }
        // InfoState, LastUpdated, Token, SHA1, ChangeNumber (+ BinarySHA1 ab v40)
        let header = 4 + 4 + 8 + 20 + 4 + if version >= 40 { 20 } else { 0 };
        if r.skip(header).is_some() {
            let mut meta = AppMeta { name: String::new(), kind: String::new() };
            let mut path = Vec::new();
            let _ = read_kv(&mut r, table.as_deref(), &mut path, &mut meta);
            if !meta.name.is_empty() {
                out.insert(appid, meta);
            }
        }
        r.pos = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kv_v41(name_idx: u32, type_idx: u32, appinfo: u32, common: u32, name: &str, kind: &str) -> Vec<u8> {
        let mut v = Vec::new();
        v.push(0x00);
        v.extend_from_slice(&appinfo.to_le_bytes());
        v.push(0x02);
        v.extend_from_slice(&99u32.to_le_bytes());
        v.extend_from_slice(&7u32.to_le_bytes());
        v.push(0x00);
        v.extend_from_slice(&common.to_le_bytes());
        v.push(0x01);
        v.extend_from_slice(&name_idx.to_le_bytes());
        v.extend_from_slice(name.as_bytes());
        v.push(0);
        v.push(0x01);
        v.extend_from_slice(&type_idx.to_le_bytes());
        v.extend_from_slice(kind.as_bytes());
        v.push(0);
        v.push(0x08); // Ende common
        v.push(0x08); // Ende appinfo
        v.push(0x08); // Ende Wurzel
        v
    }

    #[test]
    fn parses_v41_with_string_table() {
        let strings = ["appinfo", "common", "name", "type", "x"];
        let mut file = Vec::new();
        file.extend_from_slice(&0x07564429u32.to_le_bytes());
        file.extend_from_slice(&1u32.to_le_bytes());
        let offset_pos = file.len();
        file.extend_from_slice(&0i64.to_le_bytes());
        for (id, name, kind) in [(570u32, "Dota 2", "Game"), (228980, "Steamworks Common Redistributables", "Tool")] {
            let kv = kv_v41(2, 3, 0, 1, name, kind);
            file.extend_from_slice(&id.to_le_bytes());
            file.extend_from_slice(&((4 + 4 + 8 + 20 + 4 + 20 + kv.len()) as u32).to_le_bytes());
            file.extend_from_slice(&[0u8; 4 + 4 + 8 + 20 + 4 + 20]);
            file.extend_from_slice(&kv);
        }
        file.extend_from_slice(&0u32.to_le_bytes());
        let table_pos = file.len() as i64;
        file[offset_pos..offset_pos + 8].copy_from_slice(&table_pos.to_le_bytes());
        file.extend_from_slice(&(strings.len() as u32).to_le_bytes());
        for s in strings {
            file.extend_from_slice(s.as_bytes());
            file.push(0);
        }
        let apps = parse(&file);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[&570].name, "Dota 2");
        assert_eq!(apps[&570].kind, "game");
        assert_eq!(apps[&228980].kind, "tool");
    }

    #[test]
    fn parses_v40_inline_keys() {
        let mut kv = Vec::new();
        kv.push(0x00);
        kv.extend_from_slice(b"appinfo\0");
        kv.push(0x00);
        kv.extend_from_slice(b"common\0");
        kv.push(0x01);
        kv.extend_from_slice(b"name\0Portal 2\0");
        kv.push(0x01);
        kv.extend_from_slice(b"type\0game\0");
        kv.push(0x07);
        kv.extend_from_slice(b"big\0");
        kv.extend_from_slice(&[1u8; 8]);
        kv.extend_from_slice(&[0x08, 0x08, 0x08]);
        let mut file = Vec::new();
        file.extend_from_slice(&0x07564428u32.to_le_bytes());
        file.extend_from_slice(&1u32.to_le_bytes());
        file.extend_from_slice(&620u32.to_le_bytes());
        file.extend_from_slice(&((60 + kv.len()) as u32).to_le_bytes());
        file.extend_from_slice(&[0u8; 60]);
        file.extend_from_slice(&kv);
        file.extend_from_slice(&0u32.to_le_bytes());
        let apps = parse(&file);
        assert_eq!(apps[&620].name, "Portal 2");
    }

    #[test]
    fn survives_garbage() {
        assert!(parse(b"").is_empty());
        assert!(parse(&[0x29, 0x44, 0x56, 0x07, 1, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 127]).is_empty());
        let mut f = vec![0x28, 0x44, 0x56, 0x07, 1, 0, 0, 0, 5, 0, 0, 0, 255, 255, 0, 0];
        f.extend_from_slice(&[0u8; 10]);
        let _ = parse(&f);
    }
}
