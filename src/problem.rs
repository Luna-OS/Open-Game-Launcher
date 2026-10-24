//! Verständliche Fehlererklärungen.
//!
//! Jeder Fehler wird in ein [`Problem`] übersetzt: Was ist passiert, warum,
//! und Schritt für Schritt, wie man es behebt – in einfacher Sprache.
//! Dieses Modul hat keine Abhängigkeiten und ist komplett testbar.

#[derive(Clone, Debug, PartialEq)]
pub enum Fix {
    /// Spiel mit Administratorrechten erneut starten
    RunAsAdmin,
    /// "Spiel bearbeiten"-Fenster öffnen (z. B. um die EXE neu auszuwählen)
    EditGame,
    /// Eine Webseite öffnen
    OpenUrl { label: String, url: String },
    /// Einen Ordner im Explorer öffnen
    OpenFolder { label: String, path: String },
    /// Bibliothek neu einlesen
    Rescan,
    /// Einstellungen öffnen
    OpenSettings,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub severity: Severity,
    pub title: String,
    pub what_happened: String,
    pub why: String,
    pub steps: Vec<String>,
    pub fixes: Vec<Fix>,
    /// Technische Details für Profis / GitHub-Issues
    pub technical: String,
}

fn s(v: &str) -> String {
    v.to_string()
}

fn steps(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| x.to_string()).collect()
}

const VCREDIST_X64: &str = "https://aka.ms/vs/17/release/vc_redist.x64.exe";
const VCREDIST_X86: &str = "https://aka.ms/vs/17/release/vc_redist.x86.exe";
const DIRECTX: &str = "https://www.microsoft.com/de-de/download/details.aspx?id=35";
const STEAM_DL: &str = "https://store.steampowered.com/about/";
const EPIC_DL: &str = "https://store.epicgames.com/de/download";
const WEBVIEW2_DL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";
const STEAM_API_KEY_URL: &str = "https://steamcommunity.com/dev/apikey";

/// Ordner, in dem eine Datei liegt (für "Ordner öffnen").
fn parent_of(path: &str) -> String {
    match path.rfind(['\\', '/']) {
        Some(i) => path[..i].to_string(),
        None => path.to_string(),
    }
}

/// Laufwerksbuchstabe wie "D:" aus einem Pfad.
pub fn drive_of(path: &str) -> Option<String> {
    let b = path.as_bytes();
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        Some(format!("{}:", (b[0] as char).to_ascii_uppercase()))
    } else {
        None
    }
}

impl Problem {
    // ------------------------------------------------------------------
    // Eigene Spiele: Probleme vor dem Start
    // ------------------------------------------------------------------

    pub fn exe_missing(game: &str, path: &str, drive_exists: bool) -> Problem {
        if !drive_exists {
            let drive = drive_of(path).unwrap_or_else(|| s("das Laufwerk"));
            return Problem {
                severity: Severity::Error,
                title: format!("Laufwerk {drive} nicht gefunden"),
                what_happened: format!(
                    "„{game}“ liegt auf dem Laufwerk {drive}. Dieses Laufwerk ist gerade nicht da."
                ),
                why: s("Meistens ist eine externe Festplatte oder ein USB-Stick nicht eingesteckt, oder das Laufwerk hat einen anderen Buchstaben bekommen."),
                steps: steps(&[
                    "Steck die externe Festplatte oder den USB-Stick ein, auf dem das Spiel liegt.",
                    "Warte ein paar Sekunden, bis Windows das Laufwerk erkannt hat (es taucht dann im Explorer unter „Dieser PC“ auf).",
                    "Klick hier im Launcher noch einmal auf „Spielen“.",
                    "Hat das Laufwerk jetzt einen anderen Buchstaben (z. B. E: statt D:)? Dann klick auf „Spiel bearbeiten“ und wähle die EXE-Datei neu aus.",
                ]),
                fixes: vec![Fix::EditGame],
                technical: format!("Pfad: {path}\nLaufwerk {drive} existiert nicht."),
            };
        }
        Problem {
            severity: Severity::Error,
            title: s("Spieldatei nicht gefunden"),
            what_happened: format!("Der Launcher wollte „{game}“ starten, aber die Datei gibt es an diesem Ort nicht mehr."),
            why: s("Das Spiel wurde wahrscheinlich verschoben, umbenannt, gelöscht oder neu installiert. Manchmal entfernt auch ein Antivirus-Programm die Datei."),
            steps: steps(&[
                "Klick unten auf „Spiel bearbeiten“.",
                "Klick neben „Programm (EXE)“ auf „Durchsuchen…“.",
                "Such den Ordner, in dem das Spiel jetzt liegt, und wähle die Start-Datei aus (meistens heißt sie wie das Spiel und endet auf .exe).",
                "Klick auf „Speichern“ und dann wieder auf „Spielen“.",
                "Findest du die Datei gar nicht mehr? Dann installiere das Spiel neu und wiederhole die Schritte.",
                "Hat dein Antivirus die Datei gelöscht? Schau in der Quarantäne deines Antivirus-Programms nach und stelle sie wieder her (nur wenn du dem Spiel vertraust).",
            ]),
            fixes: vec![
                Fix::EditGame,
                Fix::OpenFolder { label: s("Letzten Ordner öffnen"), path: parent_of(path) },
            ],
            technical: format!("Datei existiert nicht: {path}"),
        }
    }

