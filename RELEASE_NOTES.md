## MAZ Launcher 1.3.2 🔒

**Download:** `MAZ-Launcher.exe` unten herunterladen → Doppelklick → „Installieren“ → „Jetzt öffnen“.
(`MAZ-Launcher-mini.exe` ist dieselbe App, nur noch kleiner gepackt. Falls dein Virenscanner meckert, nimm die normale Version.)

### Sicherheit in 1.3.2 (zweite, unabhängige Prüfung)
- Die EXE lädt Windows-Programme und DLLs nur noch aus dem Windows-Systemordner – eine untergeschobene Datei im Downloads-Ordner kann nichts mehr ausrichten. Visual C++ ist fest eingebaut.
- Anmeldung wird nie an einen anderen Server geschickt (beim Wechsel des Servers wirst du abgemeldet).
- Eigene Spiele deinstallieren: Der Launcher zeigt vorher genau, welches Programm gestartet wird, und nimmt nur eindeutige Treffer.
- Server: Spam-Schutz kann nicht mehr umgangen werden, Suche listet nicht mehr alle Nutzer, Freundeslisten anderer sind nicht abfragbar, unsichtbare Sonderzeichen in Namen werden entfernt.
- Bilder nur noch über sichere Verbindungen, keine Netzwerkpfade.
- Steam-API-Schlüssel wird verschlüsselt gespeichert.
- Build-Sicherheit: automatische Prüfung aller Bibliotheken (cargo audit), feste Versionen, minimale Rechte, und zu jedem Release eine SHA256SUMS.txt zum Überprüfen der Downloads.

### Sicherheit in 1.3.1
- **Wichtig:** „MAZ Launcher deinstallieren“ löscht jetzt nur noch die eigenen Dateien – nie mehr den ganzen Ordner (vorher hätte das z. B. den Downloads-Ordner mitlöschen können, wenn die EXE dort lag).
- Links aus dem Internet können keine Programme oder Dateien mehr starten – nur noch echte Web- und Store-Links.
- Anmeldedaten (Konto & Epic) werden jetzt mit Windows verschlüsselt gespeichert.
- Das Fenster darf nur noch Bilder von deinem PC laden, keine anderen Dateien.
- Profilbilder werden nur noch von Google und GitHub geladen.
- Schutz vor Spam bei Nachrichten und Freundschaftsanfragen (Server-Update, siehe unten).

### Neu in 1.3.0
- **Deinstallieren:** Jedes installierte Spiel hat jetzt einen Knopf „Deinstallieren“ – Steam fragt direkt nach, bei Epic führt eine Anleitung durch die 3 Klicks, bei eigenen Spielen wird das Deinstallationsprogramm automatisch gefunden.
- **Alle Epic-Spiele:** Unter Einstellungen → Verbindungen → „Epic-Konto verbinden“ erscheint deine komplette Epic-Bibliothek, auch nie installierte Spiele.
- **Fehlerhilfe für alle Spiele:** Stürzt ein Steam- oder Epic-Spiel direkt nach dem Start ab, erklärt der Launcher automatisch, was los ist – mit Knöpfen wie „Spieldateien überprüfen“, „Visual C++ herunterladen“ oder „Grafiktreiber“.
- **„Startet nicht? Hilfe“** bei jedem Spiel mit allen wichtigen Lösungen.
- Fehler-Knöpfe sind jetzt in allen Sprachen übersetzt.
- Neue Screenshots im README.

Windows zeigt „Der Computer wurde durch Windows geschützt“? → „Weitere Informationen“ → „Trotzdem ausführen“.
