# Datenschutzerklärung – MAZ Launcher

Stand: Oktober 2026 (ab Version 1.4.0)

Der MAZ Launcher ist ein kostenloses Open-Source-Programm für Windows, macOS und Linux von MikeAdamRGB
(Kontakt: m.zweers10@gmail.com, GitHub: https://github.com/MikeAdamRGB/MAZ-Lauchner).

## Kurz gesagt
**Der MAZ Launcher läuft komplett auf deinem Computer.** Es gibt kein MAZ-Konto, keinen MAZ-Server,
keine Werbung und kein Tracking. Wir (der Entwickler) bekommen keine Daten von dir.

## Was auf deinem Computer gespeichert wird
Spieleliste-Einstellungen, eigene Spiele, Roblox-Favoriten, Designs, Favoriten und „zuletzt gespielt“:
- Windows: `%APPDATA%\MAZ Launcher` (Einstellungen) und `%LOCALAPPDATA%\MAZ Launcher` (Bilder-Cache)
- macOS: `~/Library/Application Support/MAZ Launcher` und `~/Library/Caches/MAZ Launcher`
- Linux: `~/.config/maz-launcher` und `~/.cache/maz-launcher`

Anmeldedaten für die freiwillige Epic-Verbindung werden unter Windows mit Windows (DPAPI) verschlüsselt gespeichert.

## Wann der Launcher ins Internet geht
Nur für diese Dinge – und immer direkt zum jeweiligen Anbieter, nie über einen MAZ-Server:

| Wofür | Wohin | Was wird gesendet |
|---|---|---|
| Cover-Bilder | Steam-Store / Steam-CDN, Epic-CDN, Roblox-Bilder | nur die Bild-Adresse |
| Gratis-Spiele-Liste | Epic Games Store, Steam-Store, GamerPower | nichts Persönliches |
| Roblox-Spiel hinzufügen | öffentliche Roblox-Schnittstellen (Name, Bild) | nur die Spiel-Nummer |
| Update-Prüfung (abschaltbar) | GitHub | nichts Persönliches |
| Optional: Steam-Web-API-Schlüssel | Steam | dein Schlüssel und deine SteamID |
| Optional: Epic-Konto verbinden | Epic Games | Anmeldung erfolgt bei Epic im Browser |

Wie bei jeder Internetverbindung sehen diese Anbieter deine IP-Adresse. Es gelten deren Datenschutzbestimmungen.

## Passwörter
Der MAZ Launcher fragt **nie** nach Passwörtern von Steam, Epic, Roblox oder anderen Diensten.

## Frühere Versionen (1.2.0 – 1.3.2)
Diese Versionen hatten ein freiwilliges Konto mit Freunden und Chat (Server bei Supabase, Frankfurt).
Diese Funktion wurde mit Version 1.4.0 komplett entfernt. Wer ein Konto angelegt hatte und die Löschung
der dort gespeicherten Daten wünscht, schreibt bitte an m.zweers10@gmail.com.

## Deine Rechte
Da wir keine Daten von dir speichern, gibt es bei uns nichts abzufragen oder zu löschen. Deine lokalen Daten
löschst du, indem du die oben genannten Ordner löschst (oder beim Deinstallieren „Einstellungen löschen“ wählst).