    pub fn workdir_missing(game: &str, dir: &str) -> Problem {
        Problem {
            severity: Severity::Error,
            title: s("Arbeitsordner nicht gefunden"),
            what_happened: format!("Für „{game}“ ist ein Arbeitsordner eingestellt, den es nicht gibt."),
            why: s("Der Ordner wurde verschoben oder gelöscht."),
            steps: steps(&[
                "Klick auf „Spiel bearbeiten“.",
                "Lösche den Text im Feld „Arbeitsordner“ (leer lassen ist völlig okay – dann nimmt der Launcher automatisch den Ordner der EXE).",
                "Klick auf „Speichern“ und starte das Spiel erneut.",
            ]),
            fixes: vec![Fix::EditGame],
            technical: format!("Arbeitsordner existiert nicht: {dir}"),
        }
    }

    // ------------------------------------------------------------------
    // Eigene Spiele: Fehler beim Starten (Windows-Fehlercodes)
    // ------------------------------------------------------------------

    /// Übersetzt einen Windows-Fehler beim Starten eines Programms.
    pub fn from_os_error(game: &str, path: &str, code: Option<i32>, message: &str) -> Problem {
        let tech = format!(
            "Pfad: {path}\nWindows-Fehlercode: {}\nMeldung: {message}",
            code.map(|c| c.to_string()).unwrap_or_else(|| s("unbekannt"))
        );
        match code {
            Some(2) | Some(3) => {
                let mut p = Problem::exe_missing(game, path, true);
                p.technical = tech;
                p
            }
            Some(740) => Problem {
                severity: Severity::Warning,
                title: s("Das Spiel braucht Administratorrechte"),
                what_happened: format!("„{game}“ möchte mit Administratorrechten gestartet werden."),
                why: s("Manche Spiele (oder ihre Anti-Cheat-Programme) brauchen mehr Rechte, damit sie funktionieren. Das ist normal."),
                steps: steps(&[
                    "Klick unten auf „Als Administrator starten“.",
                    "Windows fragt dann „Möchten Sie zulassen, dass…“ – klick auf „Ja“.",
                    "Damit du das nicht jedes Mal machen musst: Klick auf „Spiel bearbeiten“ und schalte „Immer als Administrator starten“ ein.",
                ]),
                fixes: vec![Fix::RunAsAdmin, Fix::EditGame],
                technical: tech,
            },
            Some(5) => Problem {
                severity: Severity::Error,
                title: s("Zugriff verweigert"),
                what_happened: format!("Windows hat den Start von „{game}“ nicht erlaubt."),
                why: s("Entweder fehlen Rechte für den Ordner, die Datei ist durch Windows gesperrt (heruntergeladene Datei), oder ein Antivirus-Programm blockiert sie."),
                steps: steps(&[
                    "Probier zuerst „Als Administrator starten“ (unten). Windows fragt nach – klick auf „Ja“.",
                    "Klappt das nicht: Öffne den Spielordner (Knopf unten), klick mit der rechten Maustaste auf die EXE-Datei → „Eigenschaften“.",
                    "Ganz unten im Reiter „Allgemein“ steht vielleicht „Zulassen“ oder „Blockierung aufheben“. Setz dort den Haken und klick auf „OK“.",
                    "Immer noch nicht? Öffne dein Antivirus-Programm und schau, ob es die Datei blockiert hat. Wenn du dem Spiel vertraust, füge es dort als Ausnahme hinzu.",
                    "Liegt das Spiel in „C:\\Programme“? Dann hilft es oft, das Spiel in einen eigenen Ordner wie „C:\\Spiele“ zu installieren.",
                ]),
                fixes: vec![
                    Fix::RunAsAdmin,
                    Fix::OpenFolder { label: s("Spielordner öffnen"), path: parent_of(path) },
                ],
                technical: tech,
            },
            Some(193) | Some(216) | Some(129) => Problem {
                severity: Severity::Error,
                title: s("Das ist keine Spiel-Datei"),
                what_happened: format!("Die ausgewählte Datei für „{game}“ kann von Windows nicht als Programm gestartet werden."),
                why: s("Wahrscheinlich wurde die falsche Datei ausgewählt, die Datei ist kaputt (z. B. unvollständiger Download), oder das Spiel ist für ein anderes System gemacht (z. B. für Mac oder ein sehr altes 16-Bit-Windows)."),
                steps: steps(&[
                    "Klick auf „Spiel bearbeiten“.",
                    "Klick auf „Durchsuchen…“ und wähle die richtige Start-Datei. Sie endet auf „.exe“ und heißt meistens wie das Spiel – nicht „unins000.exe“, „setup.exe“ oder „UnityCrashHandler.exe“.",
                    "Speichern und erneut starten.",
                    "Ist die Datei kaputt? Dann lade bzw. installiere das Spiel noch einmal neu.",
                ]),
                fixes: vec![
                    Fix::EditGame,
                    Fix::OpenFolder { label: s("Spielordner öffnen"), path: parent_of(path) },
                ],
                technical: tech,
            },
            Some(225) => Problem {
                severity: Severity::Error,
                title: s("Vom Virenschutz blockiert"),
                what_happened: format!("Dein Virenschutz hat „{game}“ blockiert, weil er die Datei für gefährlich hält."),
                why: s("Das kann ein echter Virus sein – oder ein Fehlalarm, was bei Mods, kleinen Indie-Spielen und Cracks oft passiert."),
                steps: steps(&[
                    "Wichtig: Mach nur weiter, wenn du dir sicher bist, dass die Datei aus einer vertrauenswürdigen Quelle stammt!",
                    "Öffne „Windows-Sicherheit“ (Startmenü → „Windows-Sicherheit“ eintippen → öffnen).",
                    "Klick auf „Viren- & Bedrohungsschutz“ → „Schutzverlauf“.",
                    "Such den Eintrag zum Spiel, klick darauf und wähle „Zulassen“ bzw. „Auf Gerät zulassen“.",
                    "Nutzt du ein anderes Antivirus-Programm (z. B. Avast, Norton, Kaspersky)? Dann öffne dieses und füge das Spiel dort unter „Ausnahmen“ hinzu.",
                    "Starte danach das Spiel hier noch einmal.",
                ]),
                fixes: vec![Fix::OpenUrl { label: s("Windows-Sicherheit öffnen"), url: s("windowsdefender://threat") }],
                technical: tech,
            },
            Some(1260) | Some(4551) | Some(4556) => Problem {
                severity: Severity::Error,
                title: s("Von Windows blockiert"),
                what_happened: format!("Windows hat „{game}“ durch eine Sicherheitsregel blockiert (z. B. „Intelligente App-Steuerung“ oder Richtlinien deines Arbeits-/Schul-PCs)."),
                why: s("Windows lässt nur Programme starten, denen es vertraut. Unbekannte Spiele werden dann blockiert."),
                steps: steps(&[
                    "Ist das ein Arbeits- oder Schul-PC? Dann frag bitte deinen IT-Admin – du kannst das selbst meistens nicht ändern.",
                    "Auf deinem eigenen PC: Öffne „Windows-Sicherheit“ → „App- & Browsersteuerung“.",
                    "Schau bei „Intelligente App-Steuerung“, ob sie eingeschaltet ist. Wenn du dem Spiel vertraust, kannst du sie dort ausschalten (Achtung: lässt sich danach nicht einfach wieder einschalten).",
                    "Starte danach das Spiel hier noch einmal.",
                ]),
                fixes: vec![Fix::OpenUrl { label: s("Windows-Sicherheit öffnen"), url: s("windowsdefender://appbrowser") }],
                technical: tech,
            },
            Some(32) | Some(33) => Problem {
                severity: Severity::Warning,
                title: s("Die Datei wird gerade benutzt"),
                what_happened: format!("„{game}“ ist gerade von einem anderen Programm geöffnet und kann nicht gestartet werden."),
                why: s("Vielleicht läuft das Spiel schon im Hintergrund, wird gerade aktualisiert oder ein Antivirus scannt es."),
                steps: steps(&[
                    "Warte kurz (ca. 30 Sekunden) und versuche es erneut.",
                    "Drück Strg + Umschalt + Esc, um den Task-Manager zu öffnen. Such nach dem Spiel, klick es an und dann auf „Task beenden“.",
                    "Hilft das nicht: Starte den PC neu.",
                ]),
                fixes: vec![],
                technical: tech,
            },
            Some(1155) => Problem {
                severity: Severity::Error,
                title: s("Kein Programm für diesen Dateityp"),
                what_happened: format!("Windows weiß nicht, mit welchem Programm es die Datei von „{game}“ öffnen soll."),
                why: s("Die ausgewählte Datei ist keine EXE, sondern z. B. ein Bild, ein Archiv (.zip/.rar) oder eine unbekannte Datei."),
                steps: steps(&[
                    "Ist es eine .zip- oder .rar-Datei? Dann musst du sie zuerst entpacken (Rechtsklick → „Alle extrahieren…“).",
                    "Klick danach hier auf „Spiel bearbeiten“ und wähle die .exe-Datei im entpackten Ordner aus.",
                ]),
                fixes: vec![Fix::EditGame],
                technical: tech,
            },
            _ => Problem::unknown(game, &tech),
        }
    }

