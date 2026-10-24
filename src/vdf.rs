//! Winziger Parser für Valves Text-VDF/ACF-Format
//! (`libraryfolders.vdf`, `appmanifest_*.acf`, `loginusers.vdf`).
//!
//! Absichtlich ohne Abhängigkeiten, damit die EXE klein bleibt.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Vdf {
    Str(String),
    Obj(BTreeMap<String, Vdf>),
}

impl Vdf {
    /// Kind-Element suchen (Groß-/Kleinschreibung egal, Steam ist da inkonsistent).
    pub fn get(&self, key: &str) -> Option<&Vdf> {
        match self {
            Vdf::Obj(map) => map
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v),
            Vdf::Str(_) => None,
        }
    }

    pub fn str(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(Vdf::Str(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Vdf::Str(s) => Some(s),
            Vdf::Obj(_) => None,
        }
    }

    pub fn entries(&self) -> Vec<(&String, &Vdf)> {
        match self {
            Vdf::Obj(map) => map.iter().collect(),
            Vdf::Str(_) => Vec::new(),
        }
    }
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

#[derive(Debug, PartialEq)]
enum Tok {
    Str(String),
    Open,
    Close,
}

impl<'a> Lexer<'a> {
    fn next_tok(&mut self) -> Option<Tok> {
        loop {
            let c = *self.chars.peek()?;
            if c.is_whitespace() {
                self.chars.next();
                continue;
            }
            if c == '/' {
                // Kommentar "// ..." bis Zeilenende überspringen
                self.chars.next();
                if self.chars.peek() == Some(&'/') {
                    for c in self.chars.by_ref() {
                        if c == '\n' {
                            break;
                        }
                    }
                    continue;
                }
                return Some(Tok::Str("/".into()));
            }
            if c == '[' {
                // Bedingungen wie [$WIN32] ignorieren
                for c in self.chars.by_ref() {
                    if c == ']' {
                        break;
                    }
                }
                continue;
            }
            break;
        }
        let c = self.chars.next()?;
        match c {
            '{' => Some(Tok::Open),
            '}' => Some(Tok::Close),
            '"' => {
                let mut s = String::new();
                while let Some(c) = self.chars.next() {
                    match c {
                        '"' => break,
                        '\\' => match self.chars.next() {
                            Some('n') => s.push('\n'),
                            Some('t') => s.push('\t'),
                            Some('\\') => s.push('\\'),
                            Some('"') => s.push('"'),
                            Some(other) => {
                                s.push('\\');
                                s.push(other);
                            }
                            None => break,
                        },
                        other => s.push(other),
                    }
                }
                Some(Tok::Str(s))
            }
            other => {
                // Werte ohne Anführungszeichen
                let mut s = String::new();
                s.push(other);
                while let Some(&c) = self.chars.peek() {
                    if c.is_whitespace() || c == '{' || c == '}' || c == '"' {
                        break;
                    }
                    s.push(c);
                    self.chars.next();
                }
                Some(Tok::Str(s))
            }
        }
    }
}

fn parse_obj(lx: &mut Lexer, top_level: bool) -> BTreeMap<String, Vdf> {
    let mut map = BTreeMap::new();
    loop {
        let key = match lx.next_tok() {
            Some(Tok::Str(k)) => k,
            Some(Tok::Close) | None => return map,
            Some(Tok::Open) => {
                // Kaputte Datei – verschachtelten Block überspringen
                let _ = parse_obj(lx, false);
                continue;
            }
        };
        match lx.next_tok() {
            Some(Tok::Str(v)) => {
                map.insert(key, Vdf::Str(v));
            }
            Some(Tok::Open) => {
                let child = parse_obj(lx, false);
                map.insert(key, Vdf::Obj(child));
            }
            Some(Tok::Close) | None => {
                let _ = top_level;
                return map;
            }
        }
    }
}

/// Parst einen VDF-Text. Gibt bei kaputten Dateien so viel wie möglich zurück.
pub fn parse(text: &str) -> Vdf {
    let text = text.trim_start_matches('\u{feff}');
    let mut lx = Lexer {
        chars: text.chars().peekable(),
    };
    Vdf::Obj(parse_obj(&mut lx, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_folders_new_format() {
        let txt = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"apps"
		{
			"228980"		"236223232"
			"440"		"25000000"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
}"#;
        let v = parse(txt);
        let lf = v.get("libraryfolders").unwrap();
        assert_eq!(
            lf.get("0").unwrap().str("path"),
            Some("C:\\Program Files (x86)\\Steam")
        );
        assert_eq!(lf.get("1").unwrap().str("PATH"), Some("D:\\SteamLibrary"));
        assert_eq!(lf.get("0").unwrap().get("apps").unwrap().entries().len(), 2);
    }

    #[test]
    fn parses_old_format_and_comments() {
        let txt = "// comment\n\"LibraryFolders\" { \"TimeNextStatsReport\" \"123\" \"1\" \"E:\\\\Games\" }";
        let v = parse(txt);
        let lf = v.get("libraryfolders").unwrap();
        assert_eq!(lf.str("1"), Some("E:\\Games"));
    }

    #[test]
    fn parses_appmanifest() {
        let txt = "\u{feff}\"AppState\"\n{\n\t\"appid\"\t\t\"570\"\n\t\"name\"\t\t\"Dota 2\"\n\t\"installdir\"\t\t\"dota 2 beta\"\n\t\"StateFlags\" \"4\"\n}\n";
        let v = parse(txt);
        let st = v.get("AppState").unwrap();
        assert_eq!(st.str("appid"), Some("570"));
        assert_eq!(st.str("name"), Some("Dota 2"));
        assert_eq!(st.str("installdir"), Some("dota 2 beta"));
    }

    #[test]
    fn survives_garbage() {
        let v = parse("\"a\" { \"b\" ");
        assert!(v.get("a").is_some());
        let _ = parse("}}}{{{");
        let _ = parse("");
    }
}