    /// Übersetzt den Exit-Code eines Spiels, das direkt nach dem Start abgestürzt ist.
    pub fn from_exit_code(game: &str, path: &str, code: u32) -> Option<Problem> {
        let tech = format!("Pfad: {path}\nExit-Code: {code} (0x{code:08X})");
        let folder = parent_of(path);
        let p = match code {
            0 => return None,
            // STATUS_DLL_NOT_FOUND / Delay-Load-Modul fehlt
            0xC000_0135 | 0xC06D_007E => Problem {
                severity: Severity::Error,
                title: s("Eine wichtige Windows-Datei (DLL) fehlt"),
                what_happened: format!("„{game}“ ist sofort wieder zugegangen, weil eine benötigte Zusatz-Datei fehlt."),
                why: s("Fast alle Spiele brauchen kostenlose Microsoft-Pakete („Visual C++“ und „DirectX“). Die sind auf deinem PC nicht (vollständig) installiert."),
                steps: steps(&[
                    "Klick unten auf „Visual C++ (64-Bit) herunterladen“. Öffne die heruntergeladene Datei, setz den Haken bei „Ich stimme zu“ und klick auf „Installieren“.",
                    "Mach dasselbe mit „Visual C++ (32-Bit) herunterladen“ – ja, beide! Viele Spiele brauchen beide Versionen.",
                    "Klick auf „DirectX herunterladen“, lade es herunter und installiere es (Haken bei „Bing-Leiste“ ruhig wegmachen).",
                    "Starte den PC neu.",
                    "Starte das Spiel hier erneut.",
                    "Immer noch kaputt? Schau im Spielordner nach einem Unterordner wie „_CommonRedist“, „Redist“ oder „DirectX“ und installiere alles, was darin liegt.",
                ]),
                fixes: vec![
                    Fix::OpenUrl { label: s("Visual C++ (64-Bit) herunterladen"), url: s(VCREDIST_X64) },
                    Fix::OpenUrl { label: s("Visual C++ (32-Bit) herunterladen"), url: s(VCREDIST_X86) },
                    Fix::OpenUrl { label: s("DirectX herunterladen"), url: s(DIRECTX) },
                    Fix::OpenFolder { label: s("Spielordner öffnen"), path: folder },
                ],
                technical: tech,
            },
            // STATUS_INVALID_IMAGE_FORMAT (32/64-Bit gemischt)
            0xC000_007B => Problem {
                severity: Severity::Error,
                title: s("Fehler 0xc000007b – Systemdateien passen nicht zusammen"),
                what_happened: format!("„{game}“ konnte nicht richtig starten (der berühmte Fehler 0xc000007b)."),
                why: s("Dem Spiel fehlen passende Microsoft-Zusatzpakete, oder 32-Bit- und 64-Bit-Dateien sind durcheinandergeraten."),
                steps: steps(&[
                    "Installiere „Visual C++ (64-Bit)“ UND „Visual C++ (32-Bit)“ mit den Knöpfen unten. Wenn gefragt wird, wähle „Reparieren“.",
                    "Installiere „DirectX“ mit dem Knopf unten.",
                    "Starte den PC neu.",
                    "Starte das Spiel erneut. Wenn es immer noch nicht geht: Installiere das Spiel neu.",
                ]),
                fixes: vec![
                    Fix::OpenUrl { label: s("Visual C++ (64-Bit) herunterladen"), url: s(VCREDIST_X64) },
                    Fix::OpenUrl { label: s("Visual C++ (32-Bit) herunterladen"), url: s(VCREDIST_X86) },
                    Fix::OpenUrl { label: s("DirectX herunterladen"), url: s(DIRECTX) },
                ],
                technical: tech,
            },
            // STATUS_DLL_INIT_FAILED
            0xC000_0142 => Problem {
                severity: Severity::Error,
                title: s("Das Spiel konnte sich nicht vorbereiten"),
                what_happened: format!("„{game}“ ist beim Hochfahren hängen geblieben und hat sich beendet (Fehler 0xc0000142)."),
                why: s("Oft liegt es an Programmen im Hintergrund (Overlays, Aufnahme-Tools, Antivirus) oder an einem alten Grafiktreiber."),
                steps: steps(&[
                    "Starte den PC neu – das behebt diesen Fehler sehr oft schon.",
                    "Schließe Overlays und Aufnahme-Programme (z. B. Discord-Overlay, MSI Afterburner, OBS) und versuche es erneut.",
                    "Aktualisiere deinen Grafiktreiber (Knöpfe unten: NVIDIA, AMD oder Intel – je nachdem, welche Grafikkarte du hast).",
                    "Installiere die Visual-C++-Pakete (64- und 32-Bit).",
                ]),
                fixes: vec![
                    Fix::OpenUrl { label: s("NVIDIA-Treiber"), url: s("https://www.nvidia.com/de-de/drivers/") },
                    Fix::OpenUrl { label: s("AMD-Treiber"), url: s("https://www.amd.com/de/support/download/drivers.html") },
                    Fix::OpenUrl { label: s("Intel-Treiber"), url: s("https://www.intel.de/content/www/de/de/support/detect.html") },
                    Fix::OpenUrl { label: s("Visual C++ (64-Bit)"), url: s(VCREDIST_X64) },
                ],
                technical: tech,
            },
            // STATUS_ACCESS_VIOLATION, STACK_BUFFER_OVERRUN, etc.
            0xC000_0005 | 0xC000_0409 | 0xC000_001D | 0xC000_0374 | 0xC000_00FD => Problem {
                severity: Severity::Error,
                title: s("Das Spiel ist direkt abgestürzt"),
                what_happened: format!("„{game}“ hat sich direkt nach dem Start mit einem Absturz beendet."),
                why: s("Häufige Gründe: alter Grafiktreiber, kaputte Spieldateien, fehlende Rechte oder ein Programm im Hintergrund, das stört (z. B. Overlay oder Antivirus)."),
                steps: steps(&[
                    "Starte den PC neu und versuche es noch einmal.",
                    "Aktualisiere deinen Grafiktreiber (Knöpfe unten).",
                    "Probier „Als Administrator starten“.",
                    "Schließe Overlays und Tools im Hintergrund (Discord-Overlay, MSI Afterburner, RivaTuner, OBS).",
                    "Hilft nichts: Installiere das Spiel neu. Bei Mods: Entferne die Mods und teste ohne.",
                ]),
                fixes: vec![
                    Fix::RunAsAdmin,
                    Fix::OpenUrl { label: s("NVIDIA-Treiber"), url: s("https://www.nvidia.com/de-de/drivers/") },
                    Fix::OpenUrl { label: s("AMD-Treiber"), url: s("https://www.amd.com/de/support/download/drivers.html") },
                    Fix::OpenUrl { label: s("Intel-Treiber"), url: s("https://www.intel.de/content/www/de/de/support/detect.html") },
                ],
                technical: tech,
            },
            _ => Problem {
                severity: Severity::Warning,
                title: s("Das Spiel hat sich sofort wieder beendet"),
                what_happened: format!("„{game}“ wurde gestartet, hat sich aber nach wenigen Sekunden mit einem Fehlercode wieder geschlossen."),
                why: s("Das Spiel selbst hat ein Problem gemeldet. Oft fehlen Dateien, Rechte oder es braucht einen anderen Launcher (z. B. Ubisoft Connect, EA App, Rockstar Launcher), der zuerst laufen muss."),
                steps: steps(&[
                    "Braucht das Spiel einen eigenen Launcher (Ubisoft, EA, Rockstar, Battle.net)? Dann starte diesen zuerst und melde dich an.",
                    "Probier „Als Administrator starten“.",
                    "Prüf, ob der Arbeitsordner stimmt: Klick auf „Spiel bearbeiten“ und lass das Feld „Arbeitsordner“ leer.",
                    "Starte den PC neu und versuche es erneut.",
                    "Installiere die Visual-C++-Pakete (Knopf unten) – das hilft sehr oft.",
                ]),
                fixes: vec![
                    Fix::RunAsAdmin,
                    Fix::EditGame,
                    Fix::OpenUrl { label: s("Visual C++ (64-Bit) herunterladen"), url: s(VCREDIST_X64) },
                ],
                technical: tech,
            },
        };
        Some(p)
    }

    // ------------------------------------------------------------------
    // Steam / Epic
    // ------------------------------------------------------------------

    pub fn steam_missing() -> Problem {
        Problem {
            severity: Severity::Error,
            title: s("Steam ist nicht installiert"),
            what_happened: s("Dieses Spiel gehört zu Steam, aber Steam wurde auf deinem PC nicht gefunden."),
            why: s("Steam-Spiele können nur über Steam gestartet werden (wegen Kopierschutz und Updates)."),
            steps: steps(&[
                "Klick unten auf „Steam herunterladen“.",
                "Klick auf der Webseite auf „Steam installieren“ und öffne die heruntergeladene Datei.",
                "Folge der Installation und melde dich mit deinem Steam-Konto an.",
                "Komm zurück in den MAZ Launcher und klick auf „Bibliothek neu laden“ (Pfeil-Symbol oben).",
            ]),
            fixes: vec![
                Fix::OpenUrl { label: s("Steam herunterladen"), url: s(STEAM_DL) },
                Fix::Rescan,
            ],
            technical: s("Kein Handler für steam:// registriert (HKEY_CLASSES_ROOT\\steam fehlt)."),
        }
    }

    pub fn epic_missing() -> Problem {
        Problem {
            severity: Severity::Error,
            title: s("Epic Games Launcher ist nicht installiert"),
            what_happened: s("Dieses Spiel gehört zu Epic Games, aber der Epic Games Launcher wurde nicht gefunden."),
            why: s("Epic-Spiele werden über den Epic Games Launcher gestartet (wegen Anmeldung und Updates)."),
            steps: steps(&[
                "Klick unten auf „Epic Games Launcher herunterladen“.",
                "Öffne die heruntergeladene Datei und installiere den Launcher.",
                "Melde dich mit deinem Epic-Konto an.",
                "Komm zurück und klick auf „Bibliothek neu laden“ (Pfeil-Symbol oben).",
            ]),
            fixes: vec![
                Fix::OpenUrl { label: s("Epic Games Launcher herunterladen"), url: s(EPIC_DL) },
                Fix::Rescan,
            ],
            technical: s("Kein Handler für com.epicgames.launcher:// registriert."),
        }
    }

    /// Fehler von ShellExecute (Codes <= 32) beim Öffnen von steam:// / com.epicgames.launcher://
    pub fn launcher_uri_failed(game: &str, store: &str, code: i32, uri: &str) -> Problem {
        Problem {
            severity: Severity::Error,
            title: format!("{store} konnte das Spiel nicht starten"),
            what_happened: format!("Der Launcher hat {store} gebeten, „{game}“ zu starten – aber {store} hat nicht reagiert."),
            why: format!("{store} ist vielleicht abgestürzt, hängt beim Anmelden oder ist nicht richtig installiert."),
            steps: vec![
                format!("Öffne {store} einmal von Hand und melde dich an, falls nötig."),
                format!("Prüfe in {store}, ob das Spiel dort als „installiert“ angezeigt wird und ob ein Update läuft."),
                "Klick danach hier erneut auf „Spielen“.".to_string(),
                format!("Hilft das nicht: Beende {store} komplett (Task-Manager mit Strg + Umschalt + Esc → {store} → „Task beenden“) und starte es neu."),
                format!("Immer noch nicht? Installiere {store} neu – deine Spiele bleiben dabei erhalten."),
            ],
            fixes: vec![Fix::Rescan],
            technical: format!("ShellExecute-Code: {code}\nURI: {uri}"),
        }
    }

    pub fn shortcut_failed(path: &str, message: &str) -> Problem {
        Problem {
            severity: Severity::Warning,
            title: s("Verknüpfung konnte nicht erstellt werden"),
            what_happened: s("Die Installation hat geklappt, aber die Verknüpfung konnte nicht angelegt werden."),
            why: s("Vielleicht ist dein Desktop-Ordner auf OneDrive umgeleitet oder durch einen Virenschutz geschützt."),
            steps: steps(&[
                "Öffne den App-Ordner (Knopf unten).",
                "Klick mit der rechten Maustaste auf „MAZ-Launcher.exe“ → „Weitere Optionen anzeigen“ → „Senden an“ → „Desktop (Verknüpfung erstellen)“.",
            ]),
            fixes: vec![Fix::OpenFolder { label: s("App-Ordner öffnen"), path: parent_of(path) }],
            technical: format!("Ziel: {path}\nFehler: {message}"),
        }
    }

    pub fn install_failed(dir: &str, message: &str) -> Problem {
        Problem {
            severity: Severity::Error,
            title: s("Installation fehlgeschlagen"),
            what_happened: s("Der MAZ Launcher konnte nicht in den gewählten Ordner kopiert werden."),
            why: s("Entweder fehlen Rechte für den Ordner (z. B. „C:\\Programme“), der Launcher läuft dort schon, oder die Festplatte ist voll."),
            steps: steps(&[
                "Schließe alle offenen MAZ-Launcher-Fenster (auch im Task-Manager) und versuche es erneut.",
                "Wähle einen anderen Ordner – am einfachsten den vorgeschlagenen Standardordner (der braucht keine Admin-Rechte).",
                "Prüfe, ob auf der Festplatte noch Platz frei ist.",
            ]),
            fixes: vec![],
            technical: format!("Zielordner: {dir}\nFehler: {message}"),
        }
    }

    pub fn steam_api(status: Option<u16>, message: &str) -> Problem {
        match status {
            Some(401) | Some(403) => Problem {
                severity: Severity::Warning,
                title: s("Steam-API-Schlüssel ist falsch"),
                what_happened: s("Steam hat den Zugriff auf deine Spieleliste abgelehnt."),
                why: s("Der eingetragene API-Schlüssel ist falsch, abgelaufen oder gehört nicht zu diesem Konto."),
                steps: steps(&[
                    "Klick unten auf „API-Schlüssel holen“ und melde dich bei Steam an.",
                    "Gib bei „Domainname“ einfach „localhost“ ein, setz den Haken und klick auf „Registrieren“.",
                    "Kopiere den langen Schlüssel (32 Zeichen) und füge ihn in den Einstellungen bei „Steam-API-Schlüssel“ ein.",
                    "Klick auf „Bibliothek neu laden“.",
                ]),
                fixes: vec![
                    Fix::OpenUrl { label: s("API-Schlüssel holen"), url: s(STEAM_API_KEY_URL) },
                    Fix::OpenSettings,
                ],
                technical: format!("HTTP {}: {message}", status.unwrap_or(0)),
            },
            _ => Problem {
                severity: Severity::Info,
                title: s("Nicht installierte Steam-Spiele konnten nicht geladen werden"),
                what_happened: s("Deine installierten Spiele werden ganz normal angezeigt. Nur die Liste der nicht installierten Spiele konnte nicht von Steam geladen werden."),
                why: s("Wahrscheinlich ist gerade keine Internetverbindung da, Steam hat Störungen, oder die Steam-ID ist falsch."),
                steps: steps(&[
                    "Prüfe deine Internetverbindung.",
                    "Prüfe in den Einstellungen, ob die Steam-ID stimmt (eine 17-stellige Zahl, die mit 7656 beginnt).",
                    "Versuche es in ein paar Minuten noch einmal mit „Bibliothek neu laden“.",
                ]),
                fixes: vec![Fix::OpenSettings, Fix::Rescan],
                technical: format!("HTTP {:?}: {message}", status),
            },
        }
    }

    pub fn webview2_missing() -> Problem {
        Problem {
            severity: Severity::Error,
            title: s("Microsoft WebView2 fehlt"),
            what_happened: s("Der MAZ Launcher braucht ein kleines, kostenloses Microsoft-Programm namens „WebView2“, um sein Fenster anzuzeigen. Es ist auf diesem PC nicht installiert."),
            why: s("Auf Windows 10 und 11 ist WebView2 normalerweise schon dabei. Auf älteren oder stark „aufgeräumten“ Windows-Installationen fehlt es manchmal."),
            steps: steps(&[
                "Klick auf „OK“ – dann öffnet sich die Microsoft-Downloadseite.",
                "Die Datei „MicrosoftEdgeWebview2Setup.exe“ wird heruntergeladen. Öffne sie.",
                "Warte, bis die Installation fertig ist (dauert ca. 1 Minute).",
                "Starte den MAZ Launcher noch einmal.",
            ]),
            fixes: vec![Fix::OpenUrl { label: s("WebView2 herunterladen"), url: s(WEBVIEW2_DL) }],
            technical: s("WebView2 Runtime nicht in der Registry gefunden (EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5})."),
        }
    }

    pub fn uac_cancelled(game: &str) -> Problem {
        Problem {
            severity: Severity::Info,
            title: s("Admin-Abfrage abgebrochen"),
            what_happened: format!("„{game}“ sollte mit Administratorrechten starten, aber die Windows-Abfrage wurde mit „Nein“ beantwortet oder geschlossen."),
            why: s("Ohne deine Erlaubnis darf Windows das Spiel nicht als Administrator starten."),
            steps: steps(&[
                "Klick noch einmal auf „Spielen“.",
                "Wenn Windows fragt „Möchten Sie zulassen, dass durch diese App Änderungen an Ihrem Gerät vorgenommen werden?“, klick auf „Ja“.",
                "Willst du das Spiel lieber ohne Admin-Rechte starten? Klick auf „Spiel bearbeiten“ und schalte „Immer als Administrator starten“ aus.",
            ]),
            fixes: vec![Fix::RunAsAdmin, Fix::EditGame],
            technical: s("ShellExecute(runas) → SE_ERR_ACCESSDENIED / ERROR_CANCELLED"),
        }
    }

    /// ShellExecute-Fehlercodes (≤ 32) in Windows-Fehlercodes übersetzen.
    pub fn from_shell_code(game: &str, path: &str, code: i32, as_admin: bool) -> Problem {
        let os = match code {
            2 => 2,
            3 => 3,
            5 if as_admin => return Problem::uac_cancelled(game),
            5 => 5,
            11 => 193,
            26 => 32,
            27 | 31 => 1155,
            32 => 126,
            other => other,
        };
        if os == 126 {
            if let Some(p) = Problem::from_exit_code(game, path, 0xC000_0135) {
                return p;
            }
        }
        Problem::from_os_error(game, path, Some(os), &format!("ShellExecute-Code {code}"))
    }

    pub fn unknown(game: &str, technical: &str) -> Problem {
        Problem {
            severity: Severity::Error,
            title: s("Unbekannter Fehler beim Starten"),
            what_happened: format!("„{game}“ konnte aus einem unbekannten Grund nicht gestartet werden."),
            why: s("Windows hat einen Fehler gemeldet, den der Launcher nicht genau zuordnen kann."),
            steps: steps(&[
                "Starte den PC neu und versuche es noch einmal.",
                "Probier „Als Administrator starten“.",
                "Klick auf „Spiel bearbeiten“ und prüfe, ob die EXE-Datei stimmt.",
                "Versuch, das Spiel direkt per Doppelklick im Spielordner zu starten. Kommt dort eine Fehlermeldung, hilft diese oft weiter.",
                "Kopiere die technischen Details (unten) und such im Internet danach oder melde den Fehler auf GitHub.",
            ]),
            fixes: vec![Fix::RunAsAdmin, Fix::EditGame],
            technical: technical.to_string(),
        }
    }

    /// Kurzer Text für die Zwischenablage / GitHub-Issues.
    pub fn to_clipboard_text(&self) -> String {
        let mut out = format!(
            "MAZ Launcher – {}\n\nWas ist passiert?\n{}\n\nWarum?\n{}\n\nSo behebst du es:\n",
            self.title, self.what_happened, self.why
        );
        for (i, st) in self.steps.iter().enumerate() {
            out.push_str(&format!("{}. {}\n", i + 1, st));
        }
        out.push_str(&format!("\nTechnische Details:\n{}\n", self.technical));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_detection() {
        assert_eq!(drive_of("d:\\Games\\x.exe"), Some("D:".into()));
        assert_eq!(drive_of("\\\\server\\share"), None);
    }

    #[test]
    fn missing_drive_explained() {
        let p = Problem::exe_missing("Test", "E:\\Spiele\\test.exe", false);
        assert!(p.title.contains("E:"));
    }

    #[test]
    fn os_errors_mapped() {
        assert_eq!(Problem::from_os_error("x", "C:\\a.exe", Some(740), "").fixes[0], Fix::RunAsAdmin);
        assert!(Problem::from_os_error("x", "C:\\a.exe", Some(193), "").title.contains("keine"));
        assert!(Problem::from_os_error("x", "C:\\a.exe", Some(99999), "").title.contains("Unbekannt"));
    }

    #[test]
    fn exit_codes_mapped() {
        assert!(Problem::from_exit_code("x", "C:\\a.exe", 0).is_none());
        let p = Problem::from_exit_code("x", "C:\\a.exe", 0xC000_0135).unwrap();
        assert!(p.title.contains("DLL"));
        let p = Problem::from_exit_code("x", "C:\\a.exe", 0xC000_007B).unwrap();
        assert!(p.title.contains("0xc000007b"));
        assert!(!p.to_clipboard_text().is_empty());
        assert_eq!(parent_of("C:\\a\\b.exe"), "C:\\a");
    }
}
